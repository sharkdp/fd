use regex::escape;

use crate::testenv::{get_test_env_with_abs_path, TestEnv, DEFAULT_DIRS, DEFAULT_FILES};

/// Symlink as search directory
#[test]
fn test_symlink_as_root() {
    let mut te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    te.create_broken_symlink("broken_symlink")
        .expect("Failed to create broken symlink.");

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
            {dir}/broken_symlink
            {dir}/e1 e2
            {dir}/one/
            {dir}/one/b.foo
            {dir}/one/two/
            {dir}/one/two/c.foo
            {dir}/one/two/C.Foo2
            {dir}/one/two/three/
            {dir}/one/two/three/d.foo
            {dir}/one/two/three/directory_foo/
            {dir}/symlink",
            dir = &parent_parent
        ),
    );
}

#[test]
fn test_symlink_and_absolute_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    let expected_path = if cfg!(windows) { "symlink" } else { "one/two" };

    te.assert_output_subdirectory(
        "symlink",
        &["--absolute-path"],
        &format!(
            "{abs_path}/{expected_path}/c.foo
            {abs_path}/{expected_path}/C.Foo2
            {abs_path}/{expected_path}/three/
            {abs_path}/{expected_path}/three/d.foo
            {abs_path}/{expected_path}/three/directory_foo/",
            abs_path = &abs_path,
            expected_path = expected_path
        ),
    );
}

#[test]
fn test_symlink_as_absolute_root() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &["", &format!("{abs_path}/symlink")],
        &format!(
            "{abs_path}/symlink/c.foo
            {abs_path}/symlink/C.Foo2
            {abs_path}/symlink/three/
            {abs_path}/symlink/three/d.foo
            {abs_path}/symlink/three/directory_foo/",
            abs_path = &abs_path
        ),
    );
}

#[test]
fn test_symlink_and_full_path() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    let root = te.system_root();
    let prefix = escape(&root.to_string_lossy());

    let expected_path = if cfg!(windows) { "symlink" } else { "one/two" };

    te.assert_output_subdirectory(
        "symlink",
        &[
            "--absolute-path",
            "--full-path",
            &format!("^{prefix}.*three"),
        ],
        &format!(
            "{abs_path}/{expected_path}/three/
            {abs_path}/{expected_path}/three/d.foo
            {abs_path}/{expected_path}/three/directory_foo/",
            abs_path = &abs_path,
            expected_path = expected_path
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
            &format!("^{prefix}.*symlink.*three"),
            &format!("{abs_path}/symlink"),
        ],
        &format!(
            "{abs_path}/symlink/three/
            {abs_path}/symlink/three/d.foo
            {abs_path}/symlink/three/directory_foo/",
            abs_path = &abs_path
        ),
    );
}
