use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, Write};
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, SendError, Sender};
use etcetera::BaseStrategy;
use ignore::overrides::{Override, OverrideBuilder};
use ignore::{WalkBuilder, WalkParallel, WalkState};
use regex::bytes::Regex;

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::error::print_error;
use crate::exec;
use crate::exit_codes::{merge_exitcodes, ExitCode};
use crate::filesystem;
use crate::output;

/// The receiver thread can either be buffering results or directly streaming to the console.
#[derive(PartialEq)]
enum ReceiverMode {
    /// Receiver is still buffering in order to sort the results, if the search finishes fast
    /// enough.
    Buffering,

    /// Receiver is directly printing results to the output.
    Streaming,
}

/// The Worker threads can result in a valid entry having PathBuf or an error.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum WorkerResult {
    // Errors should be rare, so it's probably better to allow large_enum_variant than
    // to box the Entry variant
    Entry(DirEntry),
    Error(ignore::Error),
}

/// A batch of WorkerResults to send over a channel.
#[derive(Clone)]
struct Batch {
    items: Arc<Mutex<Option<Vec<WorkerResult>>>>,
}

impl Batch {
    fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(Some(vec![]))),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<Vec<WorkerResult>>> {
        self.items.lock().unwrap()
    }
}

impl IntoIterator for Batch {
    type Item = WorkerResult;
    type IntoIter = std::vec::IntoIter<WorkerResult>;

    fn into_iter(self) -> Self::IntoIter {
        self.lock().take().unwrap().into_iter()
    }
}

/// Wrapper that sends batches of items at once over a channel.
struct BatchSender {
    batch: Batch,
    tx: Sender<Batch>,
    limit: usize,
}

impl BatchSender {
    fn new(tx: Sender<Batch>, limit: usize) -> Self {
        Self {
            batch: Batch::new(),
            tx,
            limit,
        }
    }

    /// Check if we need to flush a batch.
    fn needs_flush(&self, batch: Option<&Vec<WorkerResult>>) -> bool {
        match batch {
            // Limit the batch size to provide some backpressure
            Some(vec) => vec.len() >= self.limit,
            // Batch was already taken by the receiver, so make a new one
            None => true,
        }
    }

    /// Add an item to a batch.
    fn send(&mut self, item: WorkerResult) -> Result<(), SendError<()>> {
        let mut batch = self.batch.lock();

        if self.needs_flush(batch.as_ref()) {
            drop(batch);
            self.batch = Batch::new();
            batch = self.batch.lock();
        }

        let items = batch.as_mut().unwrap();
        items.push(item);

        if items.len() == 1 {
            // New batch, send it over the channel
            self.tx
                .send(self.batch.clone())
                .map_err(|_| SendError(()))?;
        }

        Ok(())
    }
}

/// Maximum size of the output buffer before flushing results to the console
const MAX_BUFFER_LENGTH: usize = 1000;
/// Default duration until output buffering switches to streaming.
const DEFAULT_MAX_BUFFER_TIME: Duration = Duration::from_millis(100);

/// Wrapper for the receiver thread's buffering behavior.
struct ReceiverBuffer<'a, W> {
    /// The configuration.
    config: &'a Config,
    /// For shutting down the senders.
    quit_flag: &'a AtomicBool,
    /// The ^C notifier.
    interrupt_flag: &'a AtomicBool,
    /// Receiver for worker results.
    rx: Receiver<Batch>,
    /// Standard output.
    stdout: W,
    /// The current buffer mode.
    mode: ReceiverMode,
    /// The deadline to switch to streaming mode.
    deadline: Instant,
    /// The buffer of quickly received paths.
    buffer: Vec<DirEntry>,
    /// Result count.
    num_results: usize,
}

impl<'a, W: Write> ReceiverBuffer<'a, W> {
    /// Create a new receiver buffer.
    fn new(state: &'a WorkerState, rx: Receiver<Batch>, stdout: W) -> Self {
        let config = &state.config;
        let quit_flag = state.quit_flag.as_ref();
        let interrupt_flag = state.interrupt_flag.as_ref();
        let max_buffer_time = config.max_buffer_time.unwrap_or(DEFAULT_MAX_BUFFER_TIME);
        let deadline = Instant::now() + max_buffer_time;

        Self {
            config,
            quit_flag,
            interrupt_flag,
            rx,
            stdout,
            mode: ReceiverMode::Buffering,
            deadline,
            buffer: Vec::with_capacity(MAX_BUFFER_LENGTH),
            num_results: 0,
        }
    }

