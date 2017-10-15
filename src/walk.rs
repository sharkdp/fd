use internal::{error, FdOptions};
use fshelper;
use output;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::time;

use regex::Regex;
use ignore::{self, WalkBuilder, DirEntry};

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

struct DirEntryResolveEngine<'a> {
    root: &'a PathBuf,
    base: &'a PathBuf,
    pattern: &'a Arc<Regex>,
    config: &'a Arc<FdOptions>,
}

impl<'a> DirEntryResolveEngine<'a> {
    fn new(
        root: &'a PathBuf,
        base: &'a PathBuf,
        config: &'a Arc<FdOptions>,
        pattern: &'a Arc<Regex>,
    ) -> Self {
        DirEntryResolveEngine {
            root: root,
            base: base,
            config: config,
            pattern: pattern,
        }
    }

    fn filter(&self, entry: &'a DirEntry) -> Option<&'a DirEntry> {
        let entry_path = entry.path();

        if entry_path == self.root {
            return None;
        }

        // Filter out unwanted file types.
        match self.config.file_type {
            FileType::Any => (),
            FileType::RegularFile => {
                if entry.file_type().map_or(false, |ft| !ft.is_file()) {
                    return None;
                }
            }
            FileType::Directory => {
                if entry.file_type().map_or(false, |ft| !ft.is_dir()) {
                    return None;
                }
            }
            FileType::SymLink => {
                if entry.file_type().map_or(false, |ft| !ft.is_symlink()) {
                    return None;
                }
            }
        }

        if let Some(ref filter_ext) = self.config.extension {
            let entry_ext = entry_path.extension().map(
                |e| e.to_string_lossy().to_lowercase(),
            );
            if entry_ext.map_or(true, |ext| ext != *filter_ext) {
                return None;
            }
        }

        Some(entry)
    }

    fn do_match(&self, entry_path: &Path) -> Option<PathBuf> {
        let search_str_o = if self.config.search_full_path {
            Some(entry_path.to_string_lossy())
        } else {
            entry_path.file_name().map(|f| f.to_string_lossy())
        };

        search_str_o.and_then(|search_str| {
            return self.pattern.find(&*search_str).map(|_| {
                let mut path_rel_buf =
                    match fshelper::path_relative_from(entry_path, &*self.base) {
                        Some(p) => p,
                        None => error("Error: could not get relative path for directory entry."),
                    };
                if path_rel_buf == PathBuf::new() {
                    path_rel_buf.push(".");
                }
                path_rel_buf.to_owned()
            });
        })
    }

    fn run(&self, entry: &DirEntry) -> Option<PathBuf> {
        self.filter(entry).and_then(|entry| {
            let entry_path = entry.path();
            self.do_match(entry_path)
        })
    }
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
                            output::print_entry(&rx_base, v, &rx_config);
                        }
                        buffer.clear();

                        // Start streaming
                        mode = ReceiverMode::Streaming;
                    }
                }
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
        let root = root.to_owned();

        Box::new(move |entry_o| {
            let engine = DirEntryResolveEngine::new(&root, &base, &config, &pattern);
            let entry = match entry_o {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };

            match engine.run(&entry) {
                Some(entry_path) => tx_thread.send(entry_path.to_owned()).unwrap(),
                None => return ignore::WalkState::Continue,
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
