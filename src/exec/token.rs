use aho_corasick::AhoCorasick;
use std::fmt::{self, Display, Formatter};
use std::sync::OnceLock;

use super::ArgumentTemplate;

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
            Token::Text(ref string) => f.write_str(string)?,
        }
        Ok(())
    }
}

static PLACEHOLDERS: OnceLock<AhoCorasick> = OnceLock::new();

pub(super) fn tokenize(input: &str) -> ArgumentTemplate {
    // NOTE: we assume that { and } have the same length
    const BRACE_LEN: usize = '{'.len_utf8();
    let mut tokens = Vec::new();
    let mut remaining = input;
    let mut buf = String::new();
    let placeholders = PLACEHOLDERS.get_or_init(|| {
        AhoCorasick::new(&["{{", "}}", "{}", "{/}", "{//}", "{.}", "{/.}"]).unwrap()
    });
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
        return ArgumentTemplate::Text(buf);
    }
    // Add final text segment
    if !buf.is_empty() {
        tokens.push(Token::Text(buf));
    }
    debug_assert!(!tokens.is_empty());
    ArgumentTemplate::Tokens(tokens)
}

fn token_from_pattern_id(id: u32) -> Token {
    use Token::*;
    match id {
        2 => Placeholder,
        3 => Basename,
        4 => Parent,
        5 => NoExt,
        6 => BasenameNoExt,
        _ => unreachable!(),
    }
}
