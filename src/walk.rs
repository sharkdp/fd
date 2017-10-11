use internal::{error, FdOptions};
use fshelper;
use output;

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::time;

use regex::Regex;
use ignore::{self, WalkBuilder};

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

/// Recursively scan the given search path and search for files / pathnames matching the pattern.
pub fn scan(root: &Path, pattern: Arc<Regex>, base: &Path, config: Arc<FdOptions>) {
    let (tx, rx) = channel();

    let walker = WalkBuilder::new(root)
                     .hidden(config.ignore_hidden)
                     .ignore(config.read_ignore)
                     .git_ignore(config.read_ignore)
                     .parents(config.read_ignore)
                     .git_global(config.read_ignore)
                     .git_exclude(config.read_ignore)
                     .follow_links(config.follow_links)
                     .max_depth(config.max_depth)
                     .threads(config.threads)
                     .build_parallel();

    // Spawn the thread that receives all results through the channel.
    let rx_config = Arc::clone(&config);
    let rx_base = base.to_owned();
    let receiver_thread = thread::spawn(move || {
        let start = time::Instant::now();

        let mut buffer = vec![];

        // Start in buffering mode
        let mut mode = ReceiverMode::Buffering;

        // Maximum time to wait before we start streaming to the console.
        let max_buffer_time = rx_config.max_buffer_time
                                       .unwrap_or_else(|| time::Duration::from_millis(100));

        for value in rx {
            match mode {
                ReceiverMode::Buffering => {
                    buffer.push(value);

                    // Have we reached the maximum time?
                    if time::Instant::now() - start > max_buffer_time {
                        // Flush the buffer
                        for v in &buffer {
                            output::print_entry(&rx_base, v, &rx_config);
                        }
                        buffer.clear();

                        // Start streaming
                        mode = ReceiverMode::Streaming;
                    }
                },
                ReceiverMode::Streaming => {
                    output::print_entry(&rx_base, &value, &rx_config);
                }
            }
        }

        // If we have finished fast enough (faster than max_buffer_time), we haven't streamed
        // anything to the console, yet. In this case, sort the results and print them:
        if !buffer.is_empty() {
            buffer.sort();
            for value in buffer {
                output::print_entry(&rx_base, &value, &rx_config);
            }
        }
    });

    // Spawn the sender threads.
    walker.run(|| {
        let base = base.to_owned();
        let config = Arc::clone(&config);
        let pattern = Arc::clone(&pattern);
        let tx_thread = tx.clone();

        Box::new(move |entry_o| {
            let entry = match entry_o {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };

            // Filter out unwanted file types.
            match config.file_type {
                FileType::Any => (),
                FileType::RegularFile => if entry.file_type().map_or(false, |ft| !ft.is_file()) {
                    return ignore::WalkState::Continue;
                },
                FileType::Directory => if entry.file_type().map_or(false, |ft| !ft.is_dir()) {
                    return ignore::WalkState::Continue;
                },
                FileType::SymLink => if entry.file_type().map_or(false, |ft| !ft.is_symlink()) {
                    return ignore::WalkState::Continue;
                },
            }

            // Filter out unwanted extensions.
            if let Some(ref filter_ext) = config.extension {
                let entry_ext = entry.path().extension().map(|e| e.to_string_lossy().to_lowercase());
                if entry_ext.map_or(true, |ext| ext != *filter_ext) {
                    return ignore::WalkState::Continue;
                }
            }

            let path_rel_buf = match fshelper::path_relative_from(entry.path(), &*base) {
                Some(p) => p,
                None => error("Error: could not get relative path for directory entry.")
            };
            let path_rel = path_rel_buf.as_path();

            let search_str_o =
                if config.search_full_path {
                    Some(path_rel.to_string_lossy())
                } else {
                    path_rel.file_name()
                            .map(|f| f.to_string_lossy())
                };

            if let Some(search_str) = search_str_o {
                // TODO: take care of the unwrap call
                pattern.find(&*search_str)
                       .map(|_| tx_thread.send(path_rel_buf.to_owned()).unwrap());
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
