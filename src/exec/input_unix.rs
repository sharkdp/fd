// Copyright (c) 2018 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#![cfg(unix)]
///! This division is a temporary solution, until we can perform these operations on
///! `Path`s directly on all platforms, without going thru `str`
use std::ffi::OsStr;
use std::path::Path;

use std::os::unix::ffi::OsStrExt;

/// Removes the parent component of the path
pub fn basename(path: &Path) -> &Path {
    use std::path::Component;
    match path.components().last() {
        Some(Component::Normal(s)) => s.as_ref(),
        _ => "".as_ref(),
    }
}

/// Removes the extension (if any) from the path
pub fn remove_extension(path: &Path) -> &Path {
    if let Some(ext) = path.extension() {
        let ext_bytes = ext.as_bytes().len();
        let stem = strip_suffix_bytes(path, ext_bytes + 1 /* for the dot character */);
        OsStr::from_bytes(stem).as_ref()
    } else {
        path
    }
}

/// Removes the basename from the path.
pub fn dirname(path: &Path) -> &OsStr {
    let base = basename(path);
    let base_bytes = base.as_os_str().as_bytes().len();

    let stripped = strip_suffix_bytes(path, base_bytes);
    if stripped.is_empty() {
        ".".as_ref()
    } else if let Some((&b'/', noslash)) = stripped.split_last() {
        OsStr::from_bytes(noslash)
    } else {
        OsStr::from_bytes(stripped)
    }
}

/// This is only meaningful on unix, where a `Path` is a sequence of non-zero bytes
/// This function slices the given `path` and returns the same data,
/// minus exactly `count` bytes at the end.
fn strip_suffix_bytes(path: &Path, count: usize) -> &[u8] {
    let all_bytes = path.as_os_str().as_bytes().len();
    let remaining = all_bytes.saturating_sub(count);

    &path.as_os_str().as_bytes()[..remaining]
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn pathinate(s: &str) -> PathBuf {
        OsString::from(s).into()
    }

    /// IMPORTANTi: comparisons are performed here as `OsString`, NEVER as `Path`s.
    /// This is because path comparison disregards a final separator,
    /// and we were fortunate enough to have this caught by integration tests
    macro_rules! func_tests {
        ($($name:ident: $func:ident for $input:expr => $output:expr)+) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(OsString::from($func(&pathinate($input))), OsString::from($output));
                }
            )+
        }
    }

    func_tests! {
        remove_ext_simple:  remove_extension  for  "foo.txt"      =>  "foo"
        remove_ext_dir:     remove_extension  for  "dir/foo.txt"  =>  "dir/foo"
        hidden:             remove_extension  for  ".foo"         =>  ".foo"
        remove_ext_utf8:    remove_extension  for  "💖.txt"       =>  "💖"
        remove_ext_empty:   remove_extension  for  ""             =>  ""

        basename_simple:  basename  for  "foo.txt"      =>  "foo.txt"
        basename_dir:     basename  for  "dir/foo.txt"  =>  "foo.txt"
        basename_empty:   basename  for  ""             =>  ""
        basename_utf8_0:  basename  for  "💖/foo.txt"   =>  "foo.txt"
        basename_utf8_1:  basename  for  "dir/💖.txt"   =>  "💖.txt"

        dirname_simple:  dirname  for  "foo.txt"      =>  "."
        dirname_dir:     dirname  for  "dir/foo.txt"  =>  "dir"
        dirname_utf8_0:  dirname  for  "💖/foo.txt"   =>  "💖"
        dirname_utf8_1:  dirname  for  "dir/💖.txt"   =>  "dir"
        dirname_empty:   dirname  for  ""             =>  "."
    }
}
