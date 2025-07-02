mod testenv;

#[cfg(unix)]
use nix::unistd::Uid;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
use test_case::test_case;

use jiff::Timestamp;
use regex::escape;

use crate::testenv::{
    create_file_with_size, get_absolute_root_path, get_test_env_with_abs_path, TestEnv,
    DEFAULT_DIRS, DEFAULT_FILES,
};

/// Simple test
#[test]
fn test_simple() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

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
        one/two/three/directory_foo/",
    );
}

/// Test each pattern type with an empty pattern.
#[test]
fn test_empty_pattern() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    let expected = "a.foo
    e1 e2
    one/
    one/b.foo
    one/two/
    one/two/c.foo
    one/two/C.Foo2
    one/two/three/
    one/two/three/d.foo
    one/two/three/directory_foo/
    symlink";

    te.assert_output(&["--regex"], expected);
    te.assert_output(&["--fixed-strings"], expected);
    te.assert_output(&["--glob"], expected);
}

/// Test multiple directory searches
#[test]
fn test_multi_file() {
    let dirs = &["test1", "test2"];
    let files = &["test1/a.foo", "test1/b.foo", "test2/a.foo"];
    let te = TestEnv::new(dirs, files);
    te.assert_output(
        &["a.foo", "test1", "test2"],
        "test1/a.foo
        test2/a.foo",
    );

    te.assert_output(
        &["", "test1", "test2"],
        "test1/a.foo
        test2/a.foo
        test1/b.foo",
    );

    te.assert_output(&["a.foo", "test1"], "test1/a.foo");

    te.assert_output(&["b.foo", "test1", "test2"], "test1/b.foo");
}

/// Test search over multiple directory with missing
#[test]
fn test_multi_file_with_missing() {
    let dirs = &["real"];
    let files = &["real/a.foo", "real/b.foo"];
    let te = TestEnv::new(dirs, files);
    te.assert_output(&["a.foo", "real", "fake"], "real/a.foo");

    te.assert_error(
        &["a.foo", "real", "fake"],
        "[fd error]: Search path 'fake' is not a directory.",
    );

    te.assert_output(
        &["", "real", "fake"],
        "real/a.foo
        real/b.foo",
    );

    te.assert_output(
        &["", "real", "fake1", "fake2"],
        "real/a.foo
        real/b.foo",
    );

    te.assert_error(
        &["", "real", "fake1", "fake2"],
        "[fd error]: Search path 'fake1' is not a directory.
        [fd error]: Search path 'fake2' is not a directory.",
    );

    te.assert_failure_with_error(
        &["", "fake1", "fake2"],
        "[fd error]: Search path 'fake1' is not a directory.
        [fd error]: Search path 'fake2' is not a directory.
        [fd error]: No valid search paths given.",
    );
}

/// Explicit root path
#[test]
fn test_explicit_root_path() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["foo", "one"],
        "one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );

    te.assert_output(
        &["foo", "one/two/three"],
        "one/two/three/d.foo
        one/two/three/directory_foo/",
    );

    te.assert_output_subdirectory(
        "one/two/",
        &["foo", "../../"],
        "../../a.foo
        ../../one/b.foo
        ../../one/two/c.foo
        ../../one/two/C.Foo2
        ../../one/two/three/d.foo
        ../../one/two/three/directory_foo/",
    );

    te.assert_output_subdirectory(
        "one/two/three",
        &["", ".."],
        "../c.foo
        ../C.Foo2
        ../three/
        ../three/d.foo
        ../three/directory_foo/",
    );
}

/// Regex searches
#[test]
fn test_regex_searches() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["[a-c].foo"],
        "a.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2",
    );

    te.assert_output(
        &["--case-sensitive", "[a-c].foo"],
        "a.foo
        one/b.foo
        one/two/c.foo",
    );
}

/// Smart case
#[test]
fn test_smart_case() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["c.foo"],
        "one/two/c.foo
        one/two/C.Foo2",
    );

    te.assert_output(&["C.Foo"], "one/two/C.Foo2");

    te.assert_output(&["Foo"], "one/two/C.Foo2");

    // Only literal uppercase chars should trigger case sensitivity.
    te.assert_output(
        &["\\Ac"],
        "one/two/c.foo
        one/two/C.Foo2",
    );
    te.assert_output(&["\\AC"], "one/two/C.Foo2");
}

