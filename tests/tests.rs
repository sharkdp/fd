// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

//! Integration tests for the CLI interface of fd.

mod testenv;

use crate::testenv::TestEnv;
use regex::escape;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime};

static DEFAULT_DIRS: &'static [&'static str] = &["one/two/three", "one/two/three/directory_foo"];

static DEFAULT_FILES: &'static [&'static str] = &[
    "a.foo",
    "one/b.foo",
    "one/two/c.foo",
    "one/two/C.Foo2",
    "one/two/three/d.foo",
    "fdignored.foo",
    "gitignored.foo",
    ".hidden.foo",
    "e1 e2",
];

fn get_absolute_root_path(env: &TestEnv) -> String {
    let path = env
        .test_root()
        .canonicalize()
        .expect("absolute path")
        .to_str()
        .expect("string")
        .to_string();

    #[cfg(windows)]
    let path = path.trim_start_matches(r"\\?\").to_string();

    path
}

#[cfg(test)]
fn get_test_env_with_abs_path(dirs: &[&'static str], files: &[&'static str]) -> (TestEnv, String) {
    let env = TestEnv::new(dirs, files);
    let root_path = get_absolute_root_path(&env);
    (env, root_path)
}

#[cfg(test)]
fn create_file_with_size<P: AsRef<Path>>(path: P, size_in_bytes: usize) {
    let content = "#".repeat(size_in_bytes);
    let mut f = fs::File::create::<P>(path).unwrap();
    f.write(content.as_bytes()).unwrap();
}

/// Simple tests
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
        one/two/three/directory_foo",
    );

    te.assert_output(
        &[],
        "a.foo
        e1 e2
        one
        one/b.foo
        one/two
        one/two/c.foo
        one/two/C.Foo2
        one/two/three
        one/two/three/d.foo
        one/two/three/directory_foo
        symlink",
    );
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
        one/two/three/directory_foo",
    );

    te.assert_output(
        &["foo", "one/two/three"],
        "one/two/three/d.foo
        one/two/three/directory_foo",
    );

    te.assert_output_subdirectory(
        "one/two",
        &["foo", "../../"],
        "../../a.foo
        ../../one/b.foo
        ../../one/two/c.foo
        ../../one/two/C.Foo2
        ../../one/two/three/d.foo
        ../../one/two/three/directory_foo",
    );

    te.assert_output_subdirectory(
        "one/two/three",
        &["", ".."],
        "../c.foo
        ../C.Foo2
        ../three
        ../three/d.foo
        ../three/directory_foo",
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

/// Glob-based searches (--glob)
#[test]
fn test_glob_searches() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--glob", "*.foo"],
        "a.foo
        one/b.foo
        one/two/c.foo
        one/two/three/d.foo",
    );

    te.assert_output(
        &["--glob", "[a-c].foo"],
        "a.foo
        one/b.foo
        one/two/c.foo",
    );

    te.assert_output(
        &["--glob", "[a-c].foo*"],
        "a.foo
        one/b.foo
        one/two/C.Foo2
        one/two/c.foo",
    );
}

/// Glob-based searches (--glob) in combination with full path searches (--full-path)
#[cfg(not(windows))] // TODO: make this work on Windows
#[test]
fn test_full_path_glob_searches() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--glob", "--full-path", "**/one/**/*.foo"],
        "one/b.foo
        one/two/c.foo
        one/two/three/d.foo",
    );
}

#[test]
fn test_smart_case_glob_searches() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--glob", "c.foo*"],
        "one/two/C.Foo2
        one/two/c.foo",
    );

    te.assert_output(&["--glob", "C.Foo*"], "one/two/C.Foo2");
}

/// Glob-based searches (--glob) in combination with --case-sensitive
#[test]
fn test_case_sensitive_glob_searches() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(&["--glob", "--case-sensitive", "c.foo*"], "one/two/c.foo");
}

/// Glob-based searches (--glob) in combination with --extension
#[test]
fn test_glob_searches_with_extension() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--glob", "--extension", "foo2", "[a-z].*"],
        "one/two/C.Foo2",
    );
}

/// Make sure that --regex overrides --glob
#[test]
fn test_regex_overrides_glob() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(&["--glob", "--regex", "Foo2$"], "one/two/C.Foo2");
}

