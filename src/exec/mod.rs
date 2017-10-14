// TODO: Possible optimization could avoid pushing characters on a buffer.
mod ticket;
mod token;
mod job;
mod paths;

use std::path::Path;

use self::paths::{basename, dirname, remove_extension};
use self::ticket::CommandTicket;
use self::token::Token;
pub use self::job::job;

/// Signifies that a placeholder token was found
const PLACE: u8 = 1;

/// Signifies that the '{' character was found.
const OPEN: u8 = 2;

/// Contains a collection of `Token`'s that are utilized to generate command strings.
///
/// The tokens are a represntation of the supplied command template, and are meant to be coupled
/// with an input in order to generate a command. The `generate()` method will be used to
/// generate a command and obtain a ticket for executing that command.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenizedCommand {
    pub tokens: Vec<Token>,
}

impl TokenizedCommand {
    pub fn new(input: &str) -> TokenizedCommand {
        let mut tokens = Vec::new();
        let mut start = 0;
        let mut flags = 0;
        let mut chars = input.char_indices();
        let mut text = String::new();

        while let Some((id, character)) = chars.next() {
            match character {
                // Backslashes are useful in cases where we want to use the '{' character
                // without having all occurrences of it to collect placeholder tokens.
                '\\' => {
                    if let Some((_, nchar)) = chars.next() {
                        if nchar != '{' {
                            text.push(character);
                        }
                        text.push(nchar);
                    }
                }
                // When a raw '{' is discovered, we will note it's position, and use that for a
                // later comparison against valid placeholder tokens.
                '{' if flags & OPEN == 0 => {
                    flags |= OPEN;
                    start = id;
                    if !text.is_empty() {
                        append(&mut tokens, &text);
                        text.clear();
                    }
                }
                // If the `OPEN` bit is set, we will compare the contents between the discovered
                // '{' and '}' characters against a list of valid tokens, then pushing the
                // corresponding token onto the `tokens` vector.
                '}' if flags & OPEN != 0 => {
                    flags ^= OPEN;
                    match &input[start + 1..id] {
                        "" => tokens.push(Token::Placeholder),
                        "." => tokens.push(Token::NoExt),
                        "/" => tokens.push(Token::Basename),
                        "//" => tokens.push(Token::Parent),
                        "/." => tokens.push(Token::BasenameNoExt),
                        _ => {
                            append(&mut tokens, &input[start..id + 1]);
                            continue;
                        }
                    }
                    flags |= PLACE;
                }
                // We aren't collecting characters for a text string if the `OPEN` bit is set.
                _ if flags & OPEN != 0 => (),
                // Push the character onto the text buffer
                _ => text.push(character),
            }
        }

        // Take care of any stragglers left behind.
        if !text.is_empty() {
            append(&mut tokens, &text);
        }

        // If a placeholder token was not supplied, append one at the end of the command.
        if flags & PLACE == 0 {
            append(&mut tokens, " ");
            tokens.push(Token::Placeholder)
        }

        TokenizedCommand { tokens: tokens }
    }

    /// Generates a ticket that is required to execute the generated command.
    ///
    /// Using the internal `tokens` field, and a supplied `input` variable, commands will be
    /// written into the `command` buffer. Once all tokens have been processed, the mutable
    /// reference of the `command` will be wrapped within a `CommandTicket`, which will be
    /// responsible for executing the command and clearing the buffer.
    pub fn generate<'a>(&self, command: &'a mut String, input: &Path) -> CommandTicket<'a> {
        for token in &self.tokens {
            match *token {
                Token::Basename => *command += basename(&input.to_string_lossy()),
                Token::BasenameNoExt => {
                    *command += remove_extension(basename(&input.to_string_lossy()))
                }
                Token::NoExt => *command += remove_extension(&input.to_string_lossy()),
                Token::Parent => *command += dirname(&input.to_string_lossy()),
                Token::Placeholder => *command += &input.to_string_lossy(),
                Token::Text(ref string) => *command += string,
            }
        }

        CommandTicket::new(command)
    }
}

/// If the last token is a text token, append to that token. Otherwise, create a new token.
fn append(tokens: &mut Vec<Token>, elem: &str) {
    // Useful to avoid a borrowing issue with the tokens vector.
    let mut append_text = false;

    // If the last token is a `Text` token, simply the `elem` at the end.
    match tokens.last_mut() {
        Some(&mut Token::Text(ref mut string)) => *string += elem,
        _ => append_text = true,
    };

    // Otherwise, we will need to add a new `Text` token that contains the `elem`
    if append_text {
        tokens.push(Token::Text(String::from(elem)));
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenizedCommand, Token};

    #[test]
    fn tokens() {
        let expected = TokenizedCommand {
            tokens: vec![Token::Text("echo ${SHELL}: ".into()), Token::Placeholder],
        };

        assert_eq!(TokenizedCommand::new("echo $\\{SHELL}: {}"), expected);
        assert_eq!(TokenizedCommand::new("echo ${SHELL}:"), expected);
        assert_eq!(
            TokenizedCommand::new("echo {.}"),
            TokenizedCommand { tokens: vec![Token::Text("echo ".into()), Token::NoExt] }
        );
    }
}
