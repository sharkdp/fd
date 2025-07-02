mod testenv;

use std::fs;
use std::io::Write;
use test_case::test_case;

use crate::testenv::{TestEnv, DEFAULT_DIRS, DEFAULT_FILES};

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
        one/two/three/directory_foo/",
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
        one/two/three/directory_foo/",
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

/// Ignore parent ignore files (--no-ignore-parent)
#[test]
fn test_no_ignore_parent() {
    let dirs = &["inner"];
    let files = &[
        "inner/parent-ignored",
        "inner/child-ignored",
        "inner/not-ignored",
    ];
    let te = TestEnv::new(dirs, files);

    // Ignore 'parent-ignored' in root
    fs::File::create(te.test_root().join(".gitignore"))
        .unwrap()
        .write_all(b"parent-ignored")
        .unwrap();
    // Ignore 'child-ignored' in inner
    fs::File::create(te.test_root().join("inner/.gitignore"))
        .unwrap()
        .write_all(b"child-ignored")
        .unwrap();

    te.assert_output_subdirectory("inner", &[], "not-ignored");

    te.assert_output_subdirectory(
        "inner",
        &["--no-ignore-parent"],
        "parent-ignored
        not-ignored",
    );
}

/// Ignore parent ignore files (--no-ignore-parent) with an inner git repo
#[test]
fn test_no_ignore_parent_inner_git() {
    let dirs = &["inner"];
    let files = &[
        "inner/parent-ignored",
        "inner/child-ignored",
        "inner/not-ignored",
    ];
    let te = TestEnv::new(dirs, files);

    // Make the inner folder also appear as a git repo
    fs::create_dir_all(te.test_root().join("inner/.git")).unwrap();

    // Ignore 'parent-ignored' in root
    fs::File::create(te.test_root().join(".gitignore"))
        .unwrap()
        .write_all(b"parent-ignored")
        .unwrap();
    // Ignore 'child-ignored' in inner
    fs::File::create(te.test_root().join("inner/.gitignore"))
        .unwrap()
        .write_all(b"child-ignored")
        .unwrap();

    te.assert_output_subdirectory(
        "inner",
        &[],
        "not-ignored
        parent-ignored",
    );

    te.assert_output_subdirectory(
        "inner",
        &["--no-ignore-parent"],
        "not-ignored
        parent-ignored",
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

/// Don't require git to respect gitignore (--no-require-git)
#[test]
fn test_respect_ignore_files() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    // Not in a git repo anymore
    fs::remove_dir(te.test_root().join(".git")).unwrap();

    // don't respect gitignore because we're not in a git repo
    te.assert_output(
        &["foo"],
        "a.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );

    // respect gitignore because we set `--no-require-git`
    te.assert_output(
        &["--no-require-git", "foo"],
        "a.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );

    // make sure overriding works
    te.assert_output(
        &["--no-require-git", "--require-git", "foo"],
        "a.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );

    te.assert_output(
        &["--no-require-git", "--no-ignore", "foo"],
        "a.foo
        gitignored.foo
        fdignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );
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
        one/two/three/directory_foo/",
    );
}

/// Test that --no-ignore-vcs still respects .fdignored in parent directory
#[test]
fn test_no_ignore_vcs_child_dir() {
    let te = TestEnv::new(
        &["inner"],
        &["inner/fdignored.foo", "inner/foo", "inner/gitignored.foo"],
    );

    te.assert_output_subdirectory(
        "inner",
        &["--no-ignore-vcs", "foo"],
        "foo
        gitignored.foo",
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
        ".hidden.foo
        a.foo
        fdignored.foo
        gitignored.foo
        one/b.foo
        one/two/c.foo
        one/two/C.Foo2
        one/two/three/d.foo
        one/two/three/directory_foo/",
    );
}

#[cfg(not(windows))]
#[test]
fn test_global_ignore() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES).global_ignore_file("one");
    te.assert_output(
        &[],
        "a.foo
    e1 e2
    symlink",
    );
}

#[cfg(not(windows))]
#[test_case("--unrestricted", ".hidden.foo
a.foo
fdignored.foo
gitignored.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo/"; "unrestricted")]
#[test_case("--no-ignore", "a.foo
fdignored.foo
gitignored.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo/"; "no-ignore")]
#[test_case("--no-global-ignore-file", "a.foo
one/b.foo
one/two/c.foo
one/two/C.Foo2
one/two/three/d.foo
one/two/three/directory_foo/"; "no-global-ignore-file")]
fn test_no_global_ignore(flag: &str, expected_output: &str) {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES).global_ignore_file("one");
    te.assert_output(&[flag, "foo"], expected_output);
}

#[test]
fn test_gitignore_parent() {
    let te = TestEnv::new(&["sub"], &[".abc", "sub/.abc"]);

    fs::File::create(te.test_root().join(".gitignore"))
        .unwrap()
        .write_all(b".abc\n")
        .unwrap();

    te.assert_output_subdirectory("sub", &["--hidden"], "");
    te.assert_output_subdirectory("sub", &["--hidden", "--search-path", "."], "");
}

/// Test behavior of .git directory with various flags
#[test]
fn test_git_dir() {
    let te = TestEnv::new(
        &[".git/one", "other_dir/.git", "nested/dir/.git"],
        &[
            ".git/one/foo.a",
            ".git/.foo",
            ".git/a.foo",
            "other_dir/.git/foo1",
            "nested/dir/.git/foo2",
        ],
    );

    te.assert_output(
        &["--hidden", "foo"],
        ".git/one/foo.a
        .git/.foo
        .git/a.foo
        other_dir/.git/foo1
        nested/dir/.git/foo2",
    );
    te.assert_output(&["--no-ignore", "foo"], "");
    te.assert_output(
        &["--hidden", "--no-ignore", "foo"],
        ".git/one/foo.a
         .git/.foo
         .git/a.foo
         other_dir/.git/foo1
         nested/dir/.git/foo2",
    );
    te.assert_output(
        &["--hidden", "--no-ignore-vcs", "foo"],
        ".git/one/foo.a
         .git/.foo
         .git/a.foo
         other_dir/.git/foo1
         nested/dir/.git/foo2",
    );
}