/// Full path search (--full-path)
#[test]
fn test_full_path() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    let root = te.system_root();
    let prefix = escape(&root.to_string_lossy());

    te.assert_output(
        &[
            "--full-path",
            &format!("^{prefix}.*three.*foo$", prefix = prefix),
        ],
        "one/two/three/d.foo
        one/two/three/directory_foo",
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
        one/two/three/directory_foo",
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

/// Ignored files (--no-ignore)
#[test]
fn test_no_ignore() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--no-ignore", "foo"],
        "a.foo
        fdignored.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo",
    );

    te.assert_output(
        &["--hidden", "--no-ignore", "foo"],
        ".hidden.foo
        a.foo
        fdignored.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo",
    );
}

/// .gitignore and .fdignore
#[test]
fn test_gitignore_and_fdignore() {
    let files = &[
        "ignored-by-nothing",
        "ignored-by-fdignore",
        "ignored-by-gitignore",
        "ignored-by-both",
    ];
    let te = TestEnv::new(&[], files);

    fs::File::create(te.test_root().join(".fdignore"))
        .unwrap()
        .write_all(b"ignored-by-fdignore\nignored-by-both")
        .unwrap();

    fs::File::create(te.test_root().join(".gitignore"))
        .unwrap()
        .write_all(b"ignored-by-gitignore\nignored-by-both")
        .unwrap();

    te.assert_output(&["ignored"], "ignored-by-nothing");

    te.assert_output(
        &["--no-ignore-vcs", "ignored"],
        "ignored-by-nothing
        ignored-by-gitignore",
    );

    te.assert_output(
        &["--no-ignore", "ignored"],
        "ignored-by-nothing
        ignored-by-fdignore
        ignored-by-gitignore
        ignored-by-both",
    );
}

/// Precedence of .fdignore files
#[test]
fn test_custom_ignore_precedence() {
    let dirs = &["inner"];
    let files = &["inner/foo"];
    let te = TestEnv::new(dirs, files);

    // Ignore 'foo' via .gitignore
    fs::File::create(te.test_root().join("inner/.gitignore"))
        .unwrap()
        .write_all(b"foo")
        .unwrap();

    // Whitelist 'foo' via .fdignore
    fs::File::create(te.test_root().join(".fdignore"))
        .unwrap()
        .write_all(b"!foo")
        .unwrap();

    te.assert_output(&["foo"], "inner/foo");

    te.assert_output(&["--no-ignore-vcs", "foo"], "inner/foo");

    te.assert_output(&["--no-ignore", "foo"], "inner/foo");
}

/// VCS ignored files (--no-ignore-vcs)
#[test]
fn test_no_ignore_vcs() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--no-ignore-vcs", "foo"],
        "a.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo",
    );
}

/// Custom ignore files (--ignore-file)
#[test]
fn test_custom_ignore_files() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    // Ignore 'C.Foo2' and everything in 'three'.
    fs::File::create(te.test_root().join("custom.ignore"))
        .unwrap()
        .write_all(b"C.Foo2\nthree")
        .unwrap();

    te.assert_output(
        &["--ignore-file", "custom.ignore", "foo"],
        "a.foo
        one/b.foo
        one/two/c.foo",
    );
}

/// Ignored files with ripgrep aliases (-u / -uu)
#[test]
fn test_no_ignore_aliases() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["-u", "foo"],
        "a.foo
        fdignored.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo",
    );

    te.assert_output(
        &["-uu", "foo"],
        ".hidden.foo
        a.foo
        fdignored.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo",
    );
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

/// Null separator (--print0)
#[test]
fn test_print0() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--print0", "foo"],
        "a.fooNULL
        one/b.fooNULL
        one/two/C.Foo2NULL
        one/two/c.fooNULL
        one/two/three/d.fooNULL
        one/two/three/directory_fooNULL",
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
        one
        one/b.foo
        one/two
        one/two/c.foo
        one/two/C.Foo2
        one/two/three
        symlink",
    );

    te.assert_output(
        &["--max-depth", "2"],
        "a.foo
        e1 e2
        one
        one/b.foo
        one/two
        symlink",
    );

    te.assert_output(
        &["--max-depth", "1"],
        "a.foo
        e1 e2
        one
        symlink",
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
            {abs_path}/one
            {abs_path}/one/b.foo
            {abs_path}/one/two
            {abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo
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
            {abs_path}/one/two/three/directory_foo",
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
            {abs_path}/one/two/three/directory_foo",
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
            {abs_path}/one/two/three/directory_foo",
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
        "one
        one/two
        one/two/three
        one/two/three/directory_foo",
    );

    te.assert_output(
        &["--type", "d", "--type", "l"],
        "one
        one/two
        one/two/three
        one/two/three/directory_foo
        symlink",
    );

    te.assert_output(&["--type", "l"], "symlink");
}