/// Case sensitivity (--case-sensitive)
#[test]
fn test_case_sensitive() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(&["--case-sensitive", "c.foo"], "one/two/c.foo");

    te.assert_output(&["--case-sensitive", "C.Foo"], "one/two/C.Foo2");

    te.assert_output(
        &["--ignore-case", "--case-sensitive", "C.Foo"],
        "one/two/C.Foo2",
    );
}

/// Case insensitivity (--ignore-case)
#[test]
fn test_case_insensitive() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--ignore-case", "C.Foo"],
        "one/two/c.foo
        one/two/C.Foo2",
    );

    te.assert_output(
        &["--case-sensitive", "--ignore-case", "C.Foo"],
        "one/two/c.foo
        one/two/C.Foo2",
    );
}

/// Full path search (--full-path)
#[test]
fn test_full_path() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    let root = te.system_root();
    let prefix = escape(&root.to_string_lossy());

    te.assert_output(
        &["--full-path", &format!("^{prefix}.*three.*foo$")],
        "one/two/three/d.foo
        one/two/three/directory_foo/",
    );
}

/// Hidden files (--hidden)
#[test]
fn test_hidden() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--hidden", "foo"],
        ".hidden.foo
        a.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );
}

/// Hidden file attribute on Windows
#[cfg(windows)]
#[test]
fn test_hidden_file_attribute() {
    use std::os::windows::fs::OpenOptionsExt;

    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileattributesa
    const FILE_ATTRIBUTE_HIDDEN: u32 = 2;

    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .attributes(FILE_ATTRIBUTE_HIDDEN)
        .open(te.test_root().join("hidden-file.txt"))
        .unwrap();

    te.assert_output(&["--hidden", "hidden-file.txt"], "hidden-file.txt");
    te.assert_output(&["hidden-file.txt"], "");
}

/// Symlinks (--follow)
#[test]
fn test_follow() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--follow", "c.foo"],
        "one/two/c.foo
        one/two/C.Foo2
        symlink/c.foo
        symlink/C.Foo2",
    );
}

// File system boundaries (--one-file-system)
// Limited to Unix because, to the best of my knowledge, there is no easy way to test a use case
// file systems mounted into the tree on Windows.
// Not limiting depth causes massive delay under Darwin, see BurntSushi/ripgrep#1429
#[test]
#[cfg(unix)]
fn test_file_system_boundaries() {
    // Helper function to get the device ID for a given path
    // Inspired by https://github.com/BurntSushi/ripgrep/blob/8892bf648cfec111e6e7ddd9f30e932b0371db68/ignore/src/walk.rs#L1693
    fn device_num(path: impl AsRef<Path>) -> u64 {
        use std::os::unix::fs::MetadataExt;

        path.as_ref().metadata().map(|md| md.dev()).unwrap()
    }

    // Can't simulate file system boundaries
    let te = TestEnv::new(&[], &[]);

    let dev_null = Path::new("/dev/null");

    // /dev/null should exist in all sane Unixes. Skip if it doesn't exist for some reason.
    // Also skip should it be on the same device as the root partition for some reason.
    if !dev_null.is_file() || device_num(dev_null) == device_num("/") {
        return;
    }

    te.assert_output(
        &["--full-path", "--max-depth", "2", "^/dev/null$", "/"],
        "/dev/null",
    );
    te.assert_output(
        &[
            "--one-file-system",
            "--full-path",
            "--max-depth",
            "2",
            "^/dev/null$",
            "/",
        ],
        "",
    );
}

#[test]
fn test_follow_broken_symlink() {
    let mut te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    te.create_broken_symlink("broken_symlink")
        .expect("Failed to create broken symlink.");

    te.assert_output(
        &["symlink"],
        "broken_symlink
        symlink",
    );
    te.assert_output(
        &["--type", "symlink", "symlink"],
        "broken_symlink
        symlink",
    );

    te.assert_output(&["--type", "file", "symlink"], "");

    te.assert_output(
        &["--follow", "--type", "symlink", "symlink"],
        "broken_symlink",
    );
    te.assert_output(&["--follow", "--type", "file", "symlink"], "");
}

/// Null separator (--print0)
#[test]
fn test_print0() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--print0", "foo"],
        "./a.fooNULL
        ./one/b.fooNULL
        ./one/two/C.Foo2NULL
        ./one/two/c.fooNULL
        ./one/two/three/d.fooNULL
        ./one/two/three/directory_foo/NULL",
    );
}

