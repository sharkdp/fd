//! Integration tests for the CLI interface of fd.

#![allow(dead_code, unused_imports)]

use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use std::os::unix;

#[cfg(windows)]
use std::os::windows;

extern crate tempdir;

use tempdir::TempDir;

struct TestEnv {
    temp_dir: TempDir,
    fd: PathBuf,
}

impl TestEnv {
    pub fn new() -> TestEnv {
        let temp_dir = TestEnv::create_working_directory()
            .expect("working directory");
        let fd = TestEnv::find_fd();

        TestEnv {
            temp_dir: temp_dir,
            fd: fd,
        }
    }

    pub fn root(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    pub fn assert_output(&self, args: &[&str], expected: &str) {
        let expected = TestEnv::normalize_output(expected, true);

        let mut cmd = self.command();
        for arg in args {
            cmd.arg(arg);
        }
        let output = cmd.output().expect("executing fd");

        if !output.status.success() {
            // TODO: Add arguments to error message.
            panic!("fd returned non-zero status");
        }

        let actual = TestEnv::normalize_output(&String::from_utf8_lossy(&output.stdout), false);

        // Compare actual output to expected output.
        // TODO: Show diff if not equal.
        assert_eq!(expected, actual);
    }

    fn normalize_output(s: &str, trim_left: bool) -> String {
        // Split into lines and normalize separators.
        let mut lines = s
            .replace('\0', "NULL\n")
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

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.fd);
        cmd.current_dir(self.temp_dir.path());
        cmd
    }

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

            #[cfg(unix)]
            unix::fs::symlink(root.join("one/two"), root.join("symlink"))?;

            #[cfg(windows)]
            windows::fs::symlink_file(root.join("one/two"), root.join("symlink"))?;

            fs::File::create(root.join(".ignore"))?.write_all(b"ignored.foo")?;
        }

        Ok(temp_dir)
    }

    fn find_fd() -> PathBuf {
        // Tests exe is in target/debug/deps, the fd exe is in target/debug
        let root = env::current_exe().expect("tests executable")
            .parent().expect("tests executable directory")
            .parent().expect("fd executable directory")
            .to_path_buf();

        let exe_name = if cfg!(windows) { "fd.exe" } else { "fd" };

        let path = root.join(exe_name);
        if !path.is_file() {
            root.join("..").join(exe_name)
        } else {
            path
        }
    }
}

mod tests {
    use TestEnv;

