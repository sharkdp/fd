// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::env::current_dir;
use std::io;
use std::path::{Path, PathBuf};

pub fn path_absolute_form(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let path = path.strip_prefix(".").unwrap_or(path);
        current_dir().map(|path_buf| path_buf.join(path))
    }
}

pub fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path_buf = path_absolute_form(path)?;

    #[cfg(windows)]
    let path_buf = Path::new(
        path_buf
            .as_path()
            .to_string_lossy()
            .trim_left_matches(r"\\?\"),
    ).to_path_buf();

    Ok(path_buf)
}

// Path::is_dir() is not guarandteed to be intuitively correct for "." and ".."
// See: https://github.com/rust-lang/rust/issues/45302
pub fn is_dir(path: &Path) -> bool {
    if path.file_name().is_some() {
        path.is_dir()
    } else {
        path.is_dir() && path.canonicalize().is_ok()
    }
}

/// Remove the `./` prefix from a path.
///
/// This code is an adapted version of the `pathutil::strip_prefix`
/// helper function in ripgrep (https://github.com/BurntSushi/ripgrep).
#[cfg(unix)]
pub fn strip_current_dir<'a>(path: &'a Path) -> &'a Path {
    use std::os::unix::ffi::OsStrExt;
    use std::ffi::OsStr;

    let prefix = b"./";
    let path_raw = path.as_os_str().as_bytes();
    if path_raw.len() < 2 || &path_raw[0..2] != prefix {
        path
    } else {
        Path::new(OsStr::from_bytes(&path_raw[2..]))
    }
}

/// Remove the `./` prefix from a path.
#[cfg(not(unix))]
pub fn strip_current_dir<'a>(path: &'a Path) -> &'a Path {
    path.strip_prefix("./").unwrap_or(&path)
}

#[test]
fn test_strip_current_dir() {
    let expect_stripped = |expected, input| {
        let stripped = Path::new(expected);
        let base = Path::new(input);
        assert_eq!(stripped, strip_current_dir(&base));
    };

    expect_stripped("foo/bar.txt", "./foo/bar.txt");
    expect_stripped("", "./");
    expect_stripped("foo", "./foo");
    expect_stripped("foo.txt", "foo.txt");
    expect_stripped("../foo", "../foo");
}