/// Maximum depth (--max-depth)
#[test]
fn test_max_depth() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--max-depth", "3"],
        "a.foo
        e1 e2
        one/
        one/b.foo
        one/two/
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/
        symlink",
    );

    te.assert_output(
        &["--max-depth", "2"],
        "a.foo
        e1 e2
        one/
        one/b.foo
        one/two/
        symlink",
    );

    te.assert_output(
        &["--max-depth", "1"],
        "a.foo
        e1 e2
        one/
        symlink",
    );
}

/// Minimum depth (--min-depth)
#[test]
fn test_min_depth() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--min-depth", "3"],
        "one/two/c.foo
        one/two/C.Foo2
        one/two/three/
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );

    te.assert_output(
        &["--min-depth", "4"],
        "one/two/three/d.foo
        one/two/three/directory_foo/",
    );
}

/// Exact depth (--exact-depth)
#[test]
fn test_exact_depth() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--exact-depth", "3"],
        "one/two/c.foo
        one/two/C.Foo2
        one/two/three/",
    );
}

/// Pruning (--prune)
#[test]
fn test_prune() {
    let dirs = &["foo/bar", "bar/foo", "baz"];
    let files = &[
        "foo/foo.file",
        "foo/bar/foo.file",
        "bar/foo.file",
        "bar/foo/foo.file",
        "baz/foo.file",
    ];

    let te = TestEnv::new(dirs, files);

    te.assert_output(
        &["foo"],
        "foo/
        foo/foo.file
        foo/bar/foo.file
        bar/foo.file
        bar/foo/
        bar/foo/foo.file
        baz/foo.file",
    );

    te.assert_output(
        &["--prune", "foo"],
        "foo/
        bar/foo/
        bar/foo.file
        baz/foo.file",
    );
}

/// Absolute paths (--absolute-path)
#[test]
fn test_absolute_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--absolute-path"],
        &format!(
            "{abs_path}/a.foo
            {abs_path}/e1 e2
            {abs_path}/one/
            {abs_path}/one/b.foo
            {abs_path}/one/two/
            {abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three/
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo/
            {abs_path}/symlink",
            abs_path = &abs_path
        ),
    );

    te.assert_output(
        &["--absolute-path", "foo"],
        &format!(
            "{abs_path}/a.foo
            {abs_path}/one/b.foo
            {abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo/",
            abs_path = &abs_path
        ),
    );
}

/// Show absolute paths if the path argument is absolute
#[test]
fn test_implicit_absolute_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["foo", &abs_path],
        &format!(
            "{abs_path}/a.foo
            {abs_path}/one/b.foo
            {abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo/",
            abs_path = &abs_path
        ),
    );
}

/// Absolute paths should be normalized
#[test]
fn test_normalized_absolute_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output_subdirectory(
        "one",
        &["--absolute-path", "foo", ".."],
        &format!(
            "{abs_path}/a.foo
            {abs_path}/one/b.foo
            {abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo/",
            abs_path = &abs_path
        ),
    );
}

/// File type filter (--type)
#[test]
fn test_type() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--type", "f"],
        "a.foo
        e1 e2
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo",
    );

    te.assert_output(&["--type", "f", "e1"], "e1 e2");

    te.assert_output(
        &["--type", "d"],
        "one/
        one/two/
        one/two/three/
        one/two/three/directory_foo/",
    );

    te.assert_output(
        &["--type", "d", "--type", "l"],
        "one/
        one/two/
        one/two/three/
        one/two/three/directory_foo/
        symlink",
    );

    te.assert_output(&["--type", "l"], "symlink");
}

/// Test `--type executable`
#[cfg(unix)]
#[test]
fn test_type_executable() {
    use std::os::unix::fs::OpenOptionsExt;

    // This test assumes the current user isn't root
    // (otherwise if the executable bit is set for any level, it is executable for the current
    // user)
    if Uid::current().is_root() {
        return;
    }

    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    fs::OpenOptions::new()
        .create_new(true)
        .truncate(true)
        .write(true)
        .mode(0o777)
        .open(te.test_root().join("executable-file.sh"))
        .unwrap();

    fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o645)
        .open(te.test_root().join("not-user-executable-file.sh"))
        .unwrap();

    te.assert_output(&["--type", "executable"], "executable-file.sh");

    te.assert_output(
        &["--type", "executable", "--type", "directory"],
        "executable-file.sh
        one/
        one/two/
        one/two/three/
        one/two/three/directory_foo/",
    );
}

