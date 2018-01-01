// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::path::MAIN_SEPARATOR;

/// Removes the parent component of the path
pub fn basename(path: &str) -> &str {
    let mut index = 0;
    for (id, character) in path.char_indices() {
        if character == MAIN_SEPARATOR {
            index = id;
        }
    }

    // FIXME: On Windows, should return what for C:file.txt D:file.txt and \\server\share ?
    if index != 0 {
        return &path[index + 1..];
    }

    path
}

/// Removes the extension from the path
pub fn remove_extension(path: &str) -> &str {
    let mut has_dir = false;
    let mut dir_index = 0;
    let mut ext_index = 0;

    for (id, character) in path.char_indices() {
        if character == MAIN_SEPARATOR {
            has_dir = true;
            dir_index = id;
        }
        if character == '.' {
            ext_index = id;
        }
    }

    // Account for hidden files and directories
    if ext_index != 0 && (!has_dir || dir_index + 2 <= ext_index) {
        return &path[0..ext_index];
    }

    path
}

/// Removes the basename from the path.
pub fn dirname(path: &str) -> &str {
    let mut has_dir = false;
    let mut index = 0;
    for (id, character) in path.char_indices() {
        if character == MAIN_SEPARATOR {
            has_dir = true;
            index = id;
        }
    }

    // FIXME: On Windows, return what for C:file.txt D:file.txt and \\server\share ?
    if !has_dir {
        "."
    } else if index == 0 {
        &path[..1]
    } else {
        &path[0..index]
    }
}

#[cfg(test)]
mod tests {
    use super::{basename, dirname, remove_extension, MAIN_SEPARATOR};

    fn correct(input: &str) -> String {
        input.replace('/', &MAIN_SEPARATOR.to_string())
    }

    #[test]
    fn path_remove_ext_simple() {
        assert_eq!(remove_extension("foo.txt"), "foo");
    }

    #[test]
    fn path_remove_ext_dir() {
        assert_eq!(
            remove_extension(&correct("dir/foo.txt")),
            correct("dir/foo")
        );
    }

    #[test]
    fn path_hidden() {
        assert_eq!(remove_extension(".foo"), ".foo")
    }

    #[test]
    fn path_remove_ext_utf8() {
        assert_eq!(remove_extension("💖.txt"), "💖");
    }

    #[test]
    fn path_remove_ext_empty() {
        assert_eq!(remove_extension(""), "");
    }

    #[test]
    fn path_basename_simple() {
        assert_eq!(basename("foo.txt"), "foo.txt");
    }

    #[test]
    fn path_basename_no_ext() {
        assert_eq!(remove_extension(basename("foo.txt")), "foo");
    }

    #[test]
    fn path_basename_dir() {
        assert_eq!(basename(&correct("dir/foo.txt")), "foo.txt");
    }

    #[test]
    fn path_basename_empty() {
        assert_eq!(basename(""), "");
    }

    #[test]
    fn path_basename_utf8() {
        assert_eq!(basename(&correct("💖/foo.txt")), "foo.txt");
        assert_eq!(basename(&correct("dir/💖.txt")), "💖.txt");
    }

    #[test]
    fn path_dirname_simple() {
        assert_eq!(dirname("foo.txt"), ".");
    }

    #[test]
    fn path_dirname_dir() {
        assert_eq!(dirname(&correct("dir/foo.txt")), "dir");
    }

    #[test]
    fn path_dirname_utf8() {
        assert_eq!(dirname(&correct("💖/foo.txt")), "💖");
        assert_eq!(dirname(&correct("dir/💖.txt")), "dir");
    }

    #[test]
    fn path_dirname_empty() {
        assert_eq!(dirname(""), ".");
    }

    #[test]
    fn path_dirname_root() {
        #[cfg(windows)]
        assert_eq!(dirname("C:\\"), "C:");
        #[cfg(windows)]
        assert_eq!(dirname("\\"), "\\");
        #[cfg(not(windows))]
        assert_eq!(dirname("/"), "/");
    }
}