/// Test `--type executable`
#[cfg(unix)]
#[test]
fn test_type_executable() {
    use std::os::unix::fs::OpenOptionsExt;

    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .mode(0o777)
        .open(te.test_root().join("executable-file.sh"))
        .unwrap();

    te.assert_output(&["--type", "executable"], "executable-file.sh");

    te.assert_output(
        &["--type", "executable", "--type", "directory"],
        "executable-file.sh
        one
        one/two
        one/two/three
        one/two/three/directory_foo",
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
        dir_empty",
    );

    te.assert_output(
        &["--type", "empty", "--type", "file", "--type", "directory"],
        "0_bytes.foo
        dir_empty",
    );

    te.assert_output(&["--type", "empty", "--type", "file"], "0_bytes.foo");

    te.assert_output(&["--type", "empty", "--type", "directory"], "dir_empty");
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

/// Symlink as search directory
#[test]
fn test_symlink_as_root() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    // From: http://pubs.opengroup.org/onlinepubs/9699919799/functions/getcwd.html
    // The getcwd() function shall place an absolute pathname of the current working directory in
    // the array pointed to by buf, and return buf. The pathname shall contain no components that
    // are dot or dot-dot, or are symbolic links.
    //
    // Key points:
    // 1. The path of the current working directory of a Unix process cannot contain symlinks.
    // 2. The path of the current working directory of a Windows process can contain symlinks.
    //
    // More:
    // 1. On Windows, symlinks are resolved after the ".." component.
    // 2. On Unix, symlinks are resolved immediately as encountered.

    let parent_parent = if cfg!(windows) { ".." } else { "../.." };
    te.assert_output_subdirectory(
        "symlink",
        &["", parent_parent],
        &format!(
            "{dir}/a.foo
            {dir}/e1 e2
            {dir}/one
            {dir}/one/b.foo
            {dir}/one/two
            {dir}/one/two/c.foo
            {dir}/one/two/C.Foo2
            {dir}/one/two/three
            {dir}/one/two/three/d.foo
            {dir}/one/two/three/directory_foo
            {dir}/symlink",
            dir = &parent_parent
        ),
    );
}

#[test]
fn test_symlink_and_absolute_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output_subdirectory(
        "symlink",
        &["--absolute-path"],
        &format!(
            "{abs_path}/one/two/c.foo
            {abs_path}/one/two/C.Foo2
            {abs_path}/one/two/three
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo",
            abs_path = &abs_path
        ),
    );
}

#[test]
fn test_symlink_as_absolute_root() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["", &format!("{abs_path}/symlink", abs_path = abs_path)],
        &format!(
            "{abs_path}/symlink/c.foo
            {abs_path}/symlink/C.Foo2
            {abs_path}/symlink/three
            {abs_path}/symlink/three/d.foo
            {abs_path}/symlink/three/directory_foo",
            abs_path = &abs_path
        ),
    );
}

#[test]
fn test_symlink_and_full_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    let root = te.system_root();
    let prefix = escape(&root.to_string_lossy());

    te.assert_output_subdirectory(
        "symlink",
        &[
            "--absolute-path",
            "--full-path",
            &format!("^{prefix}.*three", prefix = prefix),
        ],
        &format!(
            "{abs_path}/one/two/three
            {abs_path}/one/two/three/d.foo
            {abs_path}/one/two/three/directory_foo",
            abs_path = &abs_path
        ),
    );
}

#[test]
fn test_symlink_and_full_path_abs_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    let root = te.system_root();
    let prefix = escape(&root.to_string_lossy());
    te.assert_output(
        &[
            "--full-path",
            &format!("^{prefix}.*symlink.*three", prefix = prefix),
            &format!("{abs_path}/symlink", abs_path = abs_path),
        ],
        &format!(
            "{abs_path}/symlink/three
            {abs_path}/symlink/three/d.foo
            {abs_path}/symlink/three/directory_foo",
            abs_path = &abs_path
        ),
    );
}

/// Exclude patterns (--exclude)
#[test]
fn test_excludes() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["--exclude", "*.foo"],
        "one
        one/two
        one/two/C.Foo2
        one/two/three
        one/two/three/directory_foo
        e1 e2
        symlink",
    );

    te.assert_output(
        &["--exclude", "*.foo", "--exclude", "*.Foo2"],
        "one
        one/two
        one/two/three
        one/two/three/directory_foo
        e1 e2
        symlink",
    );

    te.assert_output(
        &["--exclude", "*.foo", "--exclude", "*.Foo2", "foo"],
        "one/two/three/directory_foo",
    );

    te.assert_output(
        &["--exclude", "one/two", "foo"],
        "a.foo
        one/b.foo",
    );

    te.assert_output(
        &["--exclude", "one/**/*.foo"],
        "a.foo
        e1 e2
        one
        one/two
        one/two/C.Foo2
        one/two/three
        one/two/three/directory_foo
        symlink",
    );
}