/// Test `--type empty`
#[test]
fn test_type_empty() {
    let te = TestEnv::new(&["dir_empty", "dir_nonempty"], &[]);

    create_file_with_size(te.test_root().join("0_bytes.foo"), 0);
    create_file_with_size(te.test_root().join("5_bytes.foo"), 5);

    create_file_with_size(te.test_root().join("dir_nonempty").join("2_bytes.foo"), 2);

    te.assert_output(
        &["--type", "empty"],
        "0_bytes.foo
        dir_empty/",
    );

    te.assert_output(
        &["--type", "empty", "--type", "file", "--type", "directory"],
        "0_bytes.foo
        dir_empty/",
    );

    te.assert_output(&["--type", "empty", "--type", "file"], "0_bytes.foo");

    te.assert_output(&["--type", "empty", "--type", "directory"], "dir_empty/");
}

/// File extension (--extension)
#[test]
fn test_extension() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--extension", "foo"],
        "a.foo
        one/b.foo
        one/two/c.foo
        one/two/three/d.foo",
    );

    te.assert_output(
        &["--extension", ".foo"],
        "a.foo
        one/b.foo
        one/two/c.foo
        one/two/three/d.foo",
    );

    te.assert_output(
        &["--extension", ".foo", "--extension", "foo2"],
        "a.foo
        one/b.foo
        one/two/c.foo
        one/two/three/d.foo
        one/two/C.Foo2",
    );

    te.assert_output(&["--extension", ".foo", "a"], "a.foo");

    te.assert_output(&["--extension", "foo2"], "one/two/C.Foo2");

    let te2 = TestEnv::new(&[], &["spam.bar.baz", "egg.bar.baz", "yolk.bar.baz.sig"]);

    te2.assert_output(
        &["--extension", ".bar.baz"],
        "spam.bar.baz
        egg.bar.baz",
    );

    te2.assert_output(&["--extension", "sig"], "yolk.bar.baz.sig");

    te2.assert_output(&["--extension", "bar.baz.sig"], "yolk.bar.baz.sig");

    let te3 = TestEnv::new(&[], &["latin1.e\u{301}xt", "smiley.☻"]);

    te3.assert_output(&["--extension", "☻"], "smiley.☻");

    te3.assert_output(&["--extension", ".e\u{301}xt"], "latin1.e\u{301}xt");

    let te4 = TestEnv::new(&[], &[".hidden", "test.hidden"]);

    te4.assert_output(&["--hidden", "--extension", ".hidden"], "test.hidden");
}

/// No file extension (test for the pattern provided in the --help text)
#[test]
fn test_no_extension() {
    let te = TestEnv::new(
        DEFAULT_DIRS,
        &["a.foo", "aa", "one/b.foo", "one/bb", "one/two/three/d"],
    );

    te.assert_output(
        &["^[^.]+$"],
        "aa
        one/
        one/bb
        one/two/
        one/two/three/
        one/two/three/d
        one/two/three/directory_foo/
        symlink",
    );

    te.assert_output(
        &["^[^.]+$", "--type", "file"],
        "aa
        one/bb
        one/two/three/d",
    );
}
/// Exclude patterns (--exclude)
#[test]
fn test_excludes() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--exclude", "*.foo"],
        "one/
        one/two/
        one/two/C.Foo2
        one/two/three/
        one/two/three/directory_foo/
        e1 e2
        symlink",
    );

    te.assert_output(
        &["--exclude", "*.foo", "--exclude", "*.Foo2"],
        "one/
        one/two/
        one/two/three/
        one/two/three/directory_foo/
        e1 e2
        symlink",
    );

    te.assert_output(
        &["--exclude", "*.foo", "--exclude", "*.Foo2", "foo"],
        "one/two/three/directory_foo/",
    );

    te.assert_output(
        &["--exclude", "one/two/", "foo"],
        "a.foo
        one/b.foo",
    );

    te.assert_output(
        &["--exclude", "one/**/*.foo"],
        "a.foo
        e1 e2
        one/
        one/two/
        one/two/C.Foo2
        one/two/three/
        one/two/three/directory_foo/
        symlink",
    );
}

