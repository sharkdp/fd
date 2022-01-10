use std::ffi::OsStr;
use std::fs::{FileType, Metadata};
use std::io;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::{borrow::Cow, io::Write};

use anyhow::{anyhow, Result};
use ignore::overrides::OverrideBuilder;
use ignore::{self, WalkBuilder};
use once_cell::unsync::OnceCell;
use regex::bytes::Regex;

use crate::config::Config;
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
pub enum WorkerResult {
    Entry(PathBuf),
    Error(ignore::Error),
}

/// Maximum size of the output buffer before flushing results to the console
pub const MAX_BUFFER_LENGTH: usize = 1000;
/// Default duration until output buffering switches to streaming.
pub const DEFAULT_MAX_BUFFER_TIME: Duration = Duration::from_millis(100);

/// Recursively scan the given search path for files / pathnames matching the pattern.
///
/// If the `--exec` argument was supplied, this will create a thread pool for executing
/// jobs in parallel from a given command line and the discovered paths. Otherwise, each
/// path will simply be written to standard output.
pub fn scan(path_vec: &[PathBuf], pattern: Arc<Regex>, config: Arc<Config>) -> Result<ExitCode> {
    let mut path_iter = path_vec.iter();
    let first_path_buf = path_iter
        .next()
        .expect("Error: Path vector can not be empty");
    let (tx, rx) = channel();

    let mut override_builder = OverrideBuilder::new(first_path_buf.as_path());

    for pattern in &config.exclude_patterns {
        override_builder
            .add(pattern)
            .map_err(|e| anyhow!("Malformed exclude pattern: {}", e))?;
    }
    let overrides = override_builder
        .build()
        .map_err(|_| anyhow!("Mismatch in exclude patterns"))?;

    let mut walker = WalkBuilder::new(first_path_buf.as_path());
    walker
        .hidden(config.ignore_hidden)
        .ignore(config.read_fdignore)
        .parents(config.read_parent_ignore && (config.read_fdignore || config.read_vcsignore))
        .git_ignore(config.read_vcsignore)
        .git_global(config.read_vcsignore)
        .git_exclude(config.read_vcsignore)
        .overrides(overrides)
        .follow_links(config.follow_links)
        // No need to check for supported platforms, option is unavailable on unsupported ones
        .same_file_system(config.one_file_system)
        .max_depth(config.max_depth);

    if config.read_fdignore {
        walker.add_custom_ignore_filename(".fdignore");
    }

    if config.read_global_ignore {
        #[cfg(target_os = "macos")]
        let config_dir_op = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| dirs_next::home_dir().map(|d| d.join(".config")));

        #[cfg(not(target_os = "macos"))]
        let config_dir_op = dirs_next::config_dir();

        if let Some(global_ignore_file) = config_dir_op
            .map(|p| p.join("fd").join("ignore"))
            .filter(|p| p.is_file())
        {
            let result = walker.add_ignore(global_ignore_file);
            match result {
                Some(ignore::Error::Partial(_)) => (),
                Some(err) => {
                    print_error(format!(
                        "Malformed pattern in global ignore file. {}.",
                        err.to_string()
                    ));
                }
                None => (),
            }
        }
    }

    for ignore_file in &config.ignore_files {
        let result = walker.add_ignore(ignore_file);
        match result {
            Some(ignore::Error::Partial(_)) => (),
            Some(err) => {
                print_error(format!(
                    "Malformed pattern in custom ignore file. {}.",
                    err.to_string()
                ));
            }
            None => (),
        }
    }

    for path_entry in path_iter {
        walker.add(path_entry.as_path());
    }

    let parallel_walker = walker.threads(config.threads).build_parallel();

    // Flag for cleanly shutting down the parallel walk
    let quit_flag = Arc::new(AtomicBool::new(false));
    // Flag specifically for quitting due to ^C
    let interrupt_flag = Arc::new(AtomicBool::new(false));

    if config.ls_colors.is_some() && config.command.is_none() {
        let quit_flag = Arc::clone(&quit_flag);
        let interrupt_flag = Arc::clone(&interrupt_flag);

        ctrlc::set_handler(move || {
            quit_flag.store(true, Ordering::Relaxed);

            if interrupt_flag.fetch_or(true, Ordering::Relaxed) {
                // Ctrl-C has been pressed twice, exit NOW
                ExitCode::KilledBySigint.exit();
            }
        })
        .unwrap();
    }

    // Spawn the thread that receives all results through the channel.
    let receiver_thread = spawn_receiver(&config, &quit_flag, &interrupt_flag, rx);

    // Spawn the sender threads.
    spawn_senders(&config, &quit_flag, pattern, parallel_walker, tx);

    // Wait for the receiver thread to print out all results.
    let exit_code = receiver_thread.join().unwrap();

    if interrupt_flag.load(Ordering::Relaxed) {
        Ok(ExitCode::KilledBySigint)
    } else {
        Ok(exit_code)
    }
}