    /// Process results until finished.
    fn process(&mut self) -> ExitCode {
        loop {
            if let Err(ec) = self.poll() {
                self.quit_flag.store(true, Ordering::Relaxed);
                return ec;
            }
        }
    }

    /// Receive the next worker result.
    fn recv(&self) -> Result<Batch, RecvTimeoutError> {
        match self.mode {
            ReceiverMode::Buffering => {
                // Wait at most until we should switch to streaming
                self.rx.recv_deadline(self.deadline)
            }
            ReceiverMode::Streaming => {
                // Wait however long it takes for a result
                Ok(self.rx.recv()?)
            }
        }
    }

    /// Wait for a result or state change.
    fn poll(&mut self) -> Result<(), ExitCode> {
        match self.recv() {
            Ok(batch) => {
                for result in batch {
                    match result {
                        WorkerResult::Entry(dir_entry) => {
                            if self.config.quiet {
                                return Err(ExitCode::HasResults(true));
                            }

                            match self.mode {
                                ReceiverMode::Buffering => {
                                    self.buffer.push(dir_entry);
                                    if self.buffer.len() > MAX_BUFFER_LENGTH {
                                        self.stream()?;
                                    }
                                }
                                ReceiverMode::Streaming => {
                                    self.print(&dir_entry)?;
                                }
                            }

                            self.num_results += 1;
                            if let Some(max_results) = self.config.max_results {
                                if self.num_results >= max_results {
                                    return self.stop();
                                }
                            }
                        }
                        WorkerResult::Error(err) => {
                            if self.config.show_filesystem_errors {
                                print_error(err.to_string());
                            }
                        }
                    }
                }

                // If we don't have another batch ready, flush before waiting
                if self.mode == ReceiverMode::Streaming && self.rx.is_empty() {
                    self.flush()?;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                self.stream()?;
            }
            Err(RecvTimeoutError::Disconnected) => {
                return self.stop();
            }
        }

        Ok(())
    }

    /// Output a path.
    fn print(&mut self, entry: &DirEntry) -> Result<(), ExitCode> {
        if let Err(e) = output::print_entry(&mut self.stdout, entry, self.config) {
            if e.kind() != ::std::io::ErrorKind::BrokenPipe {
                print_error(format!("Could not write to output: {e}"));
                return Err(ExitCode::GeneralError);
            }
        }

        if self.interrupt_flag.load(Ordering::Relaxed) {
            // Ignore any errors on flush, because we're about to exit anyway
            let _ = self.flush();
            return Err(ExitCode::KilledBySigint);
        }

        Ok(())
    }

    /// Switch ourselves into streaming mode.
    fn stream(&mut self) -> Result<(), ExitCode> {
        self.mode = ReceiverMode::Streaming;

        let buffer = mem::take(&mut self.buffer);
        for path in buffer {
            self.print(&path)?;
        }

        self.flush()
    }

    /// Stop looping.
    fn stop(&mut self) -> Result<(), ExitCode> {
        if self.mode == ReceiverMode::Buffering {
            self.buffer.sort();
            self.stream()?;
        }

        if self.config.quiet {
            Err(ExitCode::HasResults(self.num_results > 0))
        } else {
            Err(ExitCode::Success)
        }
    }

    /// Flush stdout if necessary.
    fn flush(&mut self) -> Result<(), ExitCode> {
        if self.stdout.flush().is_err() {
            // Probably a broken pipe. Exit gracefully.
            return Err(ExitCode::GeneralError);
        }
        Ok(())
    }
}

/// State shared by the sender and receiver threads.
struct WorkerState {
    /// The search patterns.
    patterns: Vec<Regex>,
    /// The command line configuration.
    config: Config,
    /// Flag for cleanly shutting down the parallel walk
    quit_flag: Arc<AtomicBool>,
    /// Flag specifically for quitting due to ^C
    interrupt_flag: Arc<AtomicBool>,
}

impl WorkerState {
    fn new(patterns: Vec<Regex>, config: Config) -> Self {
        let quit_flag = Arc::new(AtomicBool::new(false));
        let interrupt_flag = Arc::new(AtomicBool::new(false));

        Self {
            patterns,
            config,
            quit_flag,
            interrupt_flag,
        }
    }

    fn build_overrides(&self, paths: &[PathBuf]) -> Result<Override> {
        let first_path = &paths[0];
        let config = &self.config;

        let mut builder = OverrideBuilder::new(first_path);

        for pattern in &config.exclude_patterns {
            builder
                .add(pattern)
                .map_err(|e| anyhow!("Malformed exclude pattern: {}", e))?;
        }

        builder
            .build()
            .map_err(|_| anyhow!("Mismatch in exclude patterns"))
    }

