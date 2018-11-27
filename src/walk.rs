// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate ctrlc;

use exec;
use exit_codes::ExitCode;
use fshelper;
use internal::{opts::FdOptions, MAX_BUFFER_LENGTH};
use output;

use std::error::Error;
use std::fs::{self, FileType};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use ignore::overrides::OverrideBuilder;
use ignore::{self, WalkBuilder};
use regex::Regex;

/// The receiver thread can either be buffering results or directly streaming to the console.
enum ReceiverMode {
    /// Receiver is still buffering in order to sort the results, if the search finishes fast
    /// enough.
    Buffering,

    /// Receiver is directly printing results to the output.
    Streaming,
}

/// The Worker threads can result in a valid entry having PathBuf or an error.
pub enum WorkerResult {
    Entry(PathBuf),
    Error(ignore::Error),
}

/// A representation of a directory entry, used by the Worker.
pub struct DirEntry {
    pub path: PathBuf,
    pub file_type: Option<FileType>,
}

/// Recursively scan the given search path for files / pathnames matching the pattern.
///
/// If the `--exec` argument was supplied, this will create a thread pool for executing
/// jobs in parallel from a given command line and the discovered paths. Otherwise, each
/// path will simply be written to standard output.
pub fn scan(path_vec: &[PathBuf], pattern: Arc<Regex>, config: Arc<FdOptions>) {
    let mut path_iter = path_vec.iter();
    let first_path_buf = path_iter
        .next()
        .expect("Error: Path vector can not be empty");
    let (tx, rx) = channel();
    let threads = config.threads;
    let show_filesystem_errors = config.show_filesystem_errors;

    let mut override_builder = OverrideBuilder::new(first_path_buf.as_path());

    for pattern in &config.exclude_patterns {
        let res = override_builder.add(pattern);
        if res.is_err() {
            print_error_and_exit!("Malformed exclude pattern '{}'", pattern);
        }
    }
    let overrides = override_builder.build().unwrap_or_else(|_| {
        print_error_and_exit!("Mismatch in exclude patterns");
    });

    let mut walker = WalkBuilder::new(first_path_buf.as_path());
    walker
        .hidden(config.ignore_hidden)
        .ignore(config.read_fdignore)
        .parents(config.read_fdignore || config.read_vcsignore)
        .git_ignore(config.read_vcsignore)
        .git_global(config.read_vcsignore)
        .git_exclude(config.read_vcsignore)
        .overrides(overrides)
        .follow_links(config.follow_links)
        .max_depth(config.max_depth);

    if config.read_fdignore {
        walker.add_custom_ignore_filename(".fdignore");
    }

    for ignore_file in &config.ignore_files {
        let result = walker.add_ignore(ignore_file);
        if let Some(err) = result {
            match err {
                ignore::Error::Partial(_) => (),
                _ => {
                    print_error!(
                        "{}",
                        format!(
                            "Malformed pattern in custom ignore file '{}': {}.",
                            ignore_file.to_string_lossy(),
                            err.description()
                        )
                    );
                }
            }
        }
    }

    for path_entry in path_iter {
        walker.add(path_entry.as_path());
    }

    let parallel_walker = walker.threads(threads).build_parallel();

    let wants_to_quit = Arc::new(AtomicBool::new(false));
    let receiver_wtq = Arc::clone(&wants_to_quit);
    let sender_wtq = Arc::clone(&wants_to_quit);
    if config.ls_colors.is_some() && config.command.is_none() {
        let wq = Arc::clone(&receiver_wtq);
        ctrlc::set_handler(move || {
            wq.store(true, Ordering::Relaxed);
        })
        .unwrap();
    }

    // Spawn the thread that receives all results through the channel.
    let rx_config = Arc::clone(&config);
    let receiver_thread = thread::spawn(move || {
        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = rx_config.command {
            if cmd.in_batch_mode() {
                exec::batch(rx, cmd, show_filesystem_errors);
            } else {
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
                    let handle =
                        thread::spawn(move || exec::job(rx, cmd, out_perm, show_filesystem_errors));

                    // Push the handle of the spawned thread into the vector for later joining.
                    handles.push(handle);
                }

                // Wait for all threads to exit before exiting the program.
                for h in handles {
                    h.join().unwrap();
                }
            }
        } else {
            let start = time::Instant::now();

            let mut buffer = vec![];

            // Start in buffering mode
            let mut mode = ReceiverMode::Buffering;

            // Maximum time to wait before we start streaming to the console.
            let max_buffer_time = rx_config
                .max_buffer_time
                .unwrap_or_else(|| time::Duration::from_millis(100));

            for worker_result in rx {
                match worker_result {
                    WorkerResult::Entry(value) => {
                        match mode {
                            ReceiverMode::Buffering => {
                                buffer.push(value);

                                // Have we reached the maximum buffer size or maximum buffering time?
                                if buffer.len() > MAX_BUFFER_LENGTH
                                    || time::Instant::now() - start > max_buffer_time
                                {
                                    // Flush the buffer
                                    for v in &buffer {
                                        output::print_entry(v, &rx_config, &receiver_wtq);
                                    }
                                    buffer.clear();

                                    // Start streaming
                                    mode = ReceiverMode::Streaming;
                                }
                            }
                            ReceiverMode::Streaming => {
                                output::print_entry(&value, &rx_config, &receiver_wtq);
                            }
                        }
                    }
                    WorkerResult::Error(err) => {
                        if show_filesystem_errors {
                            print_error!("{}", err);
                        }
                    }
                }
            }

            // If we have finished fast enough (faster than max_buffer_time), we haven't streamed
            // anything to the console, yet. In this case, sort the results and print them:
            if !buffer.is_empty() {
                buffer.sort();
                for value in buffer {
                    output::print_entry(&value, &rx_config, &receiver_wtq);
                }
            }
        }
    });

    // Spawn the sender threads.
    parallel_walker.run(|| {
        let config = Arc::clone(&config);
        let pattern = Arc::clone(&pattern);
        let tx_thread = tx.clone();
        let wants_to_quit = Arc::clone(&sender_wtq);

        Box::new(move |entry_o| {
            if wants_to_quit.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }

            let entry = match entry_o {
                Ok(e) => {
                    // Skip the root directory entry.
                    if e.depth() == 0 {
                        return ignore::WalkState::Continue;
                    }

                    // Transform ignore::DirEntry into our own DirEntry.
                    DirEntry {
                        file_type: e.file_type(),
                        path: e.into_path(),
                    }
                }
                Err(err) => {
                    // Keep track of a possible broken symlink.
                    let mut symlink_o = None;
                    if let ignore::Error::WithPath { ref path, ref err } = err {
                        // An I/O error when the path does not exist (should) indicate a dangling symlink.
                        if err.is_io() && !path.exists() {
                            symlink_o = Some(DirEntry {
                                file_type: path.symlink_metadata().map(|s| s.file_type()).ok(),
                                path: path.to_path_buf(),
                            });
                        }
                    };

                    if let Some(symlink) = symlink_o {
                        symlink
                    } else {
                        tx_thread.send(WorkerResult::Error(err)).unwrap();
                        return ignore::WalkState::Continue;
                    }
                }
            };

            // Filter out unwanted file types.
            if let Some(ref file_types) = config.file_types {
                if let Some(ref file_type) = entry.file_type {
                    if (!file_types.files && file_type.is_file())
                        || (!file_types.directories && file_type.is_dir())
                        || (!file_types.symlinks && file_type.is_symlink())
                        || (file_types.executables_only
                            && !fs::metadata(&entry.path)
                                .map(|m| fshelper::is_executable(&m))
                                .unwrap_or(false))
                        || (file_types.empty_only && !fshelper::is_empty(&entry))
                    {
                        return ignore::WalkState::Continue;
                    } else if !(file_type.is_file() || file_type.is_dir() || file_type.is_symlink())
                    {
                        // This is probably a block device, char device, fifo or socket. Skip it.
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            // Filter out unwanted extensions.
            if let Some(ref exts_regex) = config.extensions {
                if let Some(path_str) = entry.path.file_name().map_or(None, |s| s.to_str()) {
                    if !exts_regex.is_match(path_str) {
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            // Filter out unwanted sizes if it is a file and we have been given size constraints.
            if config.size_constraints.len() > 0 {
                if entry.path.is_file() {
                    if let Ok(metadata) = entry.path.metadata() {
                        let file_size = metadata.len();
                        if config
                            .size_constraints
                            .iter()
                            .any(|sc| !sc.is_within(file_size))
                        {
                            return ignore::WalkState::Continue;
                        }
                    } else {
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            // Filter out unwanted modification times
            if !config.time_constraints.is_empty() {
                let mut matched = false;
                if entry.path.is_file() {
                    if let Ok(metadata) = entry.path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            matched = config
                                .time_constraints
                                .iter()
                                .all(|tf| tf.applies_to(&modified));
                        }
                    }
                }
                if !matched {
                    return ignore::WalkState::Continue;
                }
            }

            let search_str_o = if config.search_full_path {
                match fshelper::path_absolute_form(&entry.path) {
                    Ok(path_abs_buf) => Some(path_abs_buf.to_string_lossy().into_owned()),
                    Err(_) => {
                        print_error_and_exit!("Unable to retrieve absolute path.");
                    }
                }
            } else {
                entry
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
            };

            if let Some(search_str) = search_str_o {
                if pattern.is_match(&*search_str) {
                    // TODO: take care of the unwrap call
                    tx_thread.send(WorkerResult::Entry(entry.path)).unwrap()
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

    if wants_to_quit.load(Ordering::Relaxed) {
        process::exit(ExitCode::KilledBySigint.into());
    }
}
