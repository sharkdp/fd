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
mod path_tests {
    use super::{basename, dirname, remove_extension, MAIN_SEPARATOR};

    fn correct(input: &str) -> String {
        input.replace('/', &MAIN_SEPARATOR.to_string())
    }

    macro_rules! func_tests {
        ($($name:ident: $func:ident for $input:expr => $output:expr)+) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!($func(&correct($input)), correct($output));
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

    #[test]
    fn dirname_root() {
        #[cfg(windows)]
        assert_eq!(dirname("C:\\"), "C:");
        #[cfg(windows)]
        assert_eq!(dirname("\\"), "\\");
        #[cfg(not(windows))]
        assert_eq!(dirname("/"), "/");
    }
}
