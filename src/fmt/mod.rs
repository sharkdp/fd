mod input;

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::fs::Metadata;
use std::path::{Component, Path, Prefix};

use jiff::{Timestamp, tz::TimeZone};

use aho_corasick::AhoCorasick;

use self::input::{basename, dirname, remove_extension};

/// Designates what should be written to a buffer
///
/// Each `Token` contains either text, or a placeholder variant, which will be used to generate
/// commands after all tokens for a given command template have been collected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Placeholder,
    Basename,
    Parent,
    NoExt,
    BasenameNoExt,
    Type,
    Size,
    Name,
    Path,
    Modified,
    Text(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Token::Placeholder => f.write_str("{}")?,
            Token::Basename => f.write_str("{/}")?,
            Token::Parent => f.write_str("{//}")?,
            Token::NoExt => f.write_str("{.}")?,
            Token::BasenameNoExt => f.write_str("{/.}")?,
            Token::Type => f.write_str("%y")?,
            Token::Size => f.write_str("%s")?,
            Token::Name => f.write_str("%n")?,
            Token::Path => f.write_str("%p")?,
            Token::Modified => f.write_str("%t")?,
            Token::Text(ref string) => f.write_str(string)?,
        }
        Ok(())
    }
}

pub struct FormatContext<'a> {
    pub path: &'a Path,
    pub metadata: Option<&'a Metadata>,
}

/// A parsed format string
///
/// This is either a collection of `Token`s including at least one placeholder variant,
/// or a fixed text.
#[derive(Clone, Debug, PartialEq)]
pub enum FormatTemplate {
    Tokens(Vec<Token>),
    Text(String),
}

fn unescape_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => result.push('\t'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('0') => result.push('\0'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
}

impl FormatTemplate {
    pub fn has_tokens(&self) -> bool {
        matches!(self, FormatTemplate::Tokens(_))
    }

    pub fn parse(fmt: &str) -> Self {
        Self::parse_with_short_placeholders(fmt, true)
    }

    pub fn parse_exec(fmt: &str) -> Self {
        Self::parse_with_short_placeholders(fmt, false)
    }

    fn parse_with_short_placeholders(fmt: &str, include_short_placeholders: bool) -> Self {
        // NOTE: we assume that { and } have the same length
        const BRACE_LEN: usize = '{'.len_utf8();
        let mut tokens = Vec::new();
        let mut remaining = fmt;
        let mut buf = String::new();
        let patterns = if include_short_placeholders {
            vec![
                "{{", "}}", "{}", "{/}", "{//}", "{.}", "{/.}", "%y", "%s", "%n", "%p", "%t",
            ]
        } else {
            vec!["{{", "}}", "{}", "{/}", "{//}", "{.}", "{/.}"]
        };
        let placeholders = AhoCorasick::new(patterns).unwrap();
        while let Some(m) = placeholders.find(remaining) {
            match m.pattern().as_u32() {
                0 | 1 => {
                    // we found an escaped {{ or }}, so add
                    // everything up to the first char to the buffer
                    // then skip the second one.
                    buf += &remaining[..m.start() + BRACE_LEN];
                    remaining = &remaining[m.end()..];
                }
                id if !remaining[m.end()..].starts_with('}') => {
                    buf += &remaining[..m.start()];
                    if !buf.is_empty() {
                        tokens.push(Token::Text(std::mem::take(&mut buf)));
                    }
                    tokens.push(token_from_pattern_id(id));
                    remaining = &remaining[m.end()..];
                }
                _ => {
                    // We got a normal pattern, but the final "}"
                    // is escaped, so add up to that to the buffer, then
                    // skip the final }
                    buf += &remaining[..m.end()];
                    remaining = &remaining[m.end() + BRACE_LEN..];
                }
            }
        }
        // Add the rest of the string to the buffer, and add the final buffer to the tokens
        if !remaining.is_empty() {
            buf += remaining;
        }
        if tokens.is_empty() {
            // No placeholders were found, so just return the text
            return FormatTemplate::Text(unescape_text(&buf));
        }
        // Add final text segment
        if !buf.is_empty() {
            tokens.push(Token::Text(unescape_text(&buf)));
        }
        debug_assert!(!tokens.is_empty());
        FormatTemplate::Tokens(tokens)
    }

    fn file_type(metadata: Option<&Metadata>) -> &'static str {
        let Some(metadata) = metadata else {
            return "unknown";
        };

        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_dir() {
            "dir"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        }
    }

    /// Generate a result string from this template. If path_separator is Some, then it will replace
    /// the path separator in all placeholder tokens. Fixed text and tokens are not affected by
    /// path separator substitution.
    pub fn generate(&self, path: impl AsRef<Path>, path_separator: Option<&str>) -> OsString {
        let path = path.as_ref();
        let metadata = std::fs::symlink_metadata(path).ok();
        self.generate_with_context(
            FormatContext {
                path,
                metadata: metadata.as_ref(),
            },
            path_separator,
        )
    }

    pub fn generate_with_context(
        &self,
        context: FormatContext<'_>,
        path_separator: Option<&str>,
    ) -> OsString {
        use Token::*;
        let path = context.path;
        let metadata = context.metadata;

        match *self {
            Self::Tokens(ref tokens) => {
                let mut s = OsString::new();
                for token in tokens {
                    match token {
                        Basename => s.push(Self::replace_separator(basename(path), path_separator)),
                        BasenameNoExt => s.push(Self::replace_separator(
                            &remove_extension(basename(path).as_ref()),
                            path_separator,
                        )),
                        NoExt => s.push(Self::replace_separator(
                            &remove_extension(path),
                            path_separator,
                        )),
                        Parent => s.push(Self::replace_separator(&dirname(path), path_separator)),
                        Placeholder => {
                            s.push(Self::replace_separator(path.as_ref(), path_separator))
                        }
                        Text(string) => s.push(string),
                        Type => s.push(Self::file_type(metadata)),
                        Size => s.push(
                            metadata
                                .map_or_else(String::new, |metadata| metadata.len().to_string()),
                        ),
                        Name => s.push(Self::replace_separator(basename(path), path_separator)),
                        Path => s.push(Self::replace_separator(path.as_os_str(), path_separator)),
                        Modified => s.push(
                            metadata
                                .and_then(|metadata| metadata.modified().ok())
                                .and_then(|modified| Timestamp::try_from(modified).ok())
                                .map(|timestamp| {
                                    timestamp
                                        .to_zoned(TimeZone::system())
                                        .strftime("%Y-%m-%d %H:%M:%S %Z")
                                        .to_string()
                                })
                                .unwrap_or_default(),
                        ),
                    }
                }
                s
            }
            Self::Text(ref text) => OsString::from(text),
        }
    }

    /// Replace the path separator in the input with the custom separator string. If path_separator
    /// is None, simply return a borrowed Cow<OsStr> of the input. Otherwise, the input is
    /// interpreted as a Path and its components are iterated through and re-joined into a new
    /// OsString.
    fn replace_separator<'a>(path: &'a OsStr, path_separator: Option<&str>) -> Cow<'a, OsStr> {
        // fast-path - no replacement necessary
        if path_separator.is_none() {
            return Cow::Borrowed(path);
        }

        let path_separator = path_separator.unwrap();
        let mut out = OsString::with_capacity(path.len());
        let mut components = Path::new(path).components().peekable();

        while let Some(comp) = components.next() {
            match comp {
                // Absolute paths on Windows are tricky.  A Prefix component is usually a drive
                // letter or UNC path, and is usually followed by RootDir. There are also
                // "verbatim" prefixes beginning with "\\?\" that skip normalization. We choose to
                // ignore verbatim path prefixes here because they're very rare, might be
                // impossible to reach here, and there's no good way to deal with them. If users
                // are doing something advanced involving verbatim windows paths, they can do their
                // own output filtering with a tool like sed.
                Component::Prefix(prefix) => {
                    if let Prefix::UNC(server, share) = prefix.kind() {
                        // Prefix::UNC is a parsed version of '\\server\share'
                        out.push(path_separator);
                        out.push(path_separator);
                        out.push(server);
                        out.push(path_separator);
                        out.push(share);
                    } else {
                        // All other Windows prefix types are rendered as-is. This results in e.g. "C:" for
                        // drive letters. DeviceNS and Verbatim* prefixes won't have backslashes converted,
                        // but they're not returned by directories fd can search anyway so we don't worry
                        // about them.
                        out.push(comp.as_os_str());
                    }
                }

                // Root directory is always replaced with the custom separator.
                Component::RootDir => out.push(path_separator),

                // Everything else is joined normally, with a trailing separator if we're not last
                _ => {
                    out.push(comp.as_os_str());
                    if components.peek().is_some() {
                        out.push(path_separator);
                    }
                }
            }
        }
        Cow::Owned(out)
    }
}

