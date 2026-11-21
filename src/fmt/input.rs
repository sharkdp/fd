use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::dir_entry::starts_with_dash;
use crate::filesystem::strip_current_dir;

/// Removes the parent component of the path
pub fn basename(path: &Path) -> &OsStr {
    path.file_name().unwrap_or(path.as_os_str())
}

/// Removes the extension from the path
pub fn remove_extension(path: &Path) -> OsString {
    let dirname = dirname(path);
    let stem = path.file_stem().unwrap_or(path.as_os_str());

    let result = PathBuf::from(dirname).join(stem);
    let stripped = strip_current_dir(&result);
    if path.starts_with(".") && starts_with_dash(stripped) {
        result.into_os_string()
    } else {
        stripped.to_owned().into_os_string()
    }
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
        remove_ext_cwd:     remove_extension  for  "./foo.txt"    =>  "foo"
        remove_ext_dash:    remove_extension  for  "./-rf.jpg"    =>  "./-rf"
        remove_ext_dash_raw: remove_extension for "-rf.jpg"      =>  "-rf"
        remove_ext_dash_dir: remove_extension for "./-dir/a.jpg" =>  "./-dir/a"
        remove_ext_dash_nested: remove_extension for "dir/-rf.jpg" => "dir/-rf"
        remove_ext_dash_no_ext: remove_extension for "./-rf"     =>  "./-rf"

        basename_simple:  basename  for  "foo.txt"      =>  "foo.txt"
        basename_dir:     basename  for  "dir/foo.txt"  =>  "foo.txt"
        basename_empty:   basename  for  ""             =>  ""
        basename_utf8_0:  basename  for  "💖/foo.txt"   =>  "foo.txt"
        basename_utf8_1:  basename  for  "dir/💖.txt"   =>  "💖.txt"

        dirname_simple:  dirname  for  "foo.txt"      =>  "."
        dirname_dir:     dirname  for  "dir/foo.txt"  =>  "dir"
        dirname_utf8_0:  dirname  for  "💖/foo.txt"   =>  "💖"
        dirname_utf8_1:  dirname  for  "dir/💖.txt"   =>  "dir"
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