#[test]
fn format() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--format", "path={}", "--path-separator=/"],
        "path=a.foo
        path=e1 e2
        path=one
        path=one/b.foo
        path=one/two
        path=one/two/C.Foo2
        path=one/two/c.foo
        path=one/two/three
        path=one/two/three/d.foo
        path=one/two/three/directory_foo
        path=symlink",
    );

    te.assert_output(
        &["foo", "--format", "noExt={.}", "--path-separator=/"],
        "noExt=a
        noExt=one/b
        noExt=one/two/C
        noExt=one/two/c
        noExt=one/two/three/d
        noExt=one/two/three/directory_foo",
    );

    te.assert_output(
        &["foo", "--format", "basename={/}", "--path-separator=/"],
        "basename=a.foo
        basename=b.foo
        basename=C.Foo2
        basename=c.foo
        basename=d.foo
        basename=directory_foo",
    );

    te.assert_output(
        &["foo", "--format", "name={/.}", "--path-separator=/"],
        "name=a
        name=b
        name=C
        name=c
        name=d
        name=directory_foo",
    );

    te.assert_output(
        &["foo", "--format", "parent={//}", "--path-separator=/"],
        "parent=.
        parent=one
        parent=one/two
        parent=one/two
        parent=one/two/three
        parent=one/two/three",
    );
}

/// Non-zero exit code (--quiet)
#[test]
fn test_quiet() {
    let dirs = &[];
    let files = &["a.foo", "b.foo"];
    let te = TestEnv::new(dirs, files);

    te.assert_output(&["-q"], "");
    te.assert_output(&["--quiet"], "");
    te.assert_output(&["--has-results"], "");
    te.assert_failure_with_error(&["--quiet", "c.foo"], "")
}

/// Literal search (--fixed-strings)
#[test]
fn test_fixed_strings() {
    let dirs = &["test1", "test2"];
    let files = &["test1/a.foo", "test1/a_foo", "test2/Download (1).tar.gz"];
    let te = TestEnv::new(dirs, files);

    // Regex search, dot is treated as "any character"
    te.assert_output(
        &["a.foo"],
        "test1/a.foo
         test1/a_foo",
    );

    // Literal search, dot is treated as character
    te.assert_output(&["--fixed-strings", "a.foo"], "test1/a.foo");

    // Regex search, parens are treated as group
    te.assert_output(&["download (1)"], "");

    // Literal search, parens are treated as characters
    te.assert_output(
        &["--fixed-strings", "download (1)"],
        "test2/Download (1).tar.gz",
    );

    // Combine with --case-sensitive
    te.assert_output(&["--fixed-strings", "--case-sensitive", "download (1)"], "");
}

/// Filenames with invalid UTF-8 sequences
#[cfg(target_os = "linux")]
#[test]
fn test_invalid_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dirs = &["test1"];
    let files = &[];
    let te = TestEnv::new(dirs, files);

    fs::File::create(
        te.test_root()
            .join(OsStr::from_bytes(b"test1/test_\xFEinvalid.txt")),
    )
    .unwrap();

    te.assert_output(&["", "test1/"], "test1/test_�invalid.txt");

    te.assert_output(&["invalid", "test1/"], "test1/test_�invalid.txt");

    // Should not be found under a different extension
    te.assert_output(&["-e", "zip", "", "test1/"], "");
}