    fn build_walker(&self, paths: &[PathBuf]) -> Result<WalkParallel> {
        let first_path = &paths[0];
        let config = &self.config;
        let overrides = self.build_overrides(paths)?;

        let mut builder = WalkBuilder::new(first_path);

        // Settings that are always applied or controlled by other specific flags
        builder
            .hidden(config.ignore_hidden)
            .overrides(overrides) // Apply command-line exclude patterns (--exclude)
            .follow_links(config.follow_links)
            .same_file_system(config.one_file_system)
            .max_depth(config.max_depth);

        if let Some(custom_ignore_name) = &config.custom_ignore_file_name {
            // A custom ignore file name is specified: disable other file-based ignores
            builder.ignore(false);         // Do not look for default ".ignore" files
            builder.git_ignore(false);     // Do not use .gitignore
            builder.git_global(false);     // Do not use global git ignore
            builder.git_exclude(false);    // Do not use .git/info/exclude
            // .require_git(false) might also be set here, but if git_ignore is false, it may not be needed.
            builder.parents(config.read_parent_ignore); // Governs if custom_ignore_name is sought in parent dirs
            builder.add_custom_ignore_filename(custom_ignore_name);
        } else {
            // Default ignore file behavior
            builder.ignore(config.read_fdignore); // Look for ".ignore" if read_fdignore is true
            builder.parents(config.read_parent_ignore && (config.read_fdignore || config.read_vcsignore));
            builder.git_ignore(config.read_vcsignore);
            builder.git_global(config.read_vcsignore);
            builder.git_exclude(config.read_vcsignore);
            builder.require_git(config.require_git_to_read_vcsignore);

            if config.read_fdignore {
                builder.add_custom_ignore_filename(".fdignore");
            }

            if config.read_global_ignore {
                if let Ok(basedirs) = etcetera::choose_base_strategy() {
                    let global_ignore_file = basedirs.config_dir().join("fd").join("ignore");
                    if global_ignore_file.is_file() {
                        let result = builder.add_ignore(&global_ignore_file); // Pass by reference
                        match result {
                            Some(ignore::Error::Partial(partial_err)) => {
                                print_error(format!(
                                    "Partially malformed pattern in global ignore file {}: {:?}.",
                                    global_ignore_file.display(),
                                    partial_err
                                ));
                            }
                            Some(err) => {
                                print_error(format!(
                                    "Malformed pattern in global ignore file {}: {:?}.",
                                    global_ignore_file.display(),
                                    err
                                ));
                            }
                            None => (),
                        }
                    }
                }
            }
        }

        // Add ignore files specified with --ignore-file (these are explicit paths from CLI)
        for ignore_file_path in &config.ignore_files {
            let result = builder.add_ignore(ignore_file_path); // Pass by reference
            match result {
                Some(ignore::Error::Partial(partial_err)) => {
                     print_error(format!(
                        "Partially malformed pattern in custom ignore file {}: {:?}.",
                        ignore_file_path.display(),
                        partial_err
                    ));
                }
                Some(err) => {
                    print_error(format!(
                        "Malformed pattern in custom ignore file {}: {}.",
                        ignore_file_path.display(),
                        err
                    ));
                }
                None => (),
            }
        }

        for path in &paths[1..] {
            builder.add(path);
        }

        let walker = builder.threads(config.threads).build_parallel();
        Ok(walker)
    }

    /// Run the receiver work, either on this thread or a pool of background
    /// threads (for --exec).
    fn receive(&self, rx: Receiver<Batch>) -> ExitCode {
        let config = &self.config;

        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = config.command {
            if cmd.in_batch_mode() {
                exec::batch(rx.into_iter().flatten(), cmd, config)
            } else {
                let out_perm = Mutex::new(());

                thread::scope(|scope| {
                    // Each spawned job will store its thread handle in here.
                    let threads = config.threads;
                    let mut handles = Vec::with_capacity(threads);
                    for _ in 0..threads {
                        let rx = rx.clone();

                        // Spawn a job thread that will listen for and execute inputs.
                        let handle = scope
                            .spawn(|| exec::job(rx.into_iter().flatten(), cmd, &out_perm, config));

                        // Push the handle of the spawned thread into the vector for later joining.
                        handles.push(handle);
                    }
                    let exit_codes = handles.into_iter().map(|handle| handle.join().unwrap());
                    merge_exitcodes(exit_codes)
                })
            }
        } else {
            let stdout = io::stdout().lock();
            let stdout = io::BufWriter::new(stdout);

            ReceiverBuffer::new(self, rx, stdout).process()
        }
    }