// Convert the id from an aho-corasick match to the
// appropriate token
fn token_from_pattern_id(id: u32) -> Token {
    use Token::*;
    match id {
        2 => Placeholder,
        3 => Basename,
        4 => Parent,
        5 => NoExt,
        6 => BasenameNoExt,
        7 => Type,
        8 => Size,
        9 => Name,
        10 => Path,
        11 => Modified,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod fmt_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_no_placeholders() {
        let templ = FormatTemplate::parse("This string has no placeholders");
        assert_eq!(
            templ,
            FormatTemplate::Text("This string has no placeholders".into())
        );
    }

    #[test]
    fn parse_only_brace_escapes() {
        let templ = FormatTemplate::parse("This string only has escapes like {{ and }}");
        assert_eq!(
            templ,
            FormatTemplate::Text("This string only has escapes like { and }".into())
        );
    }

    #[test]
    fn parse_short_placeholders() {
        use Token::*;

        assert_eq!(
            FormatTemplate::parse("%y%s%n%p%t"),
            FormatTemplate::Tokens(vec![Type, Size, Name, Path, Modified])
        );
    }

    #[test]
    fn all_placeholders() {
        use Token::*;

        let templ = FormatTemplate::parse(
            "{{path={} \
            basename={/} \
            parent={//} \
            noExt={.} \
            basenameNoExt={/.} \
            }}",
        );
        assert_eq!(
            templ,
            FormatTemplate::Tokens(vec![
                Text("{path=".into()),
                Placeholder,
                Text(" basename=".into()),
                Basename,
                Text(" parent=".into()),
                Parent,
                Text(" noExt=".into()),
                NoExt,
                Text(" basenameNoExt=".into()),
                BasenameNoExt,
                Text(" }".into()),
            ])
        );

        let mut path = PathBuf::new();
        path.push("a");
        path.push("folder");
        path.push("file.txt");

        let expanded = templ.generate(&path, Some("/")).into_string().unwrap();

        assert_eq!(
            expanded,
            "{path=a/folder/file.txt \
            basename=file.txt \
            parent=a/folder \
            noExt=a/folder/file \
            basenameNoExt=file }"
        );
    }
}
