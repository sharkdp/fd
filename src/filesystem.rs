use std::env;
use std::fs;
use std::io;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use normpath::PathExt;

use crate::dir_entry;

pub fn path_absolute_form(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let path = path.strip_prefix(".").unwrap_or(path);
    env::current_dir().map(|path_buf| path_buf.join(path))
}

pub fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path_buf = path_absolute_form(path)?;

    #[cfg(windows)]
    let path_buf = strip_verbatim_prefix(&path_buf);

    Ok(path_buf)
}

/// Remove the `\\?\` verbatim prefix from a path without turning it into a
/// relative path.
///
/// The prefix is dropped by rewriting the path *component by component*, so
/// that the meaning of the remaining components is preserved:
///
/// * `\\?\C:\foo`              -> `C:\foo`            (verbatim disk path)
/// * `\\?\UNC\server\share\foo` -> `\\server\share\foo` (verbatim UNC path)
///
/// Verbatim paths that cannot be expressed without the prefix (for example
/// `\\?\Volume{GUID}\foo`, where `Volume{GUID}` is a device name and not a
/// relative file name) are returned unchanged, and so are all non-verbatim
/// paths such as `\\server\share\foo` or `\\.\COM1`.
///
/// Unlike the previous string trimming, this never goes through
/// `to_string_lossy()`, so paths that are not valid UTF-8 survive unchanged.
#[cfg(windows)]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    use std::ffi::OsString;
    use std::path::{Component, Prefix};

    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path.to_path_buf();
    };

    let has_root = matches!(components.clone().next(), Some(Component::RootDir));
    let mut base = OsString::new();
    // `Prefix::Disk` / `Prefix::UNC` cannot rebuild this prefix: a `Prefix` is a
    // parse label, not the bytes of the path, and `PrefixComponent` has no stable
    // constructor. Writing it out by hand is the only way to drop `\\?\`.
    match prefix.kind() {
        Prefix::VerbatimDisk(drive) => {
            base.push(format!("{}:", drive as char));
            if has_root {
                base.push(r"\");
            }
        }
        Prefix::VerbatimUNC(server, share) => {
            base.push(r"\\");
            base.push(server);
            base.push(r"\");
            base.push(share);
            if has_root {
                base.push(r"\");
            }
        }
        // `Prefix::Verbatim(_)` (device paths like `\\?\Volume{GUID}`) and
        // `Prefix::DeviceNS(_)` cannot be written without their prefix.
        _ => return path.to_path_buf(),
    }

    if has_root {
        // The root separator is already part of `base`, and `join` would append
        // it again; skipping the root component keeps the prefix we just built
        // instead of letting `join` replace it.
        let _ = components.next();
    }
    if components.as_path().is_empty() {
        // `\\?\C:` and `\\?\UNC\server\share`: the verbatim form is the whole
        // path, and joining an empty tail would append a separator the input
        // did not have.
        return PathBuf::from(base);
    }
    Path::new(&base).join(components)
}

pub fn is_existing_directory(path: &Path) -> bool {
    // Note: we do not use `.exists()` here, as `.` always exists, even if
    // the CWD has been deleted.
    path.is_dir() && (path.file_name().is_some() || path.normalize().is_ok())
}

pub fn is_empty(entry: &dir_entry::DirEntry) -> bool {
    if let Some(file_type) = entry.file_type() {
        if file_type.is_dir() {
            if let Ok(mut entries) = fs::read_dir(entry.path()) {
                entries.next().is_none()
            } else {
                false
            }
        } else if file_type.is_file() {
            entry.metadata().map(|m| m.len() == 0).unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(any(unix, target_os = "redox"))]
pub fn is_block_device(ft: fs::FileType) -> bool {
    ft.is_block_device()
}

#[cfg(windows)]
pub fn is_block_device(_: fs::FileType) -> bool {
    false
}

#[cfg(any(unix, target_os = "redox"))]
pub fn is_char_device(ft: fs::FileType) -> bool {
    ft.is_char_device()
}

#[cfg(windows)]
pub fn is_char_device(_: fs::FileType) -> bool {
    false
}

#[cfg(any(unix, target_os = "redox"))]
pub fn is_socket(ft: fs::FileType) -> bool {
    ft.is_socket()
}

#[cfg(windows)]
pub fn is_socket(_: fs::FileType) -> bool {
    false
}

#[cfg(any(unix, target_os = "redox"))]
pub fn is_pipe(ft: fs::FileType) -> bool {
    ft.is_fifo()
}

#[cfg(windows)]
pub fn is_pipe(_: fs::FileType) -> bool {
    false
}

/// Remove the `./` prefix from a path.
pub fn strip_current_dir(path: &Path) -> &Path {
    path.strip_prefix(".").unwrap_or(path)
}

/// Default value for the path_separator, mainly for MSYS/MSYS2, which set the MSYSTEM
/// environment variable, and we set fd's path separator to '/' rather than Rust's default of '\'.
///
/// Returns Some to use a nonstandard path separator, or None to use rust's default on the target
/// platform.
pub fn default_path_separator() -> Option<String> {
    if cfg!(windows) {
        let msystem = env::var("MSYSTEM").ok()?;
        if !msystem.is_empty() {
            return Some("/".to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::strip_current_dir;
    use std::path::Path;

    #[test]
    fn strip_current_dir_basic() {
        assert_eq!(strip_current_dir(Path::new("./foo")), Path::new("foo"));
        assert_eq!(strip_current_dir(Path::new("foo")), Path::new("foo"));
        assert_eq!(
            strip_current_dir(Path::new("./foo/bar/baz")),
            Path::new("foo/bar/baz")
        );
        assert_eq!(
            strip_current_dir(Path::new("foo/bar/baz")),
            Path::new("foo/bar/baz")
        );
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::{absolute_path, strip_verbatim_prefix};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::{Path, PathBuf};

    #[test]
    fn strip_verbatim_prefix_verbatim_disk() {
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:\foo\bar")),
            Path::new(r"C:\foo\bar")
        );
        // A bare drive root keeps its root separator.
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:\")),
            Path::new(r"C:\")
        );
        // Drive-relative verbatim path (`\\?\C:`) must not gain a root.
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:")),
            Path::new(r"C:")
        );
    }

    #[test]
    fn strip_verbatim_prefix_verbatim_unc() {
        let stripped = strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\foo"));
        assert_eq!(stripped, Path::new(r"\\server\share\foo"));
        // The whole point of the bug: the result must stay absolute.
        assert!(stripped.is_absolute());
        assert_eq!(
            stripped.components().next(),
            Path::new(r"\\server\share\foo").components().next()
        );

        // The shape produced by the old `trim_start_matches(r"\\?\")` code.
        assert_ne!(stripped, Path::new(r"UNC\server\share\foo"));
        assert_ne!(stripped, Path::new(r"\server\share\foo"));
    }

    #[test]
    fn strip_verbatim_prefix_verbatim_unc_without_trailing_path() {
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share")),
            Path::new(r"\\server\share")
        );
        // ... and keeps the exact bytes: joining an empty tail would have
        // appended a separator the input did not have.
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share"))
                .as_os_str()
                .as_encoded_bytes(),
            Path::new(r"\\server\share").as_os_str().as_encoded_bytes()
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\")),
            Path::new(r"\\server\share\")
        );
    }

    #[test]
    fn strip_verbatim_prefix_leaves_other_paths_alone() {
        // Verbatim device paths cannot be expressed without the prefix.
        let volume = Path::new(r"\\?\Volume{2b1e7a6b-1f2c-4c66-8c47-9b0e0e2f3a11}\foo");
        assert_eq!(strip_verbatim_prefix(volume), volume);
        let pipe = Path::new(r"\\?\pipe\fd");
        assert_eq!(strip_verbatim_prefix(pipe), pipe);

        // Non-verbatim paths are untouched.
        for path in [
            r"C:\foo\bar",
            r"\\server\share\foo",
            r"\\.\COM1",
            r"C:",
            r"foo\bar",
        ] {
            assert_eq!(strip_verbatim_prefix(Path::new(path)), Path::new(path));
        }
    }

    #[test]
    fn absolute_path_keeps_unc_absolute() {
        assert_eq!(
            absolute_path(Path::new(r"\\?\UNC\server\share\foo")).unwrap(),
            Path::new(r"\\server\share\foo")
        );
        assert_eq!(
            absolute_path(Path::new(r"\\?\C:\foo")).unwrap(),
            Path::new(r"C:\foo")
        );
        assert_eq!(
            absolute_path(Path::new(r"\\server\share\foo")).unwrap(),
            Path::new(r"\\server\share\foo")
        );
    }

    #[test]
    fn absolute_path_preserves_non_utf8_paths() {
        // A lone surrogate is not valid UTF-8 but is a valid UTF-16 code unit,
        // and therefore a valid Windows path component. `to_string_lossy()`
        // used to replace it with U+FFFD.
        let mut wide: Vec<u16> = r"\\?\D:\".encode_utf16().collect();
        wide.push(0xD800);
        wide.extend("x".encode_utf16());
        let path = PathBuf::from(OsString::from_wide(&wide));

        let expected = {
            let mut wide: Vec<u16> = r"D:\".encode_utf16().collect();
            wide.push(0xD800);
            wide.extend("x".encode_utf16());
            PathBuf::from(OsString::from_wide(&wide))
        };
        assert_eq!(absolute_path(&path).unwrap(), expected);
        assert_eq!(
            absolute_path(&path).unwrap().as_os_str().as_encoded_bytes(),
            expected.as_os_str().as_encoded_bytes()
        );
    }
}
