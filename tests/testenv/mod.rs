use std;
use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

#[cfg(unix)]
use std::os::unix;

#[cfg(windows)]
use std::os::windows;

extern crate diff;
extern crate tempdir;

use self::tempdir::TempDir;

/// Environment for the integration tests.
pub struct TestEnv {
    /// Temporary working directory.
    temp_dir: TempDir,

    /// Path to the *fd* executable.
    fd_exe: PathBuf,
}

/// Create the working directory and the test files.
fn create_working_directory() -> Result<TempDir, io::Error> {
    let temp_dir = TempDir::new("fd-tests")?;

    {
        let root = temp_dir.path();

        fs::create_dir_all(root.join("one/two/three"))?;

        fs::File::create(root.join("a.foo"))?;
        fs::File::create(root.join("one/b.foo"))?;
        fs::File::create(root.join("one/two/c.foo"))?;
        fs::File::create(root.join("one/two/C.Foo2"))?;
        fs::File::create(root.join("one/two/three/d.foo"))?;
        fs::create_dir(root.join("one/two/three/directory_foo"))?;
        fs::File::create(root.join("ignored.foo"))?;
        fs::File::create(root.join(".hidden.foo"))?;

        #[cfg(unix)] unix::fs::symlink(root.join("one/two"), root.join("symlink"))?;

        // Note: creating symlinks on Windows requires the `SeCreateSymbolicLinkPrivilege` which
        // is by default only granted for administrators.
        #[cfg(windows)] windows::fs::symlink_dir(root.join("one/two"), root.join("symlink"))?;

        fs::File::create(root.join(".ignore"))?.write_all(
            b"ignored.foo",
        )?;
    }

    Ok(temp_dir)
}

/// Find the *fd* executable.
fn find_fd_exe() -> PathBuf {
    // Tests exe is in target/debug/deps, the *fd* exe is in target/debug
    let root = env::current_exe()
        .expect("tests executable")
        .parent()
        .expect("tests executable directory")
        .parent()
        .expect("fd executable directory")
        .to_path_buf();

    let exe_name = if cfg!(windows) { "fd.exe" } else { "fd" };

    root.join(exe_name)
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
            diff::Result::Left(l) => format!("-{}", l),
            diff::Result::Both(l, _) => format!(" {}", l),
            diff::Result::Right(r) => format!("+{}", r),
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
fn normalize_output(s: &str, trim_left: bool) -> String {
    // Split into lines and normalize separators.
    let mut lines = s.replace('\0', "NULL\n")
        .lines()
        .map(|line| {
            let line = if trim_left { line.trim_left() } else { line };
            line.replace('/', &std::path::MAIN_SEPARATOR.to_string())
        })
        .collect::<Vec<_>>();

    // Sort ignoring case.
    lines.sort_by_key(|s| s.to_lowercase());

    lines.join("\n")
}

impl TestEnv {
    pub fn new() -> TestEnv {
        let temp_dir = create_working_directory().expect("working directory");
        let fd_exe = find_fd_exe();

        TestEnv {
            temp_dir: temp_dir,
            fd_exe: fd_exe,
        }
    }

    /// Get the root directory for the tests.
    pub fn root(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Assert that calling *fd* with the specified arguments produces the expected output.
    pub fn assert_output(&self, args: &[&str], expected: &str) {
        self.assert_output_subdirectory(".", args, expected)
    }

    /// Assert that calling *fd* in the specified path under the root working directory,
    /// and with the specified arguments produces the expected output.
    pub fn assert_output_subdirectory<P: AsRef<Path>>(
        &self,
        path: P,
        args: &[&str],
        expected: &str,
    ) {
        // Setup *fd* command.
        let mut cmd = process::Command::new(&self.fd_exe);
        cmd.current_dir(self.temp_dir.path().join(path));
        cmd.args(args);

        // Run *fd*.
        let output = cmd.output().expect("fd output");

        // Check for exit status.
        if !output.status.success() {
            panic!(format_exit_error(args, &output));
        }

        // Normalize both expected and actual output.
        let expected = normalize_output(expected, true);
        let actual = normalize_output(&String::from_utf8_lossy(&output.stdout), false);

        // Compare actual output to expected output.
        if expected != actual {
            panic!(format_output_error(args, &expected, &actual));
        }
    }
}
