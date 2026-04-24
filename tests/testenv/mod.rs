use std::env;
use std::fs;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix;
#[cfg(windows)]
use std::os::windows;
use std::path::{Path, PathBuf};
use std::process;

use tempfile::TempDir;

/// Environment for the integration tests.
pub struct TestEnv {
    /// Temporary working directory.
    temp_dir: TempDir,

    /// Path to the *fd* executable.
    fd_exe: PathBuf,

    /// Normalize each line by sorting the whitespace-separated words
    normalize_line: bool,

    /// Temporary directory for storing test config (global ignore file)
    config_dir: Option<TempDir>,
}

/// Create the working directory and the test files.
fn create_working_directory(
    directories: &[&'static str],
    files: &[&'static str],
) -> Result<TempDir, io::Error> {
    let temp_dir = tempfile::Builder::new().prefix("fd-tests").tempdir()?;

    {
        let root = temp_dir.path();

        // Pretend that this is a Git repository in order for `.gitignore` files to be respected
        fs::create_dir_all(root.join(".git"))?;

        for directory in directories {
            fs::create_dir_all(root.join(directory))?;
        }

        for file in files {
            fs::File::create(root.join(file))?;
        }

        #[cfg(unix)]
        unix::fs::symlink(root.join("one/two"), root.join("symlink"))?;

        // Note: creating symlinks on Windows requires the `SeCreateSymbolicLinkPrivilege` which
        // is by default only granted for administrators.
        #[cfg(windows)]
        windows::fs::symlink_dir(root.join("one/two"), root.join("symlink"))?;

        fs::File::create(root.join(".fdignore"))?.write_all(b"fdignored.foo")?;

        fs::File::create(root.join(".gitignore"))?.write_all(b"gitignored.foo")?;
    }

    Ok(temp_dir)
}

fn create_config_directory_with_global_ignore(ignore_file_content: &str) -> io::Result<TempDir> {
    let config_dir = tempfile::Builder::new().prefix("fd-config").tempdir()?;
    let fd_dir = config_dir.path().join("fd");
    fs::create_dir(&fd_dir)?;
    let mut ignore_file = fs::File::create(fd_dir.join("ignore"))?;
    ignore_file.write_all(ignore_file_content.as_bytes())?;

    Ok(config_dir)
}

/// Find the *fd* executable.
fn find_fd_exe() -> PathBuf {
    // Read the location of the fd executable from the environment
    PathBuf::from(env::var("CARGO_BIN_EXE_fd").unwrap_or(env!("CARGO_BIN_EXE_fd").to_string()))
}

/// Format an error message for when *fd* did not exit successfully.
fn format_exit_error(args: &[&str], output: &process::Output) -> String {
    format!(
        "`fd {}` did not exit successfully.\nstdout:\n---\n{}---\nstderr:\n---\n{}---",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Format an error message for when the output of *fd* did not match the expected output.
fn format_output_error(args: &[&str], expected: &str, actual: &str) -> String {
    // Generate diff text.
    let diff_text = diff::lines(expected, actual)
        .into_iter()
        .map(|diff| match diff {
            diff::Result::Left(l) => format!("-{l}"),
            diff::Result::Both(l, _) => format!(" {l}"),
            diff::Result::Right(r) => format!("+{r}"),
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        concat!(
            "`fd {}` did not produce the expected output.\n",
            "Showing diff between expected and actual:\n{}\n"
        ),
        args.join(" "),
        diff_text
    )
}

/// Normalize the output for comparison.
fn normalize_output(s: &str, trim_start: bool, normalize_line: bool) -> String {
    // Split into lines and normalize separators.
    let mut lines = s
        .replace('\0', "NULL\n")
        .lines()
        .map(|line| {
            let line = if trim_start { line.trim_start() } else { line };
            let line = line.replace('/', std::path::MAIN_SEPARATOR_STR);
            if normalize_line {
                let mut words: Vec<_> = line.split_whitespace().collect();
                words.sort_unstable();
                return words.join(" ");
            }
            line
        })
        .collect::<Vec<_>>();

    lines.sort();
    lines.join("\n")
}

/// Trim whitespace from the beginning of each line.
fn trim_lines(s: &str) -> String {
    s.lines()
        .map(|line| line.trim_start())
        .fold(String::new(), |mut str, line| {
            str.push_str(line);
            str.push('\n');
            str
        })
}

impl TestEnv {
    pub fn new(directories: &[&'static str], files: &[&'static str]) -> TestEnv {
        let temp_dir = create_working_directory(directories, files).expect("working directory");
        let fd_exe = find_fd_exe();

        TestEnv {
            temp_dir,
            fd_exe,
            normalize_line: false,
            config_dir: None,
        }
    }

    pub fn normalize_line(self, normalize: bool) -> TestEnv {
        TestEnv {
            temp_dir: self.temp_dir,
            fd_exe: self.fd_exe,
            normalize_line: normalize,
            config_dir: self.config_dir,
        }
    }

    pub fn global_ignore_file(self, content: &str) -> TestEnv {
        let config_dir =
            create_config_directory_with_global_ignore(content).expect("config directory");
        TestEnv {
            config_dir: Some(config_dir),
            ..self
        }
    }

    /// Returns the canonical path for files or folders in the temporary directory.
    /// Normally, `TestEnv` creates a temporary directory under `/var` for tests. However, this
    /// doesn't work well with `--exclude-absolute-path`, as it filters entries using their
    /// absolute paths, and the canonical path of `/var` is actually `/private/var` on unix.
    pub fn get_canonical_path_in_temp_dir(&self, path: &str) -> String {
        let joined = self.temp_dir.path().join(path);

        #[cfg(unix)]
        {
            if let Ok(canonical) = std::fs::canonicalize(&joined) {
                return canonical.to_string_lossy().into_owned();
            }
        }

        joined.to_string_lossy().into_owned()
    }

    /// Create a broken symlink at the given path in the temp_dir.
    pub fn create_broken_symlink<P: AsRef<Path>>(
        &mut self,
        link_path: P,
    ) -> Result<PathBuf, io::Error> {
        let root = self.test_root();
        let broken_symlink_link = root.join(link_path);
        {
            let temp_target_dir = tempfile::Builder::new()
                .prefix("fd-tests-broken-symlink")
                .tempdir()?;
            let broken_symlink_target = temp_target_dir.path().join("broken_symlink_target");
            fs::File::create(&broken_symlink_target)?;
            #[cfg(unix)]
            unix::fs::symlink(&broken_symlink_target, &broken_symlink_link)?;
            #[cfg(windows)]
            windows::fs::symlink_file(&broken_symlink_target, &broken_symlink_link)?;
        }
        Ok(broken_symlink_link)
    }

    /// Get the root directory for the tests.
    pub fn test_root(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Get the path of the fd executable.
    #[cfg_attr(windows, allow(unused))]
    pub fn test_exe(&self) -> &PathBuf {
        &self.fd_exe
    }

    /// Get the root directory of the file system.
    pub fn system_root(&self) -> PathBuf {
        let mut components = self.temp_dir.path().components();
        PathBuf::from(components.next().expect("root directory").as_os_str())
    }

    /// Assert that calling *fd* in the specified path under the root working directory,
    /// and with the specified arguments produces the expected output.
    pub fn assert_success_and_get_output<P: AsRef<Path>>(
        &self,
        path: P,
        args: &[&str],
    ) -> process::Output {
        // Run *fd*.
        let output = self.run_command(path.as_ref(), args);

        // Check for exit status.
        if !output.status.success() {
            panic!("{}", format_exit_error(args, &output));
        }

        output
    }

    pub fn assert_success_and_get_normalized_output<P: AsRef<Path>>(
        &self,
        path: P,
        args: &[&str],
    ) -> String {
        let output = self.assert_success_and_get_output(path, args);
        normalize_output(
            &String::from_utf8_lossy(&output.stdout),
            false,
            self.normalize_line,
        )
    }

    /// Assert that calling *fd* with the specified arguments produces the expected output.
    pub fn assert_output(&self, args: &[&str], expected: &str) {
        self.assert_output_subdirectory(".", args, expected)
    }

    /// Similar to assert_output, but able to handle non-utf8 output
    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn assert_output_raw(&self, args: &[&str], expected: &[u8]) {
        let output = self.assert_success_and_get_output(".", args);

        assert_eq!(expected, &output.stdout[..]);
    }

    /// Assert that calling *fd* in the specified path under the root working directory,
    /// and with the specified arguments produces the expected output.
    pub fn assert_output_subdirectory<P: AsRef<Path>>(
        &self,
        path: P,
        args: &[&str],
        expected: &str,
    ) {
        // Normalize both expected and actual output.
        let expected = normalize_output(expected, true, self.normalize_line);
        let actual = self.assert_success_and_get_normalized_output(path, args);

        // Compare actual output to expected output.
        if expected != actual {
            panic!("{}", format_output_error(args, &expected, &actual));
        }
    }

    /// Assert that calling *fd* with the specified arguments produces the expected error,
    /// and does not succeed.
    pub fn assert_failure_with_error(&self, args: &[&str], expected: &str) {
        let status = self.assert_error_subdirectory(".", args, Some(expected));
        if status.success() {
            panic!("error '{expected}' did not occur.");
        }
    }

    /// Assert that calling *fd* with the specified arguments does not succeed.
    pub fn assert_failure(&self, args: &[&str]) {
        let status = self.assert_error_subdirectory(".", args, None);
        if status.success() {
            panic!("Failure did not occur as expected.");
        }
    }

    /// Assert that calling *fd* with the specified arguments produces the expected error.
    pub fn assert_error(&self, args: &[&str], expected: &str) -> process::ExitStatus {
        self.assert_error_subdirectory(".", args, Some(expected))
    }

    fn run_command(&self, path: &Path, args: &[&str]) -> process::Output {
        // Setup *fd* command.
        let mut cmd = process::Command::new(&self.fd_exe);
        cmd.current_dir(self.temp_dir.path().join(path));
        if let Some(config_dir) = &self.config_dir {
            cmd.env("XDG_CONFIG_HOME", config_dir.path());
        } else {
            cmd.arg("--no-global-ignore-file");
        }
        // Make sure LS_COLORS is unset to ensure consistent
        // color output
        cmd.env("LS_COLORS", "");
        cmd.args(args);

        // Run *fd*.
        cmd.output().expect("fd output")
    }

    /// Assert that calling *fd* in the specified path under the root working directory,
    /// and with the specified arguments produces an error with the expected message.
    fn assert_error_subdirectory<P: AsRef<Path>>(
        &self,
        path: P,
        args: &[&str],
        expected: Option<&str>,
    ) -> process::ExitStatus {
        let output = self.run_command(path.as_ref(), args);

        if let Some(expected) = expected {
            // Normalize both expected and actual output.
            let expected_error = trim_lines(expected);
            let actual_err = trim_lines(&String::from_utf8_lossy(&output.stderr));

            // Compare actual output to expected output.
            if !actual_err.trim_start().starts_with(&expected_error) {
                panic!(
                    "{}",
                    format_output_error(args, &expected_error, &actual_err)
                );
            }
        }

        output.status
    }
}
