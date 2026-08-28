use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::filesystem::strip_current_dir;

/// Removes the parent component of the path
pub fn basename(path: &Path) -> &OsStr {
    path.file_name().unwrap_or(path.as_os_str())
}

/// Removes the extension from the path
pub fn remove_extension(path: &Path) -> OsString {
    let dirname = dirname(path);
    let stem = path.file_stem().unwrap_or(path.as_os_str());

    let path = PathBuf::from(dirname).join(stem);

    strip_current_dir(&path).to_owned().into_os_string()
}

/// Returns the extension of the file, or an empty OsString.
pub fn extension(path: &Path) -> OsString {
    path.extension()
        .unwrap_or(OsStr::new(""))
        .to_owned()
}

/// Returns the extension of the basename, or an empty OsString.
pub fn basename_extension(path: &Path) -> OsString {
    path.file_name()
        .and_then(|name| Path::new(name).extension())
        .unwrap_or(OsStr::new(""))
        .to_owned()
}

/// Removes the basename from the path.
pub fn dirname(path: &Path) -> OsString {
    path.parent()
        .map(|p| {
            if p == OsStr::new("") {
                OsString::from(".")
            } else {
                p.as_os_str().to_owned()
            }
        })
        .unwrap_or_else(|| path.as_os_str().to_owned())
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use std::path::MAIN_SEPARATOR_STR;

    fn correct(input: &str) -> String {
        input.replace('/', MAIN_SEPARATOR_STR)
    }

    macro_rules! func_tests {
        ($($name:ident: $func:ident for $input:expr => $output:expr)+) => {
            $(
                #[test]
                fn $name() {
                    let input_path = PathBuf::from(&correct($input));
                    let output_string = OsString::from(correct($output));
                    assert_eq!($func(&input_path), output_string);
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

        ext_simple:     extension  for  "foo.txt"      =>  "txt"
        ext_dir:        extension  for  "dir/foo.txt"  =>  "txt"
        ext_none:       extension  for  "foo"          =>  ""
        ext_utf8:       extension  for  "💖.txt"       =>  "txt"
        ext_hidden:     extension  for  ".foo"         =>  ""

        bname_ext_simple:  basename_extension  for  "foo.txt"      =>  "txt"
        bname_ext_dir:     basename_extension  for  "dir/foo.txt"  =>  "txt"
        bname_ext_none:    basename_extension  for  "foo"          =>  ""
        bname_ext_utf8:    basename_extension  for  "dir/💖.txt"   =>  "txt"
    }

    #[test]
    #[cfg(windows)]
    fn dirname_root() {
        assert_eq!(dirname(&PathBuf::from("C:")), OsString::from("C:"));
        assert_eq!(dirname(&PathBuf::from("\\")), OsString::from("\\"));
    }

    #[test]
    #[cfg(not(windows))]
    fn dirname_root() {
        assert_eq!(dirname(&PathBuf::from("/")), OsString::from("/"));
    }
}