/// Filtering for file size (--size)
#[test]
fn test_size() {
    let te = TestEnv::new(&[], &[]);

    create_file_with_size(te.test_root().join("0_bytes.foo"), 0);
    create_file_with_size(te.test_root().join("11_bytes.foo"), 11);
    create_file_with_size(te.test_root().join("30_bytes.foo"), 30);
    create_file_with_size(te.test_root().join("3_kilobytes.foo"), 3 * 1000);
    create_file_with_size(te.test_root().join("4_kibibytes.foo"), 4 * 1024);

    // Zero and non-zero sized files.
    te.assert_output(
        &["", "--size", "+0B"],
        "0_bytes.foo
        11_bytes.foo
        30_bytes.foo
        3_kilobytes.foo
        4_kibibytes.foo",
    );

    // Zero sized files.
    te.assert_output(&["", "--size", "-0B"], "0_bytes.foo");
    te.assert_output(&["", "--size", "0B"], "0_bytes.foo");
    te.assert_output(&["", "--size=0B"], "0_bytes.foo");
    te.assert_output(&["", "-S", "0B"], "0_bytes.foo");

    // Files with 2 bytes or more.
    te.assert_output(
        &["", "--size", "+2B"],
        "11_bytes.foo
        30_bytes.foo
        3_kilobytes.foo
        4_kibibytes.foo",
    );

    // Files with 2 bytes or less.
    te.assert_output(&["", "--size", "-2B"], "0_bytes.foo");

    // Files with size between 1 byte and 11 bytes.
    te.assert_output(&["", "--size", "+1B", "--size", "-11B"], "11_bytes.foo");

    // Files with size equal 11 bytes.
    te.assert_output(&["", "--size", "11B"], "11_bytes.foo");

    // Files with size between 1 byte and 30 bytes.
    te.assert_output(
        &["", "--size", "+1B", "--size", "-30B"],
        "11_bytes.foo
        30_bytes.foo",
    );

    // Combine with a search pattern
    te.assert_output(&["^11_", "--size", "+1B", "--size", "-30B"], "11_bytes.foo");

    // Files with size between 12 and 30 bytes.
    te.assert_output(&["", "--size", "+12B", "--size", "-30B"], "30_bytes.foo");

    // Files with size between 31 and 100 bytes.
    te.assert_output(&["", "--size", "+31B", "--size", "-100B"], "");

    // Files with size between 3 kibibytes and 5 kibibytes.
    te.assert_output(&["", "--size", "+3ki", "--size", "-5ki"], "4_kibibytes.foo");

    // Files with size between 3 kilobytes and 5 kilobytes.
    te.assert_output(
        &["", "--size", "+3k", "--size", "-5k"],
        "3_kilobytes.foo
        4_kibibytes.foo",
    );

    // Files with size greater than 3 kilobytes and less than 3 kibibytes.
    te.assert_output(&["", "--size", "+3k", "--size", "-3ki"], "3_kilobytes.foo");

    // Files with size equal 4 kibibytes.
    te.assert_output(&["", "--size", "+4ki", "--size", "-4ki"], "4_kibibytes.foo");
    te.assert_output(&["", "--size", "4ki"], "4_kibibytes.foo");
}

#[cfg(test)]
fn create_file_with_modified<P: AsRef<Path>>(path: P, duration_in_secs: u64) {
    let st = SystemTime::now() - Duration::from_secs(duration_in_secs);
    let ft = filetime::FileTime::from_system_time(st);
    fs::File::create(&path).expect("creation failed");
    filetime::set_file_times(&path, ft, ft).expect("time modification failed");
}

#[cfg(test)]
fn remove_symlink<P: AsRef<Path>>(path: P) {
    #[cfg(unix)]
    fs::remove_file(path).expect("remove symlink");

    // On Windows, symlinks remember whether they point to files or directories, so try both
    #[cfg(windows)]
    fs::remove_file(path.as_ref())
        .or_else(|_| fs::remove_dir(path.as_ref()))
        .expect("remove symlink");
}

#[test]
fn test_modified_relative() {
    let te = TestEnv::new(&[], &[]);
    remove_symlink(te.test_root().join("symlink"));
    create_file_with_modified(te.test_root().join("foo_0_now"), 0);
    create_file_with_modified(te.test_root().join("bar_1_min"), 60);
    create_file_with_modified(te.test_root().join("foo_10_min"), 600);
    create_file_with_modified(te.test_root().join("bar_1_h"), 60 * 60);
    create_file_with_modified(te.test_root().join("foo_2_h"), 2 * 60 * 60);
    create_file_with_modified(te.test_root().join("bar_1_day"), 24 * 60 * 60);

    te.assert_output(
        &["", "--changed-within", "15min"],
        "foo_0_now
        bar_1_min
        foo_10_min",
    );

    te.assert_output(
        &["", "--change-older-than", "15min"],
        "bar_1_h
        foo_2_h
        bar_1_day",
    );

    te.assert_output(
        &["foo", "--changed-within", "12h"],
        "foo_0_now
        foo_10_min
        foo_2_h",
    );
}

#[cfg(test)]
fn change_file_modified<P: AsRef<Path>>(path: P, iso_date: &str) {
    let st = iso_date
        .parse::<Timestamp>()
        .map(SystemTime::from)
        .expect("invalid date");
    let ft = filetime::FileTime::from_system_time(st);
    filetime::set_file_times(path, ft, ft).expect("time modification failde");
}