/// Shell script execution (--exec)
#[test]
fn test_exec() {
    assert_exec_output("--exec");
}

/// Shell script execution using -exec
#[test]
fn test_exec_substitution() {
    assert_exec_output("-exec");
}

// Shell script execution using -x
#[test]
fn test_exec_short_arg() {
    assert_exec_output("-x");
}

#[cfg(test)]
fn assert_exec_output(exec_style: &str) {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    // TODO Windows tests: D:file.txt \file.txt \\server\share\file.txt ...
    if !cfg!(windows) {
        te.assert_output(
            &["--absolute-path", "foo", exec_style, "echo"],
            &format!(
                "{abs_path}/a.foo
                {abs_path}/one/b.foo
                {abs_path}/one/two/C.Foo2
                {abs_path}/one/two/c.foo
                {abs_path}/one/two/three/d.foo
                {abs_path}/one/two/three/directory_foo",
                abs_path = &abs_path
            ),
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{}"],
            "a.foo
            one/b.foo
            one/two/C.Foo2
            one/two/c.foo
            one/two/three/d.foo
            one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{.}"],
            "a
            one/b
            one/two/C
            one/two/c
            one/two/three/d
            one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{/}"],
            "a.foo
            b.foo
            C.Foo2
            c.foo
            d.foo
            directory_foo",
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{/.}"],
            "a
            b
            C
            c
            d
            directory_foo",
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{//}"],
            ".
            one
            one/two
            one/two
            one/two/three
            one/two/three",
        );

        te.assert_output(&["e1", exec_style, "printf", "%s.%s\n"], "e1 e2.");
    }
}

#[test]
fn test_exec_batch() {
    assert_exec_batch_output("--exec-batch");
}

#[test]
fn test_exec_batch_short_arg() {
    assert_exec_batch_output("-X");
}

#[cfg(test)]
fn assert_exec_batch_output(exec_style: &str) {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    let te = te.normalize_line(true);

    // TODO Test for windows
    if !cfg!(windows) {
        te.assert_output(
            &["--absolute-path", "foo", exec_style, "echo"],
            &format!(
                "{abs_path}/a.foo {abs_path}/one/b.foo {abs_path}/one/two/C.Foo2 {abs_path}/one/two/c.foo {abs_path}/one/two/three/d.foo {abs_path}/one/two/three/directory_foo",
                abs_path = &abs_path
            ),
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{}"],
            "a.foo one/b.foo one/two/C.Foo2 one/two/c.foo one/two/three/d.foo one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", exec_style, "echo", "{/}"],
            "a.foo b.foo C.Foo2 c.foo d.foo directory_foo",
        );

        te.assert_output(&["no_match", exec_style, "echo", "Matched: ", "{/}"], "");

        te.assert_error(
            &["foo", exec_style, "echo", "{}", "{}"],
            "[fd error]: Only one placeholder allowed for batch commands",
        );

        te.assert_error(
            &["foo", exec_style, "echo", "{/}", ";", "-x", "echo"],
            "error: The argument '--exec <cmd>' cannot be used with '--exec-batch <cmd>'",
        );

        te.assert_error(
            &["foo", exec_style],
            "error: The argument '--exec-batch <cmd>' requires a value but none was supplied",
        );

        te.assert_error(
            &["foo", exec_style, "echo {}"],
            "[fd error]: First argument of exec-batch is expected to be a fixed executable",
        );
    }
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
}

#[cfg(test)]
fn create_file_with_modified<P: AsRef<Path>>(path: P, duration_in_secs: u64) {
    let st = SystemTime::now() - Duration::from_secs(duration_in_secs);
    let ft = filetime::FileTime::from_system_time(st);
    fs::File::create(&path).expect("creation failed");
    filetime::set_file_times(&path, ft, ft).expect("time modification failed");
}

#[test]
fn test_modified_relative() {
    let te = TestEnv::new(&[], &[]);
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
    let st = humantime::parse_rfc3339(iso_date).expect("invalid date");
    let ft = filetime::FileTime::from_system_time(st);
    filetime::set_file_times(path, ft, ft).expect("time modification failde");
}

#[test]
fn test_modified_asolute() {
    let te = TestEnv::new(&[], &["15mar2018", "30dec2017"]);
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
        one=two=three=directory_foo",
    );
}