    /// Simple tests
    #[test]
    fn test_simple() {
        let te = TestEnv::new();

        te.assert_output(&["a.foo"], "a.foo");
        te.assert_output(&["b.foo"], "one/b.foo");
        te.assert_output(&["d.foo"], "one/two/three/d.foo");

        te.assert_output(
            &["foo"],
            "a.foo
            one/b.foo
            one/two/c.foo
            one/two/C.Foo2
            one/two/three/d.foo
            one/two/three/directory_foo");

        te.assert_output(
            &[],
            "a.foo
            one
            one/b.foo
            one/two
            one/two/c.foo
            one/two/C.Foo2
            one/two/three
            one/two/three/d.foo
            one/two/three/directory_foo
            symlink");
    }

    /// Explicit root path
    // TODO: Fails on windows
    #[cfg_attr(windows, ignore)]
    #[test]
    fn test_explicit_root_path() {
        let te = TestEnv::new();

        te.assert_output(
            &["foo", "one"],
            "one/b.foo
            one/two/c.foo
            one/two/C.Foo2
            one/two/three/d.foo
            one/two/three/directory_foo");

        te.assert_output(
            &["foo", "one/two/three"],
            "one/two/three/d.foo
            one/two/three/directory_foo");
    }

    /// Regex searches
    #[test]
    fn test_regex_searches() {
        let te = TestEnv::new();

        te.assert_output(
            &["[a-c].foo"],
            "a.foo
            one/b.foo
            one/two/c.foo
            one/two/C.Foo2");

        te.assert_output(
            &["--case-sensitive", "[a-c].foo"],
            "a.foo
            one/b.foo
            one/two/c.foo");
    }

    /// Smart case
    #[test]
    fn test_smart_case() {
        let te = TestEnv::new();

        te.assert_output(
            &["c.foo"],
            "one/two/c.foo
            one/two/C.Foo2");

        te.assert_output(
            &["C.Foo"],
            "one/two/C.Foo2");

        te.assert_output(
            &["Foo"],
            "one/two/C.Foo2");

        te.assert_output(
            &["--case-sensitive", "[a-c].foo"],
            "a.foo
            one/b.foo
            one/two/c.foo");
    }

    /// Case sensitivity (--case-sensitive)
    #[test]
    fn test_case_sensitive() {
        let te = TestEnv::new();

        te.assert_output(
            &["--case-sensitive", "c.foo"],
            "one/two/c.foo");

        te.assert_output(
            &["--case-sensitive", "C.Foo"],
            "one/two/C.Foo2");
    }

    /// Full path search (--full-path)
    #[test]
    fn test_full_path() {
        let te = TestEnv::new();

        te.assert_output(
            &["--full-path", "three.*foo"],
            "one/two/three/d.foo
            one/two/three/directory_foo");

        te.assert_output(
            &["--full-path", "^a\\.foo"],
            "a.foo");
    }

    /// Hidden files (--hidden)
    #[test]
    fn test_hidden() {
        let te = TestEnv::new();

        te.assert_output(
            &["--hidden", "foo"],
            ".hidden.foo
            a.foo
            one/b.foo
            one/two/c.foo
            one/two/C.Foo2
            one/two/three/d.foo
            one/two/three/directory_foo");
    }

    /// Ignored files (--no-ignore)
    #[test]
    fn test_no_ignore() {
        let te = TestEnv::new();

        te.assert_output(
            &["--no-ignore", "foo"],
            "a.foo
            ignored.foo
            one/b.foo
            one/two/c.foo
            one/two/C.Foo2
            one/two/three/d.foo
            one/two/three/directory_foo");

        te.assert_output(
            &["--hidden", "--no-ignore", "foo"],
            ".hidden.foo
            a.foo
            ignored.foo
            one/b.foo
            one/two/c.foo
            one/two/C.Foo2
            one/two/three/d.foo
            one/two/three/directory_foo");
    }

    /// Symlinks (--follow)
    // TODO: Fails on windows
    #[cfg_attr(windows, ignore)]
    #[test]
    fn test_follow() {
        let te = TestEnv::new();

        te.assert_output(
            &["--follow", "c.foo"],
            "one/two/c.foo
            one/two/C.Foo2
            symlink/c.foo
            symlink/C.Foo2");
    }

    /// Null separator (--print0)
    #[test]
    fn test_print0() {
        let te = TestEnv::new();

        te.assert_output(
            &["--print0", "foo"],
            "a.fooNULL
            one/b.fooNULL
            one/two/C.Foo2NULL
            one/two/c.fooNULL
            one/two/three/d.fooNULL
            one/two/three/directory_fooNULL");
    }

    /// Maximum depth (--max-depth)
    #[test]
    fn test_max_depth() {
        let te = TestEnv::new();

        te.assert_output(
            &["--max-depth", "3"],
            "a.foo
            one
            one/b.foo
            one/two
            one/two/c.foo
            one/two/C.Foo2
            one/two/three
            symlink");

        te.assert_output(
            &["--max-depth", "2"],
            "a.foo
            one
            one/b.foo
            one/two
            symlink");

        te.assert_output(
            &["--max-depth", "1"],
            "a.foo
            one
            symlink");
    }

    /// Absolute paths (--absolute-path)
    // TODO: fails on windows
    #[cfg_attr(windows, ignore)]
    #[test]
    fn test_absolute_path() {
        let te = TestEnv::new();

        let abs_path = te.root()
            .canonicalize().expect("absolute path")
            .to_str().expect("string")
            .to_string();

        te.assert_output(
            &["--absolute-path", "foo"],
            &format!(
                "{abs_path}/a.foo
                {abs_path}/one/b.foo
                {abs_path}/one/two/c.foo
                {abs_path}/one/two/C.Foo2
                {abs_path}/one/two/three/d.foo
                {abs_path}/one/two/three/directory_foo",
                abs_path=abs_path
            )
        );

        te.assert_output(
            &["foo", &abs_path],
            &format!(
                "{abs_path}/a.foo
                {abs_path}/one/b.foo
                {abs_path}/one/two/c.foo
                {abs_path}/one/two/C.Foo2
                {abs_path}/one/two/three/d.foo
                {abs_path}/one/two/three/directory_foo",
                abs_path=abs_path
            )
        );
    }

    /// File type filter (--type)
    #[test]
    fn test_type() {
        let te = TestEnv::new();

        te.assert_output(
            &["--type", "f"],
            "a.foo
            one/b.foo
            one/two/c.foo
            one/two/C.Foo2
            one/two/three/d.foo");

        te.assert_output(
            &["--type", "d"],
            "one
            one/two
            one/two/three
            one/two/three/directory_foo");

        te.assert_output(
            &["--type", "s"],
            "symlink");
    }

    /// File extension (--extension)
    #[test]
    fn test_extension() {
        let te = TestEnv::new();

        te.assert_output(
            &["--extension", "foo"],
            "a.foo
            one/b.foo
            one/two/c.foo
            one/two/three/d.foo");

        te.assert_output(
            &["--extension", ".foo"],
            "a.foo
            one/b.foo
            one/two/c.foo
            one/two/three/d.foo");

        te.assert_output(
            &["--extension", "foo2"],
            "one/two/C.Foo2");
    }
}