#[test]
fn test_modified_absolute() {
    let te = TestEnv::new(&[], &["15mar2018", "30dec2017"]);
    remove_symlink(te.test_root().join("symlink"));
    change_file_modified(te.test_root().join("15mar2018"), "2018-03-15T12:00:00Z");
    change_file_modified(te.test_root().join("30dec2017"), "2017-12-30T23:59:00Z");

    te.assert_output(
        &["", "--change-newer-than", "2018-01-01 00:00:00"],
        "15mar2018",
    );
    te.assert_output(
        &["", "--changed-before", "2018-01-01 00:00:00"],
        "30dec2017",
    );
}

#[test]
fn test_custom_path_separator() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["foo", "one", "--path-separator", "="],
        "one=b.foo
        one=two=c.foo
        one=two=C.Foo2
        one=two=three=d.foo
        one=two=three=directory_foo=",
    );
}

#[test]
fn test_base_directory() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--base-directory", "one"],
        "b.foo
        two/
        two/c.foo
        two/C.Foo2
        two/three/
        two/three/d.foo
        two/three/directory_foo/",
    );

    te.assert_output(
        &["--base-directory", "one/two/", "foo"],
        "c.foo
        C.Foo2
        three/d.foo
        three/directory_foo/",
    );

    // Explicit root path
    te.assert_output(
        &["--base-directory", "one", "foo", "two"],
        "two/c.foo
        two/C.Foo2
        two/three/d.foo
        two/three/directory_foo/",
    );

    // Ignore base directory when absolute path is used
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    let abs_base_dir = &format!("{abs_path}/one/two/", abs_path = &abs_path);
    te.assert_output(
        &["--base-directory", abs_base_dir, "foo", &abs_path],
        &format!(
            "{abs_path}/a.foo
            {abs_path}/one/b.foo
            {abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo/",
            abs_path = &abs_path
        ),
    );
}

#[test]
fn test_max_results() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    // Unrestricted
    te.assert_output(
        &["--max-results=0", "c.foo"],
        "one/two/C.Foo2
         one/two/c.foo",
    );

    // Limited to two results
    te.assert_output(
        &["--max-results=2", "c.foo"],
        "one/two/C.Foo2
         one/two/c.foo",
    );

    // Limited to one result. We could find either C.Foo2 or c.foo
    let assert_just_one_result_with_option = |option| {
        let output = te.assert_success_and_get_output(".", &[option, "c.foo"]);
        let stdout = String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace(&std::path::MAIN_SEPARATOR.to_string(), "/");
        assert!(stdout == "one/two/C.Foo2" || stdout == "one/two/c.foo");
    };
    assert_just_one_result_with_option("--max-results=1");
    assert_just_one_result_with_option("-1");

    // check that --max-results & -1 conflic with --exec
    te.assert_failure(&["thing", "--max-results=0", "--exec=cat"]);
    te.assert_failure(&["thing", "-1", "--exec=cat"]);
    te.assert_failure(&["thing", "--max-results=1", "-1", "--exec=cat"]);
}

/// Filenames with non-utf8 paths are passed to the executed program unchanged
///
/// Note:
/// - the test is disabled on Darwin/OSX, since it coerces file names to UTF-8,
///   even when the requested file name is not valid UTF-8.
/// - the test is currently disabled on Windows because I'm not sure how to create
///   invalid UTF-8 files on Windows
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn test_exec_invalid_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dirs = &["test1"];
    let files = &[];
    let te = TestEnv::new(dirs, files);

    fs::File::create(
        te.test_root()
            .join(OsStr::from_bytes(b"test1/test_\xFEinvalid.txt")),
    )
    .unwrap();

    te.assert_output_raw(
        &["", "test1/", "--exec", "echo", "{}"],
        b"test1/test_\xFEinvalid.txt\n",
    );

    te.assert_output_raw(
        &["", "test1/", "--exec", "echo", "{/}"],
        b"test_\xFEinvalid.txt\n",
    );

    te.assert_output_raw(&["", "test1/", "--exec", "echo", "{//}"], b"test1\n");

    te.assert_output_raw(
        &["", "test1/", "--exec", "echo", "{.}"],
        b"test1/test_\xFEinvalid\n",
    );

    te.assert_output_raw(
        &["", "test1/", "--exec", "echo", "{/.}"],
        b"test_\xFEinvalid\n",
    );
}

