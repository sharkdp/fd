// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use exec;
use fshelper;
use internal::{error, FdOptions};
use output;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::thread;
use std::time;

use ignore::{self, WalkBuilder};
use ignore::overrides::OverrideBuilder;
use regex::Regex;

/// The receiver thread can either be buffering results or directly streaming to the console.
enum ReceiverMode {
    /// Receiver is still buffering in order to sort the results, if the search finishes fast
    /// enough.
    Buffering,

    /// Receiver is directly printing results to the output.
    Streaming,
}

/// The type of file to search for.
#[derive(Copy, Clone)]
pub enum FileType {
    Any,
    RegularFile,
    Directory,
    SymLink,
}

/// Recursively scan the given search path for files / pathnames matching the pattern.
///
/// If the `--exec` argument was supplied, this will create a thread pool for executing
/// jobs in parallel from a given command line and the discovered paths. Otherwise, each
/// path will simply be written to standard output.
pub fn scan(
    path_vec: &mut Vec<PathBuf>,
    pattern: Arc<Regex>,
    config: Arc<FdOptions>,
    wants_to_quit: &Arc<AtomicBool>,
) {
    let first_path_buf = path_vec.pop().unwrap();
    let (tx, rx) = channel();
    let threads = config.threads;

    let mut override_builder = OverrideBuilder::new(first_path_buf.as_path());

    for pattern in &config.exclude_patterns {
        let res = override_builder.add(pattern);
        if res.is_err() {
            error(&format!("Error: malformed exclude pattern '{}'", pattern));
        }
    }
    let overrides = override_builder.build().unwrap_or_else(|_| {
        error("Mismatch in exclude patterns");
    });

    let mut walker = WalkBuilder::new(first_path_buf.as_path());
    walker
        .hidden(config.ignore_hidden)
        .ignore(config.read_ignore)
        .git_ignore(config.read_gitignore)
        .parents(config.read_ignore || config.read_gitignore)
        .git_global(config.read_gitignore)
        .git_exclude(config.read_gitignore)
        .overrides(overrides)
        .follow_links(config.follow_links)
        .max_depth(config.max_depth);
    while path_vec.len() > 0 {
        walker.add(path_vec.pop().unwrap().as_path());
    }
    let parallel_walker = walker
        .threads(threads)
        .build_parallel();

    // Spawn the thread that receives all results through the channel.
    let rx_config = Arc::clone(&config);
    let quit_clone = Arc::clone(wants_to_quit);
    let receiver_thread = thread::spawn(move || {
        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = rx_config.command {
            let shared_rx = Arc::new(Mutex::new(rx));

            let out_perm = Arc::new(Mutex::new(()));

            // TODO: the following line is a workaround to replace the `unsafe` block that was
            // previously used here to avoid the (unnecessary?) cloning of the command. The
            // `unsafe` block caused problems on some platforms (SIGILL instructions on Linux) and
            // therefore had to be removed.
            let cmd = Arc::new(cmd.clone());

            // Each spawned job will store it's thread handle in here.
            let mut handles = Vec::with_capacity(threads);
            for _ in 0..threads {
                let rx = Arc::clone(&shared_rx);
                let cmd = Arc::clone(&cmd);
                let out_perm = Arc::clone(&out_perm);

                // Spawn a job thread that will listen for and execute inputs.
                let handle = thread::spawn(move || exec::job(rx, cmd, out_perm));

                // Push the handle of the spawned thread into the vector for later joining.
                handles.push(handle);
            }

            // Wait for all threads to exit before exiting the program.
            for h in handles {
                h.join().unwrap();
            }
        } else {
            let start = time::Instant::now();

            let mut buffer = vec![];

            // Start in buffering mode
            let mut mode = ReceiverMode::Buffering;

            // Maximum time to wait before we start streaming to the console.
            let max_buffer_time = rx_config.max_buffer_time.unwrap_or_else(
                || time::Duration::from_millis(100),
            );

            for value in rx {
                match mode {
                    ReceiverMode::Buffering => {
                        buffer.push(value);

                        // Have we reached the maximum time?
                        if time::Instant::now() - start > max_buffer_time {
                            // Flush the buffer
                            for v in &buffer {
                                output::print_entry(&v, &rx_config, &quit_clone);
                            }
                            buffer.clear();

                            // Start streaming
                            mode = ReceiverMode::Streaming;
                        }
                    }
                    ReceiverMode::Streaming => {
                        output::print_entry(&value, &rx_config, &quit_clone);
                    }
                }
            }

            // If we have finished fast enough (faster than max_buffer_time), we haven't streamed
            // anything to the console, yet. In this case, sort the results and print them:
            if !buffer.is_empty() {
                buffer.sort();
                for value in buffer {
                    output::print_entry(&value, &rx_config, &quit_clone);
                }
            }
        }
    });

    // Spawn the sender threads.
    parallel_walker.run(|| {
        let config = Arc::clone(&config);
        let pattern = Arc::clone(&pattern);
        let tx_thread = tx.clone();

        Box::new(move |entry_o| {
            let entry = match entry_o {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };

            let entry_path = entry.path();

            if entry.depth() == 0 {
                return ignore::WalkState::Continue;
            }

            // Filter out unwanted file types.
            match config.file_type {
                FileType::Any => (),
                FileType::RegularFile => {
                    if entry.file_type().map_or(true, |ft| !ft.is_file()) {
                        return ignore::WalkState::Continue;
                    }
                }
                FileType::Directory => {
                    if entry.file_type().map_or(true, |ft| !ft.is_dir()) {
                        return ignore::WalkState::Continue;
                    }
                }
                FileType::SymLink => {
                    if entry.file_type().map_or(true, |ft| !ft.is_symlink()) {
                        return ignore::WalkState::Continue;
                    }
                }
            }

            // Filter out unwanted extensions.
            if let Some(ref filter_ext) = config.extension {
                let entry_ext = entry_path.extension().map(
                    |e| e.to_string_lossy().to_lowercase(),
                );
                if entry_ext.map_or(true, |ext| ext != *filter_ext) {
                    return ignore::WalkState::Continue;
                }
            }

            let search_str_o = if config.search_full_path {
                match fshelper::path_absolute_form(entry_path) {
                    Ok(path_abs_buf) => Some(path_abs_buf.to_string_lossy().into_owned().into()),
                    Err(_) => error("Error: unable to get full path."),
                }
            } else {
                entry_path.file_name().map(|f| f.to_string_lossy())
            };

            if let Some(search_str) = search_str_o {
                if pattern.is_match(&*search_str) {
                    // TODO: take care of the unwrap call
                    tx_thread.send(entry_path.to_owned()).unwrap()
                }
            }

            ignore::WalkState::Continue
        })
    });

    // Drop the initial sender. If we don't do this, the receiver will block even
    // if all threads have finished, since there is still one sender around.
    drop(tx);

    // Wait for the receiver thread to print out all results.
    receiver_thread.join().unwrap();
}