    /// Spawn the sender threads.
    fn spawn_senders(&self, walker: WalkParallel, tx: Sender<Batch>) {
        walker.run(|| {
            let patterns = &self.patterns;
            let config = &self.config;
            let quit_flag = self.quit_flag.as_ref();

            let mut limit = 0x100;
            if let Some(cmd) = &config.command {
                if !cmd.in_batch_mode() && config.threads > 1 {
                    // Evenly distribute work between multiple receivers
                    limit = 1;
                }
            }
            let mut tx = BatchSender::new(tx.clone(), limit);

            Box::new(move |entry_result| {
                if quit_flag.load(Ordering::Relaxed) {
                    return WalkState::Quit;
                }

                // Handle empty custom ignore file: if a custom ignore file name is specified,
                // and a file with that name exists in the current entry's directory (if entry is dir)
                // or parent directory (if entry is file), and that file is empty (modulo comments/whitespace),
                // then skip this entry and do not recurse if it's a directory.
                if let Some(custom_ignore_name) = &config.custom_ignore_file_name {
                    if let Ok(ref live_entry) = entry_result { // Process only if entry is not an error
                        let path_of_entry = live_entry.path();
                        let mut dir_to_check_for_ignore_file = PathBuf::new();

                        if live_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                            dir_to_check_for_ignore_file.push(path_of_entry);
                        } else if let Some(parent) = path_of_entry.parent() {
                            dir_to_check_for_ignore_file.push(parent);
                        } else {
                            // Should not happen for entries from WalkParallel, but handle defensively
                            dir_to_check_for_ignore_file.push(".");
                        }

                        let custom_ignore_file_on_disk = dir_to_check_for_ignore_file.join(custom_ignore_name);

                        if custom_ignore_file_on_disk.is_file() {
                            // TODO: Consider caching the empty-check result per directory path
                            // to avoid repeated file I/O and parsing for entries in the same directory.
                            // For now, re-evaluate each time for simplicity.
                            let mut is_truly_empty = true;
                            match fs::File::open(&custom_ignore_file_on_disk) {
                                Ok(file) => {
                                    let reader = io::BufReader::new(file);
                                    for line_res in reader.lines() {
                                        if let Ok(line) = line_res {
                                            let trimmed = line.trim();
                                            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                                                is_truly_empty = false;
                                                break;
                                            }
                                        } else { // Error reading a line
                                            is_truly_empty = false;
                                            break;
                                        }
                                    }
                                }
                                Err(_e) => { // File could not be opened (e.g., permissions, or disappeared)
                                    is_truly_empty = false;
                                    // Optionally log error: print_error(format!("Could not open custom ignore file {}: {}", custom_ignore_file_on_disk.display(), _e));
                                }
                            }

                            if is_truly_empty {
                                // If the custom ignore file in this entry's effective directory is empty,
                                // skip this entry. If this entry is a directory, WalkState::Skip also prevents recursion.
                                return WalkState::Skip;
                            }
                        }
                    }
                }

                let entry = match entry_result {
                    Ok(ref e) if e.depth() == 0 => {
                        // Skip the root directory entry.
                        return WalkState::Continue;
                    }
                    Ok(e) => DirEntry::normal(e),
                    Err(ignore::Error::WithPath {
                        path,
                        err: inner_err,
                    }) => match inner_err.as_ref() {
                        ignore::Error::Io(io_error)
                            if io_error.kind() == io::ErrorKind::NotFound
                                && path
                                    .symlink_metadata()
                                    .ok()
                                    .is_some_and(|m| m.file_type().is_symlink()) =>
                        {
                            DirEntry::broken_symlink(path)
                        }
                        _ => {
                            return match tx.send(WorkerResult::Error(ignore::Error::WithPath {
                                path,
                                err: inner_err,
                            })) {
                                Ok(_) => WalkState::Continue,
                                Err(_) => WalkState::Quit,
                            }
                        }
                    },
                    Err(err) => {
                        return match tx.send(WorkerResult::Error(err)) {
                            Ok(_) => WalkState::Continue,
                            Err(_) => WalkState::Quit,
                        }
                    }
                };

                if let Some(min_depth) = config.min_depth {
                    if entry.depth().map_or(true, |d| d < min_depth) {
                        return WalkState::Continue;
                    }
                }

                // Check the name first, since it doesn't require metadata
                let entry_path = entry.path();

                let search_str: Cow<OsStr> = if config.search_full_path {
                    let path_abs_buf = filesystem::path_absolute_form(entry_path)
                        .expect("Retrieving absolute path succeeds");
                    Cow::Owned(path_abs_buf.as_os_str().to_os_string())
                } else {
                    match entry_path.file_name() {
                        Some(filename) => Cow::Borrowed(filename),
                        None => unreachable!(
                            "Encountered file system entry without a file name. This should only \
                             happen for paths like 'foo/bar/..' or '/' which are not supposed to \
                             appear in a file system traversal."
                        ),
                    }
                };

                if !patterns
                    .iter()
                    .all(|pat| pat.is_match(&filesystem::osstr_to_bytes(search_str.as_ref())))
                {
                    return WalkState::Continue;
                }

                // Filter out unwanted extensions.
                if let Some(ref exts_regex) = config.extensions {
                    if let Some(path_str) = entry_path.file_name() {
                        if !exts_regex.is_match(&filesystem::osstr_to_bytes(path_str)) {
                            return WalkState::Continue;
                        }
                    } else {
                        return WalkState::Continue;
                    }
                }

                // Filter out unwanted file types.
                if let Some(ref file_types) = config.file_types {
                    if file_types.should_ignore(&entry) {
                        return WalkState::Continue;
                    }
                }

                #[cfg(unix)]
                {
                    if let Some(ref owner_constraint) = config.owner_constraint {
                        if let Some(metadata) = entry.metadata() {
                            if !owner_constraint.matches(metadata) {
                                return WalkState::Continue;
                            }
                        } else {
                            return WalkState::Continue;
                        }
                    }
                }

                // Filter out unwanted sizes if it is a file and we have been given size constraints.
                if !config.size_constraints.is_empty() {
                    if entry_path.is_file() {
                        if let Some(metadata) = entry.metadata() {
                            let file_size = metadata.len();
                            if config
                                .size_constraints
                                .iter()
                                .any(|sc| !sc.is_within(file_size))
                            {
                                return WalkState::Continue;
                            }
                        } else {
                            return WalkState::Continue;
                        }
                    } else {
                        return WalkState::Continue;
                    }
                }

                // Filter out unwanted modification times
                if !config.time_constraints.is_empty() {
                    let mut matched = false;
                    if let Some(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            matched = config
                                .time_constraints
                                .iter()
                                .all(|tf| tf.applies_to(&modified));
                        }
                    }
                    if !matched {
                        return WalkState::Continue;
                    }
                }

                if config.is_printing() {
                    if let Some(ls_colors) = &config.ls_colors {
                        // Compute colors in parallel
                        entry.style(ls_colors);
                    }
                }

                let send_result = tx.send(WorkerResult::Entry(entry));

                if send_result.is_err() {
                    return WalkState::Quit;
                }

                // Apply pruning.
                if config.prune {
                    return WalkState::Skip;
                }

                WalkState::Continue
            })
        });
    }

    /// Perform the recursive scan.
    fn scan(&self, paths: &[PathBuf]) -> Result<ExitCode> {
        let config = &self.config;
        let walker = self.build_walker(paths)?;

        if config.ls_colors.is_some() && config.is_printing() {
            let quit_flag = Arc::clone(&self.quit_flag);
            let interrupt_flag = Arc::clone(&self.interrupt_flag);

            ctrlc::set_handler(move || {
                quit_flag.store(true, Ordering::Relaxed);

                if interrupt_flag.fetch_or(true, Ordering::Relaxed) {
                    // Ctrl-C has been pressed twice, exit NOW
                    ExitCode::KilledBySigint.exit();
                }
            })
            .unwrap();
        }

        let (tx, rx) = bounded(2 * config.threads);

        let exit_code = thread::scope(|scope| {
            // Spawn the receiver thread(s)
            let receiver = scope.spawn(|| self.receive(rx));

            // Spawn the sender threads.
            self.spawn_senders(walker, tx);

            receiver.join().unwrap()
        });

        if self.interrupt_flag.load(Ordering::Relaxed) {
            Ok(ExitCode::KilledBySigint)
        } else {
            Ok(exit_code)
        }
    }
}

/// Recursively scan the given search path for files / pathnames matching the patterns.
///
/// If the `--exec` argument was supplied, this will create a thread pool for executing
/// jobs in parallel from a given command line and the discovered paths. Otherwise, each
/// path will simply be written to standard output.
pub fn scan(paths: &[PathBuf], patterns: Vec<Regex>, config: Config) -> Result<ExitCode> {
    WorkerState::new(patterns, config).scan(paths)
}