#[test]
fn test_list_details() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    // Make sure we can execute 'fd --list-details' without any errors.
    te.assert_success_and_get_output(".", &["--list-details"]);
}

#[test]
fn test_single_and_multithreaded_execution() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(&["--threads=1", "a.foo"], "a.foo");
    te.assert_output(&["--threads=16", "a.foo"], "a.foo");
}

/// Make sure that fd fails if numeric arguments can not be parsed
#[test]
fn test_number_parsing_errors() {
    let te = TestEnv::new(&[], &[]);

    te.assert_failure(&["--threads=a"]);
    te.assert_failure(&["-j", ""]);
    te.assert_failure(&["--threads=0"]);

    te.assert_failure(&["--min-depth=a"]);
    te.assert_failure(&["--mindepth=a"]);
    te.assert_failure(&["--max-depth=a"]);
    te.assert_failure(&["--maxdepth=a"]);
    te.assert_failure(&["--exact-depth=a"]);

    te.assert_failure(&["--max-buffer-time=a"]);

    te.assert_failure(&["--max-results=a"]);
}

#[test_case("--hidden", &["--no-hidden"] ; "hidden")]
#[test_case("--no-ignore", &["--ignore"] ; "no-ignore")]
#[test_case("--no-ignore-vcs", &["--ignore-vcs"] ; "no-ignore-vcs")]
#[test_case("--no-require-git", &["--require-git"] ; "no-require-git")]
#[test_case("--follow", &["--no-follow"] ; "follow")]
#[test_case("--absolute-path", &["--relative-path"] ; "absolute-path")]
#[test_case("-u", &["--ignore", "--no-hidden"] ; "u")]
#[test_case("-uu", &["--ignore", "--no-hidden"] ; "uu")]
fn test_opposing(flag: &str, opposing_flags: &[&str]) {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    let mut flags = vec![flag];
    flags.extend_from_slice(opposing_flags);
    let out_no_flags = te.assert_success_and_get_normalized_output(".", &[]);
    let out_opposing_flags = te.assert_success_and_get_normalized_output(".", &flags);

    assert_eq!(
        out_no_flags,
        out_opposing_flags,
        "{} should override {}",
        opposing_flags.join(" "),
        flag
    );
}

/// Print error if search pattern starts with a dot and --hidden is not set
/// (Unix only, hidden files on Windows work differently)
#[test]
#[cfg(unix)]
fn test_error_if_hidden_not_set_and_pattern_starts_with_dot() {
    let te = TestEnv::new(&[], &[".gitignore", ".whatever", "non-hidden"]);

    te.assert_failure(&["^\\.gitignore"]);
    te.assert_failure(&["--glob", ".gitignore"]);

    te.assert_output(&["--hidden", "^\\.gitignore"], ".gitignore");
    te.assert_output(&["--hidden", "--glob", ".gitignore"], ".gitignore");
    te.assert_output(&[".gitignore"], "");
}

#[test]
fn test_strip_cwd_prefix() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--strip-cwd-prefix", "."],
        "a.foo
        e1 e2
        one/
        one/b.foo
        one/two/
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/
        one/two/three/d.foo
        one/two/three/directory_foo/
        symlink",
    );
}

/// When fd is ran from a non-existent working directory, but an existent
/// directory is passed in the arguments, it should still run fine
#[test]
#[cfg(all(not(windows), not(target_os = "illumos")))]
fn test_invalid_cwd() {
    let te = TestEnv::new(&[], &[]);

    let root = te.test_root().join("foo");
    fs::create_dir(&root).unwrap();
    std::env::set_current_dir(&root).unwrap();
    fs::remove_dir(&root).unwrap();

    let output = std::process::Command::new(te.test_exe())
        .arg("query")
        .arg(te.test_root())
        .output()
        .unwrap();

    if !output.status.success() {
        panic!("{output:?}");
    }
}

#[test]
fn test_hyperlink() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    #[cfg(unix)]
    let hostname = nix::unistd::gethostname().unwrap().into_string().unwrap();
    #[cfg(not(unix))]
    let hostname = "";

    let expected = format!(
        "\x1b]8;;file://{}{}/a.foo\x1b\\a.foo\x1b]8;;\x1b\\",
        hostname,
        get_absolute_root_path(&te),
    );

    te.assert_output(&["--hyperlink=always", "a.foo"], &expected);
}
