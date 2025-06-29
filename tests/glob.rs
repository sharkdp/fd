mod testenv;

use crate::testenv::{TestEnv, DEFAULT_DIRS, DEFAULT_FILES};

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

    te.assert_output(
        &["--glob", "--full-path", "**/one/*/*.foo"],
        " one/two/c.foo",
    );

    te.assert_output(
        &["--glob", "--full-path", "**/one/*/*/*.foo"],
        " one/two/three/d.foo",
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