/// Wrapper for the receiver thread's buffering behavior.
struct ReceiverBuffer<W> {
    /// The configuration.
    config: Arc<Config>,
    /// For shutting down the senders.
    quit_flag: Arc<AtomicBool>,
    /// The ^C notifier.
    interrupt_flag: Arc<AtomicBool>,
    /// Receiver for worker results.
    rx: Receiver<WorkerResult>,
    /// Standard output.
    stdout: W,
    /// The current buffer mode.
    mode: ReceiverMode,
    /// The deadline to switch to streaming mode.
    deadline: Instant,
    /// The buffer of quickly received paths.
    buffer: Vec<PathBuf>,
    /// Result count.
    num_results: usize,
}

impl<W: Write> ReceiverBuffer<W> {
    /// Create a new receiver buffer.
    fn new(
        config: Arc<Config>,
        quit_flag: Arc<AtomicBool>,
        interrupt_flag: Arc<AtomicBool>,
        rx: Receiver<WorkerResult>,
        stdout: W,
    ) -> Self {
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
    fn recv(&self) -> Result<WorkerResult, RecvTimeoutError> {
        match self.mode {
            ReceiverMode::Buffering => {
                // Wait at most until we should switch to streaming
                let now = Instant::now();
                self.deadline
                    .checked_duration_since(now)
                    .ok_or(RecvTimeoutError::Timeout)
                    .and_then(|t| self.rx.recv_timeout(t))
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
            Ok(WorkerResult::Entry(path)) => {
                if self.config.quiet {
                    return Err(ExitCode::HasResults(true));
                }

                match self.mode {
                    ReceiverMode::Buffering => {
                        self.buffer.push(path);
                        if self.buffer.len() > MAX_BUFFER_LENGTH {
                            self.stream()?;
                        }
                    }
                    ReceiverMode::Streaming => {
                        self.print(&path)?;
                        self.flush()?;
                    }
                }

                self.num_results += 1;
                if let Some(max_results) = self.config.max_results {
                    if self.num_results >= max_results {
                        return self.stop();
                    }
                }
            }
            Ok(WorkerResult::Error(err)) => {
                if self.config.show_filesystem_errors {
                    print_error(err.to_string());
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
    fn print(&mut self, path: &Path) -> Result<(), ExitCode> {
        output::print_entry(&mut self.stdout, path, &self.config);

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
        if self.config.interactive_terminal && self.stdout.flush().is_err() {
            // Probably a broken pipe. Exit gracefully.
            return Err(ExitCode::GeneralError);
        }
        Ok(())
    }
}

fn spawn_receiver(
    config: &Arc<Config>,
    quit_flag: &Arc<AtomicBool>,
    interrupt_flag: &Arc<AtomicBool>,
    rx: Receiver<WorkerResult>,
) -> thread::JoinHandle<ExitCode> {
    let config = Arc::clone(config);
    let quit_flag = Arc::clone(quit_flag);
    let interrupt_flag = Arc::clone(interrupt_flag);

    let show_filesystem_errors = config.show_filesystem_errors;
    let threads = config.threads;
    // This will be used to check if output should be buffered when only running a single thread
    let enable_output_buffering: bool = threads > 1;
    thread::spawn(move || {
        // This will be set to `Some` if the `--exec` argument was supplied.
        if let Some(ref cmd) = config.command {
            if cmd.in_batch_mode() {
                exec::batch(
                    rx,
                    cmd,
                    show_filesystem_errors,
                    enable_output_buffering,
                    config.batch_size,
                )
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
                    let handle = thread::spawn(move || {
                        exec::job(
                            rx,
                            cmd,
                            out_perm,
                            show_filesystem_errors,
                            enable_output_buffering,
                        )
                    });

                    // Push the handle of the spawned thread into the vector for later joining.
                    handles.push(handle);
                }

                // Wait for all threads to exit before exiting the program.
                let exit_codes = handles
                    .into_iter()
                    .map(|handle| handle.join().unwrap())
                    .collect::<Vec<_>>();
                merge_exitcodes(exit_codes)
            }
        } else {
            let stdout = io::stdout();
            let stdout = stdout.lock();
            let stdout = io::BufWriter::new(stdout);

            let mut rxbuffer = ReceiverBuffer::new(config, quit_flag, interrupt_flag, rx, stdout);
            rxbuffer.process()
        }
    })
}

enum DirEntryInner {
    Normal(ignore::DirEntry),
    BrokenSymlink(PathBuf),
}

pub struct DirEntry {
    inner: DirEntryInner,
    metadata: OnceCell<Option<Metadata>>,
}

impl DirEntry {
    fn normal(e: ignore::DirEntry) -> Self {
        Self {
            inner: DirEntryInner::Normal(e),
            metadata: OnceCell::new(),
        }
    }

    fn broken_symlink(path: PathBuf) -> Self {
        Self {
            inner: DirEntryInner::BrokenSymlink(path),
            metadata: OnceCell::new(),
        }
    }

    pub fn path(&self) -> &Path {
        match &self.inner {
            DirEntryInner::Normal(e) => e.path(),
            DirEntryInner::BrokenSymlink(pathbuf) => pathbuf.as_path(),
        }
    }

    pub fn file_type(&self) -> Option<FileType> {
        match &self.inner {
            DirEntryInner::Normal(e) => e.file_type(),
            DirEntryInner::BrokenSymlink(_) => self.metadata().map(|m| m.file_type()),
        }
    }

    pub fn is_symlink(&self) -> bool {
        match &self.inner {
            DirEntryInner::Normal(e) => e.path_is_symlink(),
            DirEntryInner::BrokenSymlink(_) => true,
        }
    }

    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata
            .get_or_init(|| match &self.inner {
                DirEntryInner::Normal(e) => e.metadata().ok(),
                DirEntryInner::BrokenSymlink(path) => path.symlink_metadata().ok(),
            })
            .as_ref()
    }

    pub fn depth(&self) -> Option<usize> {
        match &self.inner {
            DirEntryInner::Normal(e) => Some(e.depth()),
            DirEntryInner::BrokenSymlink(_) => None,
        }
    }
}

fn spawn_senders(
    config: &Arc<Config>,
    quit_flag: &Arc<AtomicBool>,
    pattern: Arc<Regex>,
    parallel_walker: ignore::WalkParallel,
    tx: Sender<WorkerResult>,
) {
    parallel_walker.run(|| {
        let config = Arc::clone(config);
        let pattern = Arc::clone(&pattern);
        let tx_thread = tx.clone();
        let quit_flag = Arc::clone(quit_flag);

        Box::new(move |entry_o| {
            if quit_flag.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }

            let entry = match entry_o {
                Ok(ref e) if e.depth() == 0 => {
                    // Skip the root directory entry.
                    return ignore::WalkState::Continue;
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
                                .map_or(false, |m| m.file_type().is_symlink()) =>
                    {
                        DirEntry::broken_symlink(path)
                    }
                    _ => {
                        return match tx_thread.send(WorkerResult::Error(ignore::Error::WithPath {
                            path,
                            err: inner_err,
                        })) {
                            Ok(_) => ignore::WalkState::Continue,
                            Err(_) => ignore::WalkState::Quit,
                        }
                    }
                },
                Err(err) => {
                    return match tx_thread.send(WorkerResult::Error(err)) {
                        Ok(_) => ignore::WalkState::Continue,
                        Err(_) => ignore::WalkState::Quit,
                    }
                }
            };

            if let Some(min_depth) = config.min_depth {
                if entry.depth().map_or(true, |d| d < min_depth) {
                    return ignore::WalkState::Continue;
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

            if !pattern.is_match(&filesystem::osstr_to_bytes(search_str.as_ref())) {
                return ignore::WalkState::Continue;
            }

            // Filter out unwanted extensions.
            if let Some(ref exts_regex) = config.extensions {
                if let Some(path_str) = entry_path.file_name() {
                    if !exts_regex.is_match(&filesystem::osstr_to_bytes(path_str)) {
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            // Filter out unwanted file types.
            if let Some(ref file_types) = config.file_types {
                if file_types.should_ignore(&entry) {
                    return ignore::WalkState::Continue;
                }
            }

            #[cfg(unix)]
            {
                if let Some(ref owner_constraint) = config.owner_constraint {
                    if let Some(metadata) = entry.metadata() {
                        if !owner_constraint.matches(metadata) {
                            return ignore::WalkState::Continue;
                        }
                    } else {
                        return ignore::WalkState::Continue;
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
                if let Some(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        matched = config
                            .time_constraints
                            .iter()
                            .all(|tf| tf.applies_to(&modified));
                    }
                }
                if !matched {
                    return ignore::WalkState::Continue;
                }
            }

            let send_result = tx_thread.send(WorkerResult::Entry(entry_path.to_owned()));

            if send_result.is_err() {
                return ignore::WalkState::Quit;
            }

            // Apply pruning.
            if config.prune {
                return ignore::WalkState::Skip;
            }

            ignore::WalkState::Continue
        })
    });
}
