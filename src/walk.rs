// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::exec;
use crate::exit_codes::ExitCode;
use crate::fshelper;
use crate::internal::{opts::FdOptions, MAX_BUFFER_LENGTH};
use crate::output;

use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
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
        match result {
            Some(ignore::Error::Partial(_)) => (),
            Some(err) => {
                print_error!(
                    "{}",
                    format!(
                        "Malformed pattern in custom ignore file '{}': {}.",
                        ignore_file.to_string_lossy(),
                        err.description()
                    )
                );
            }
            None => (),
        }
    }

    for path_entry in path_iter {
        walker.add(path_entry.as_path());
    }

    let parallel_walker = walker.threads(config.threads).build_parallel();

    let wants_to_quit = Arc::new(AtomicBool::new(false));
    if config.ls_colors.is_some() && config.command.is_none() {
        let wq = Arc::clone(&wants_to_quit);
        ctrlc::set_handler(move || {
            wq.store(true, Ordering::Relaxed);
        })
        .unwrap();
    }

    // Spawn the thread that receives all results through the channel.
    let receiver_thread = spawn_receiver(&config, &wants_to_quit, rx);

    // Spawn the sender threads.
    spawn_senders(&config, &wants_to_quit, pattern, parallel_walker, tx);

    // Wait for the receiver thread to print out all results.
    receiver_thread.join().unwrap();

    if wants_to_quit.load(Ordering::Relaxed) {
        process::exit(ExitCode::KilledBySigint.into());
    }
}

fn spawn_receiver(
    config: &Arc<FdOptions>,
    wants_to_quit: &Arc<AtomicBool>,
    rx: Receiver<WorkerResult>,
) -> thread::JoinHandle<()> {
    let config = Arc::clone(config);
    let wants_to_quit = Arc::clone(wants_to_quit);

    let show_filesystem_errors = config.show_filesystem_errors;
    let threads = config.threads;

    thread::spawn(move || {
        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = config.command {
            if cmd.in_batch_mode() {
                exec::batch(rx, cmd, show_filesystem_errors);
            } else {
                let shared_rx = Arc::new(Mutex::new(rx));

                let out_perm = Arc::new(Mutex::new(()));

                // Each spawned job will store it's thread handle in here.
                let mut handles = Vec::with_capacity(threads);
                for _ in 0..threads {
                    let rx = Arc::clone(&shared_rx);
                    let cmd = Arc::clone(cmd);
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
            let max_buffer_time = config
                .max_buffer_time
                .unwrap_or_else(|| time::Duration::from_millis(100));

            let stdout = io::stdout();
            let mut stdout = stdout.lock();

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
                                        output::print_entry(
                                            &mut stdout,
                                            v,
                                            &config,
                                            &wants_to_quit,
                                        );
                                    }
                                    buffer.clear();

                                    // Start streaming
                                    mode = ReceiverMode::Streaming;
                                }
                            }
                            ReceiverMode::Streaming => {
                                output::print_entry(&mut stdout, &value, &config, &wants_to_quit);
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
                    output::print_entry(&mut stdout, &value, &config, &wants_to_quit);
                }
            }
        }
    })
}

fn spawn_senders(
    config: &Arc<FdOptions>,
    wants_to_quit: &Arc<AtomicBool>,
    pattern: Arc<Regex>,
    parallel_walker: ignore::WalkParallel,
    tx: Sender<WorkerResult>,
) {
    parallel_walker.run(|| {
        let config = Arc::clone(config);
        let pattern = Arc::clone(&pattern);
        let tx_thread = tx.clone();
        let wants_to_quit = Arc::clone(wants_to_quit);

        Box::new(move |entry_o| {
            if wants_to_quit.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }

            let entry = match entry_o {
                Ok(e) => e,
                Err(err) => {
                    tx_thread.send(WorkerResult::Error(err)).unwrap();
                    return ignore::WalkState::Continue;
                }
            };

            if entry.depth() == 0 {
                return ignore::WalkState::Continue;
            }

            // Filter out unwanted file types.

            if let Some(ref file_types) = config.file_types {
                if let Some(ref entry_type) = entry.file_type() {
                    if (!file_types.files && entry_type.is_file())
                        || (!file_types.directories && entry_type.is_dir())
                        || (!file_types.symlinks && entry_type.is_symlink())
                        || (file_types.executables_only
                            && !entry
                                .metadata()
                                .map(|m| fshelper::is_executable(&m))
                                .unwrap_or(false))
                        || (file_types.empty_only && !fshelper::is_empty(&entry))
                    {
                        return ignore::WalkState::Continue;
                    } else if !(entry_type.is_file()
                        || entry_type.is_dir()
                        || entry_type.is_symlink())
                    {
                        // This is probably a block device, char device, fifo or socket. Skip it.
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            let entry_path = entry.path();

            // Filter out unwanted extensions.
            if let Some(ref exts_regex) = config.extensions {
                if let Some(path_str) = entry_path.file_name().and_then(|s| s.to_str()) {
                    if !exts_regex.is_match(path_str) {
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            // Filter out unwanted sizes if it is a file and we have been given size constraints.
            if !config.size_constraints.is_empty() {
                if entry_path.is_file() {
                    if let Ok(metadata) = entry_path.metadata() {
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
                if entry_path.is_file() {
                    if let Ok(metadata) = entry_path.metadata() {
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
                match fshelper::path_absolute_form(entry_path) {
                    Ok(path_abs_buf) => Some(path_abs_buf.to_string_lossy().into_owned().into()),
                    Err(_) => {
                        print_error_and_exit!("Unable to retrieve absolute path.");
                    }
                }
            } else {
                entry_path.file_name().map(|f| f.to_string_lossy())
            };

            if let Some(search_str) = search_str_o {
                if pattern.is_match(&*search_str) {
                    // TODO: take care of the unwrap call
                    tx_thread
                        .send(WorkerResult::Entry(entry_path.to_owned()))
                        .unwrap()
                }
            }

            ignore::WalkState::Continue
        })
    });
}
