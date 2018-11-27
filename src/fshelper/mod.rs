// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::env::current_dir;
use std::fs;
use std::io;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use walk::DirEntry;

pub fn path_absolute_form(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let path = path.strip_prefix(".").unwrap_or(path);
    current_dir().map(|path_buf| path_buf.join(path))
}

pub fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path_buf = path_absolute_form(path)?;

    #[cfg(windows)]
    let path_buf = Path::new(
        path_buf
            .as_path()
            .to_string_lossy()
            .trim_left_matches(r"\\?\"),
    )
    .to_path_buf();

    Ok(path_buf)
}

// Path::is_dir() is not guaranteed to be intuitively correct for "." and ".."
// See: https://github.com/rust-lang/rust/issues/45302
pub fn is_dir(path: &Path) -> bool {
    path.is_dir() && (path.file_name().is_some() || path.canonicalize().is_ok())
}

#[cfg(any(unix, target_os = "redox"))]
pub fn is_executable(md: &fs::Metadata) -> bool {
    md.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
pub fn is_executable(_: &fs::Metadata) -> bool {
    false
}

pub fn is_empty(entry: &DirEntry) -> bool {
    if let Some(file_type) = entry.file_type {
        if file_type.is_dir() {
            if let Ok(mut entries) = fs::read_dir(&entry.path) {
                entries.next().is_none()
            } else {
                false
            }
        } else if file_type.is_file() {
            fs::metadata(&entry.path)
                .map(|m| m.len() == 0)
                .unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    }
}
