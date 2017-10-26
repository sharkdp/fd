// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::path::MAIN_SEPARATOR;
use std::borrow::Cow;

use shell_escape::escape;

/// A builder for efficiently generating input strings.
///
/// After choosing your required specs, the `get()` method will escape special characters found
/// in the input. Allocations will only occur if special characters are found that need to be
/// escaped.
pub struct Input<'a> {
    data: &'a str,
}

impl<'a> Input<'a> {
    /// Creates a new `Input` structure, which provides access to command-building
    /// primitives, such as `basename()` and `dirname()`.
    pub fn new(data: &'a str) -> Input<'a> {
        Input { data }
    }

    /// Removes the parent component of the path
    pub fn basename(&'a mut self) -> &'a mut Self {
        let mut index = 0;
        for (id, character) in self.data.char_indices() {
            if character == MAIN_SEPARATOR {
                index = id;
            }
        }

        if index != 0 {
            self.data = &self.data[index + 1..]
        }

        self
    }

    /// Removes the extension from the path
    pub fn remove_extension(&'a mut self) -> &'a mut Self {
        let mut has_dir = false;
        let mut dir_index = 0;
        let mut ext_index = 0;

        for (id, character) in self.data.char_indices() {
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
            self.data = &self.data[0..ext_index];
        }

        self
    }

    /// Removes the basename from the path.
    pub fn dirname(&'a mut self) -> &'a mut Self {
        let mut has_dir = false;
        let mut index = 0;
        for (id, character) in self.data.char_indices() {
            if character == MAIN_SEPARATOR {
                has_dir = true;
                index = id;
            }
        }

        self.data = if !has_dir {
            "."
        } else if index == 0 {
            &self.data[..1]
        } else {
            &self.data[0..index]
        };

        self
    }

    pub fn get(&'a self) -> Cow<'a, str> {
        escape(Cow::Borrowed(self.data))
    }

    #[cfg(test)]
    fn get_private(&'a self) -> Cow<'a, str> {
        Cow::Borrowed(self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAIN_SEPARATOR, Input};

    fn correct(input: &str) -> String {
        let mut sep = String::new();
        sep.push(MAIN_SEPARATOR);
        input.replace('/', &sep)
    }

    #[test]
    fn path_remove_ext_simple() {
        assert_eq!(
            &Input::new("foo.txt").remove_extension().get_private(),
            "foo"
        );
    }

    #[test]
    fn path_remove_ext_dir() {
        assert_eq!(
            &Input::new(&correct("dir/foo.txt"))
                .remove_extension()
                .get_private(),
            &correct("dir/foo")
        );
    }

    #[test]
    fn path_hidden() {
        assert_eq!(&Input::new(".foo").remove_extension().get_private(), ".foo")
    }

    #[test]
    fn path_remove_ext_utf8() {
        assert_eq!(
            &Input::new("💖.txt").remove_extension().get_private(),
            "💖"
        );
    }

    #[test]
    fn path_remove_ext_empty() {
        assert_eq!(&Input::new("").remove_extension().get_private(), "");
    }

    #[test]
    fn path_basename_simple() {
        assert_eq!(&Input::new("foo.txt").basename().get_private(), "foo.txt");
    }

    #[test]
    fn path_basename_no_ext() {
        assert_eq!(
            &Input::new("foo.txt")
                .basename()
                .remove_extension()
                .get_private(),
            "foo"
        );
    }

    #[test]
    fn path_basename_dir() {
        assert_eq!(
            &Input::new(&correct("dir/foo.txt")).basename().get_private(),
            "foo.txt"
        );
    }

    #[test]
    fn path_basename_empty() {
        assert_eq!(&Input::new("").basename().get_private(), "");
    }

    #[test]
    fn path_basename_utf8() {
        assert_eq!(
            &Input::new(&correct("💖/foo.txt"))
                .basename()
                .get_private(),
            "foo.txt"
        );
        assert_eq!(
            &Input::new(&correct("dir/💖.txt"))
                .basename()
                .get_private(),
            "💖.txt"
        );
    }

    #[test]
    fn path_dirname_simple() {
        assert_eq!(&Input::new("foo.txt").dirname().get_private(), ".");
    }

    #[test]
    fn path_dirname_dir() {
        assert_eq!(
            &Input::new(&correct("dir/foo.txt")).dirname().get_private(),
            "dir"
        );
    }

    #[test]
    fn path_dirname_utf8() {
        assert_eq!(
            &Input::new(&correct("💖/foo.txt")).dirname().get_private(),
            "💖"
        );
        assert_eq!(
            &Input::new(&correct("dir/💖.txt")).dirname().get_private(),
            "dir"
        );
    }

    #[test]
    fn path_dirname_empty() {
        assert_eq!(&Input::new("").dirname().get_private(), ".");
    }

    #[test]
    fn path_dirname_root() {
        #[cfg(windows)]
        assert_eq!(&Input::new("C:\\").dirname().get_private(), "C:");
        #[cfg(windows)]
        assert_eq!(&Input::new("\\").dirname().get_private(), "\\");
        #[cfg(not(windows))]
        assert_eq!(&Input::new("/").dirname().get_private(), "/");
    }
}
