use std::borrow::Cow;
use std::fs::{FileType, Metadata};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
use std::path::{MAIN_SEPARATOR, Path};
use std::time::SystemTime;

#[cfg(unix)]
use base64::{Engine as _, prelude::BASE64_STANDARD};
use jiff::Timestamp;

pub fn output_json<W: Write>(
    out: &mut W,
    path: &Path,
    filetype: Option<FileType>,
    metadata: Option<&Metadata>,
    path_separator: &Option<String>,
) -> std::io::Result<()> {
    out.write_all(b"{")?;

    // Print the path as an object that either has a "text" key containing the
    // utf8 path, or a "bytes" key with the base64 encoded bytes of the path
    #[cfg(unix)]
    match path.to_str() {
        Some(text) => {
            let final_path: Cow<str> = if let Some(sep) = path_separator {
                text.replace(MAIN_SEPARATOR, sep).into()
            } else {
                text.into()
            };
            // NB: This assumes that rust's debug output for a string
            // is a valid JSON string. At time of writing this is the case
            // but it is possible, though unlikely, that this could change
            // in the future.
            write!(out, r#""path":{{"text":{:?}}}"#, final_path)?;
        }
        None => {
            let encoded_bytes = BASE64_STANDARD.encode(path.as_os_str().as_bytes());
            write!(out, r#""path":{{"bytes":"{}"}}"#, encoded_bytes)?;
        }
    };

    // On non-unix platforms, if the path isn't valid utf-8,
    // we don't know what kind of encoding was used, and
    // as_encoded_bytes() isn't necessarily stable between rust versions
    // so the best we can really do is a lossy string
    #[cfg(not(unix))]
    {
        let mut path = path.to_string_lossy();
        if let Some(sep) = path_separator {
            path = path.replace(MAIN_SEPARATOR, sep).into();
        }
        write!(out, r#""path":{{"text":{:?}}}"#, path)?;
    }

    // print the type of file
    let ft = match filetype {
        Some(ft) if ft.is_dir() => "directory",
        Some(ft) if ft.is_file() => "file",
        Some(ft) if ft.is_symlink() => "symlink",
        _ => "unknown",
    };
    write!(out, r#","type":"{}""#, ft)?;

    if let Some(meta) = metadata {
        // Output the mode as octal
        // We also need to mask it to just include the permission
        // bits and not the file type bits (that is handled by "type" above)
        #[cfg(unix)]
        write!(out, r#","mode":"{:o}""#, meta.mode() & 0x7777)?;

        write!(out, r#","size_bytes":{}"#, meta.len())?;

        // would it be better to do these with os-specific functions?
        if let Ok(modified) = meta.modified().map(json_timestamp) {
            write!(out, r#","modified":"{}""#, modified)?;
        }
        if let Ok(accessed) = meta.accessed().map(json_timestamp) {
            write!(out, r#","modified":"{}""#, accessed)?;
        }
        if let Ok(created) = meta.created().map(json_timestamp) {
            write!(out, r#","modified":"{}""#, created)?;
        }
    }

    out.write_all(b"}")
}

fn json_timestamp(time: SystemTime) -> Timestamp {
    // System timestamps should always be valid, so assume that we can
    // unwrap it
    // If we ever do want to handle an error here, maybe convert to either the MAX or MIN
    // timestamp depending on which side of the epoch the SystemTime is?
    Timestamp::try_from(time).expect("Invalid timestamp")
}
