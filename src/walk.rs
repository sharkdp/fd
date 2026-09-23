use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use crossbeam_channel::{Receiver, RecvTimeoutError, SendError, Sender, bounded};
use etcetera::BaseStrategy;
use ignore::overrides::{Override, OverrideBuilder};
use ignore::{WalkBuilder, WalkParallel, WalkState};
use regex::bytes::Regex;

use crate::config::Config;
use crate::dir_entry::DirEntry;
use crate::exec;
use crate::exit_codes::{ExitCode, merge_exitcodes};
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

/// Maximum wait before flushing pending streamed output.
const MAX_FLUSH_DELAY: Duration = Duration::from_millis(100);

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
    /// The deadline to flush pending streamed output.
    flush_deadline: Option<Instant>,
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
            flush_deadline: None,
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
                if let Some(deadline) = self.flush_deadline {
                    // Give workers time to send another batch before flushing.
                    self.rx.recv_deadline(deadline)
                } else {
                    // No pending output: wait without waking up periodically.
                    Ok(self.rx.recv()?)
                }
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
                            if let Some(max_results) = self.config.max_results
                                && self.num_results >= max_results
                            {
                                return self.stop();
                            }
                        }
                        WorkerResult::Error(err) => {
                            if self.config.show_filesystem_errors {
                                print_error!("{}", err);
                            }
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => match self.mode {
                ReceiverMode::Buffering => self.stream()?,
                ReceiverMode::Streaming => self.flush()?,
            },
            Err(RecvTimeoutError::Disconnected) => {
                return self.stop();
            }
        }

        Ok(())
    }

    /// Output a path.
    fn print(&mut self, entry: &DirEntry) -> Result<(), ExitCode> {
        if let Err(e) = output::print_entry(&mut self.stdout, entry, self.config)
            && e.kind() != ::std::io::ErrorKind::BrokenPipe
        {
            print_error!("Could not write to output: {}", e);
            return Err(ExitCode::GeneralError);
        }

        // Keep the first deadline so a trickle of results cannot postpone the flush.
        self.flush_deadline
            .get_or_insert_with(|| Instant::now() + MAX_FLUSH_DELAY);

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
        } else if self.flush_deadline.is_some() {
            self.flush()?;
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
        self.flush_deadline = None;
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
        builder
            .hidden(config.ignore_hidden)
            .ignore(config.read_fdignore)
            .parents(config.read_parent_ignore && (config.read_fdignore || config.read_vcsignore))
            .git_ignore(config.read_vcsignore)
            .git_global(config.read_vcsignore)
            .git_exclude(config.read_vcsignore)
            .require_git(config.require_git_to_read_vcsignore)
            .overrides(overrides)
            .follow_links(config.follow_links)
            // No need to check for supported platforms, option is unavailable on unsupported ones
            .same_file_system(config.one_file_system)
            .max_depth(config.max_depth);

        if config.read_fdignore {
            builder.add_custom_ignore_filename(".fdignore");
        }

        if config.read_global_ignore
            && let Ok(basedirs) = etcetera::choose_base_strategy()
        {
            let global_ignore_file = basedirs.config_dir().join("fd").join("ignore");
            if global_ignore_file.is_file() {
                let result = builder.add_ignore(global_ignore_file);
                match result {
                    Some(ignore::Error::Partial(_)) => (),
                    Some(err) => {
                        print_error!("Malformed pattern in global ignore file. {}.", err);
                    }
                    None => (),
                }
            }
        }

        for ignore_file in &config.ignore_files {
            let result = builder.add_ignore(ignore_file);
            match result {
                Some(ignore::Error::Partial(_)) => (),
                Some(err) => {
                    print_error!("Malformed pattern in custom ignore file. {}.", err);
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
                thread::scope(|scope| {
                    // Each spawned job will store its thread handle in here.
                    let threads = config.threads;
                    let mut handles = Vec::with_capacity(threads);
                    for _ in 0..threads {
                        let rx = rx.clone();

                        // Spawn a job thread that will listen for and execute inputs.
                        let handle =
                            scope.spawn(|| exec::job(rx.into_iter().flatten(), cmd, config));

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
            if let Some(cmd) = &config.command
                && !cmd.in_batch_mode()
                && config.threads > 1
            {
                // Evenly distribute work between multiple receivers
                limit = 1;
            }
            let mut tx = BatchSender::new(tx.clone(), limit);

            Box::new(move |entry| {
                if quit_flag.load(Ordering::Relaxed) {
                    return WalkState::Quit;
                }

                if let Ok(e) = &entry {
                    // If the entry is a directory that contains a
                    // "ignore contain" file", we want to skip this
                    // directory.
                    // Check the filetype first to avoid unnecessary
                    // syscalls.
                    if e.file_type().is_some_and(|t| t.is_dir()) {
                        let entry_path = e.path();
                        if config
                            .ignore_contain
                            .iter()
                            .any(|ic| entry_path.join(ic).exists())
                        {
                            return WalkState::Skip;
                        }
                    }
                    if e.depth() == 0 {
                        // Skip the root directory entry.
                        return WalkState::Continue;
                    }
                }
                let entry = match entry {
                    Ok(e) => DirEntry::normal(e),
                    Err(ignore::Error::WithPath {
                        path,
                        err: inner_err,
                    }) if inner_err
                        .io_error()
                        .is_some_and(|io_error| io_error.kind() == io::ErrorKind::NotFound)
                        && path
                            .symlink_metadata()
                            .ok()
                            .is_some_and(|m| m.file_type().is_symlink()) =>
                    {
                        DirEntry::broken_symlink(path)
                    }
                    Err(err) => {
                        return match tx.send(WorkerResult::Error(err)) {
                            Ok(_) => WalkState::Continue,
                            Err(_) => WalkState::Quit,
                        };
                    }
                };

                if let Some(min_depth) = config.min_depth
                    && entry.depth().is_none_or(|d| d < min_depth)
                {
                    return WalkState::Continue;
                }

                // Check the name first, since it doesn't require metadata
                let entry_path = entry.path();

                let search_str = search_str_for_entry(entry_path, config.full_path_base.as_deref());

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
                if let Some(ref file_types) = config.file_types
                    && file_types.should_ignore(&entry)
                {
                    return WalkState::Continue;
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
                    if let Some(metadata) = entry.metadata()
                        && let Ok(modified) = metadata.modified()
                    {
                        matched = config
                            .time_constraints
                            .iter()
                            .all(|tf| tf.applies_to(&modified));
                    }
                    if !matched {
                        return WalkState::Continue;
                    }
                }

                if config.is_printing()
                    && let Some(ls_colors) = &config.ls_colors
                {
                    // Compute colors in parallel
                    entry.style(ls_colors);
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

fn search_str_for_entry<'a>(
    entry_path: &'a std::path::Path,
    full_path_base: Option<&std::path::Path>,
) -> Cow<'a, OsStr> {
    if let Some(cwd) = full_path_base {
        // If full_path_base is some, that means that we need to return
        // the absolute path
        if entry_path.is_absolute() {
            return Cow::Borrowed(entry_path.as_os_str());
        }
        let path = entry_path.strip_prefix(".").unwrap_or(entry_path);
        Cow::Owned(cwd.join(path).into())
    } else {
        match entry_path.file_name() {
            Some(filename) => Cow::Borrowed(filename),
            None => unreachable!(
                "Encountered file system entry without a file name. This should only \
                 happen for paths like 'foo/bar/..' or '/' which are not supposed to \
                 appear in a file system traversal."
            ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct RecordingWriter {
        bytes: Vec<u8>,
        flushes: usize,
    }

    impl Write for RecordingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    fn receiver_state(args: &[&str]) -> WorkerState {
        let opts = crate::cli::Opts::parse_from(
            ["fd", "--color=never"]
                .into_iter()
                .chain(args.iter().copied()),
        );
        WorkerState::new(vec![], crate::construct_config(opts, &[]).unwrap())
    }

    fn send_path(tx: &Sender<Batch>, path: &str) {
        let batch = Batch::new();
        batch
            .lock()
            .as_mut()
            .unwrap()
            .push(WorkerResult::Entry(DirEntry::broken_symlink(
                PathBuf::from(path),
            )));
        tx.send(batch).unwrap();
    }

    #[test]
    fn receiver_batches_across_empty_queue() {
        let state = receiver_state(&["--max-buffer-time=0"]);
        let (tx, rx) = bounded(1);
        let mut receiver = ReceiverBuffer::new(&state, rx, RecordingWriter::default());
        // Switch to streaming before any results arrive.
        receiver.poll().unwrap();
        let initial_flushes = receiver.stdout.flushes;
        for path in ["first", "second", "third"] {
            send_path(&tx, path);
            receiver.poll().unwrap();
            assert!(receiver.rx.is_empty());
        }
        assert_eq!(receiver.stdout.bytes, b"first\nsecond\nthird\n");
        assert_eq!(receiver.stdout.flushes - initial_flushes, 0);
        drop(tx);
        assert_eq!(receiver.poll(), Err(ExitCode::Success));
        assert_eq!(receiver.stdout.flushes - initial_flushes, 1);
    }

    #[test]
    fn receiver_flushes_pending_output_on_timeout() {
        let state = receiver_state(&["--max-buffer-time=0"]);
        let (tx, rx) = bounded(1);
        let mut receiver =
            ReceiverBuffer::new(&state, rx, io::BufWriter::new(RecordingWriter::default()));
        receiver.poll().unwrap();
        send_path(&tx, "first");
        receiver.poll().unwrap();
        let deadline = receiver.flush_deadline.unwrap();
        send_path(&tx, "second");
        receiver.poll().unwrap();
        assert_eq!(receiver.flush_deadline, Some(deadline));
        assert!(receiver.stdout.get_ref().bytes.is_empty());
        // Expire the deadline explicitly instead of depending on a sleep.
        receiver.flush_deadline = Some(Instant::now());
        receiver.poll().unwrap();
        assert_eq!(receiver.stdout.get_ref().bytes, b"first\nsecond\n");
        assert!(receiver.flush_deadline.is_none());
        // A later burst starts a fresh deadline.
        send_path(&tx, "third");
        receiver.poll().unwrap();
        assert!(receiver.flush_deadline.is_some());
        drop(tx);
        assert_eq!(receiver.poll(), Err(ExitCode::Success));
        assert_eq!(receiver.stdout.get_ref().bytes, b"first\nsecond\nthird\n");
    }

    #[test]
    fn receiver_flushes_at_result_limit() {
        let state = receiver_state(&["--max-buffer-time=0", "--max-results=1"]);
        let (tx, rx) = bounded(1);
        let mut receiver =
            ReceiverBuffer::new(&state, rx, io::BufWriter::new(RecordingWriter::default()));
        receiver.poll().unwrap();
        send_path(&tx, "only");
        assert_eq!(receiver.process(), ExitCode::Success);
        assert_eq!(receiver.stdout.get_ref().bytes, b"only\n");
        assert!(state.quit_flag.load(Ordering::Relaxed));
    }

    #[test]
    fn receiver_sorts_initial_buffer() {
        let state = receiver_state(&["--max-buffer-time=60000"]);
        let (tx, rx) = bounded(2);
        send_path(&tx, "z");
        send_path(&tx, "a");
        drop(tx);
        let mut receiver = ReceiverBuffer::new(&state, rx, RecordingWriter::default());
        assert_eq!(receiver.process(), ExitCode::Success);
        assert_eq!(receiver.stdout.bytes, b"a\nz\n");
        assert_eq!(receiver.stdout.flushes, 1);
    }

    #[test]
    fn receiver_quiet_does_not_print() {
        let state = receiver_state(&["--quiet", "--max-buffer-time=0"]);
        let (tx, rx) = bounded(1);
        let mut receiver = ReceiverBuffer::new(&state, rx, RecordingWriter::default());
        receiver.poll().unwrap();
        send_path(&tx, "match");
        assert_eq!(receiver.process(), ExitCode::HasResults(true));
        assert!(receiver.stdout.bytes.is_empty());
        assert!(receiver.flush_deadline.is_none());
    }

    #[test]
    fn receiver_flushes_on_interrupt() {
        let state = receiver_state(&["--max-buffer-time=0"]);
        let (tx, rx) = bounded(1);
        let mut receiver =
            ReceiverBuffer::new(&state, rx, io::BufWriter::new(RecordingWriter::default()));
        receiver.poll().unwrap();
        send_path(&tx, "last");
        state.interrupt_flag.store(true, Ordering::Relaxed);
        assert_eq!(receiver.process(), ExitCode::KilledBySigint);
        assert_eq!(receiver.stdout.get_ref().bytes, b"last\n");
        assert!(state.quit_flag.load(Ordering::Relaxed));
    }

    struct FailingWriter {
        fail_write: bool,
        kind: io::ErrorKind,
    }

    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail_write {
                Err(self.kind.into())
            } else {
                Ok(bytes.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(self.kind.into())
        }
    }

    #[test]
    fn receiver_reports_output_errors() {
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::Other] {
            for fail_write in [false, true] {
                let state = receiver_state(&[]);
                let (tx, rx) = bounded(1);
                send_path(&tx, "last");
                drop(tx);
                let writer = FailingWriter { fail_write, kind };
                let mut receiver = ReceiverBuffer::new(&state, rx, writer);
                receiver.mode = ReceiverMode::Streaming;
                assert_eq!(receiver.process(), ExitCode::GeneralError);
                assert!(state.quit_flag.load(Ordering::Relaxed));
            }
        }
    }

    #[test]
    fn search_str_for_entry_with_relative_path() {
        let full_path_base = Some(Path::new("/home/user"));
        assert_eq!(
            search_str_for_entry(Path::new("foo/bar"), full_path_base),
            PathBuf::from("/home/user/foo/bar")
        );
    }

    #[test]
    fn search_str_for_entry_strips_dot_prefix() {
        let full_path_base = Some(Path::new("/home/user"));
        assert_eq!(
            search_str_for_entry(Path::new("./foo/bar"), full_path_base),
            PathBuf::from("/home/user/foo/bar")
        );
    }

    #[test]
    fn search_str_for_entry_with_absolute_path() {
        let full_path_base = Some(Path::new("/home/user"));
        assert_eq!(
            search_str_for_entry(Path::new("/absolute/path"), full_path_base),
            PathBuf::from("/absolute/path")
        );
    }

    #[test]
    fn search_str_no_base_dir() {
        assert_eq!(
            search_str_for_entry(Path::new("./foo/bar"), None),
            PathBuf::from("bar")
        );
    }

    #[test]
    fn search_str_no_base_dir_with_plain_relative_path() {
        assert_eq!(
            search_str_for_entry(Path::new("foo/bar"), None),
            PathBuf::from("bar")
        );
    }

    #[test]
    fn search_str_no_base_dir_with_file_in_current_dir() {
        assert_eq!(
            search_str_for_entry(Path::new("foo"), None),
            PathBuf::from("foo")
        );
    }
}
