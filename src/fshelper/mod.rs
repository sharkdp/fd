use std::path::{Path, PathBuf};
use std::io;

/// Get a relative path with respect to a certain base path.
/// See: https://stackoverflow.com/a/39343127/704831
pub fn path_relative_from(path: &Path, base: &Path) -> Option<PathBuf> {
    use std::path::Component;

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            Some(base.join(path))
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

pub fn path_absolute_form(path: &Path, base: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let path = path.strip_prefix(".").unwrap_or(path);
        if base.is_absolute() {
            Ok(base.join(path))
        } else {
            absolute_path(base).map(|path_buf| path_buf.join(path))
        }
    }
}

pub fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    let path_buf = path.canonicalize()?;

    #[cfg(windows)]
    let path_buf = Path::new(path_buf.as_path().to_string_lossy().trim_left_matches(
        r"\\?\",
    )).to_path_buf();

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
