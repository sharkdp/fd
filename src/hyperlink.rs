use crate::filesystem::absolute_path;
use std::fmt::{self, Formatter, Write};
use std::path::{Path, PathBuf};

pub(crate) struct PathUrl(PathBuf);

impl PathUrl {
    pub(crate) fn new(path: &Path) -> Option<PathUrl> {
        Some(PathUrl(absolute_path(path).ok()?))
    }
}

impl fmt::Display for PathUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "file://{}", host())?;
        let bytes = self.0.as_os_str().as_encoded_bytes();
        for &byte in bytes.iter() {
            encode(f, byte)?;
        }
        Ok(())
    }
}

fn encode(f: &mut Formatter, byte: u8) -> fmt::Result {
    // NOTE:
    // Most terminals can handle non-ascii unicode characters in a file url fine. But on some OSes (notably
    // windows), the encoded bytes of the path may not be valid UTF-8. Since we don't know if a
    // byte >= 128 is part of a valid UTF-8 encoding or not, we just percent encode any non-ascii
    // byte.
    // Percent encoding these bytes is probably safer anyway.
    match byte {
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'/' | b':' | b'-' | b'.' | b'_' | b'~' => {
            f.write_char(byte.into())
        }
        #[cfg(windows)]
        b'\\' => f.write_char('/'),
        _ => {
            write!(f, "%{byte:02X}")
        }
    }
}

#[cfg(unix)]
fn host() -> &'static str {
    use std::{env, sync::OnceLock};

    static HOSTNAME: OnceLock<String> = OnceLock::new();

    HOSTNAME
        .get_or_init(|| {
            env::var("WSL_DISTRO_NAME").map_or_else(
                |_| {
                    nix::unistd::gethostname()
                        .ok()
                        .and_then(|h| h.into_string().ok())
                        .unwrap_or_default()
                },
                |distro| format!("wsl$/{distro}"),
            )
        })
        .as_ref()
}

#[cfg(not(unix))]
const fn host() -> &'static str {
    ""
}

#[cfg(test)]
mod test {
    use super::*;

    // This allows us to test the encoding without having to worry about the host, or absolute path
    struct Encoded(&'static str);

    impl fmt::Display for Encoded {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            for byte in self.0.bytes() {
                encode(f, byte)?;
            }
            Ok(())
        }
    }

    #[test]
    fn test_unicode_encoding() {
        assert_eq!(
            Encoded("$*\x1bßé/∫😃\x07").to_string(),
            "%24%2A%1B%C3%9F%C3%A9/%E2%88%AB%F0%9F%98%83%07",
        );
    }
}
