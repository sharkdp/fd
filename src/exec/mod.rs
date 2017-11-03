// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

// TODO: Possible optimization could avoid pushing characters on a buffer.
mod ticket;
mod token;
mod job;
mod input;

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};

use regex::Regex;

use self::input::{basename, dirname, remove_extension};
use self::ticket::CommandTicket;
use self::token::Token;
pub use self::job::job;

/// Contains a collection of `TokenizedArgument`s that are utilized to generate command strings.
///
/// The arguments are a representation of the supplied command template, and are meant to be coupled
/// with an input in order to generate a command. The `generate()` method will be used to generate
/// a command and obtain a ticket for executing that command.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenizedCommand {
    args: Vec<TokenizedArgument>,
}

/// Represents a single command argument.
///
/// The argument is either a collection of `Token`s including at least one placeholder variant,
/// or a fixed text.
#[derive(Clone, Debug, PartialEq)]
enum TokenizedArgument {
    Tokens(Vec<Token>),
    Text(String),
}

impl TokenizedArgument {
    pub fn generate<'a>(&'a self, path: &str) -> Cow<'a, str> {
        use self::Token::*;

        match *self {
            TokenizedArgument::Tokens(ref tokens) => {
                let mut s = String::new();
                for token in tokens {
                    match *token {
                        Basename => s += basename(path),
                        BasenameNoExt => s += remove_extension(basename(path)),
                        NoExt => s += remove_extension(path),
                        Parent => s += dirname(path),
                        Placeholder => s += path,
                        Text(ref string) => s += string,
                    }
                }
                Cow::Owned(s)
            }
            TokenizedArgument::Text(ref text) => Cow::Borrowed(text),
        }
    }
}

impl TokenizedCommand {
    pub fn new<I, S>(input: I) -> TokenizedCommand
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        lazy_static! {
            static ref PLACEHOLDER: Regex = Regex::new(r"\{(/?\.?|//)\}").unwrap();
        }

        let mut args = Vec::new();
        let mut has_placeholder = false;

        for arg in input {
            let arg = arg.as_ref();

            let mut tokens = Vec::new();
            let mut start = 0;

            for placeholder in PLACEHOLDER.find_iter(arg) {
                // Leading text before the placeholder.
                if placeholder.start() > start {
                    tokens.push(Token::Text(arg[start..placeholder.start()].to_owned()));
                }

                start = placeholder.end();

                match placeholder.as_str() {
                    "{}" => tokens.push(Token::Placeholder),
                    "{.}" => tokens.push(Token::NoExt),
                    "{/}" => tokens.push(Token::Basename),
                    "{//}" => tokens.push(Token::Parent),
                    "{/.}" => tokens.push(Token::BasenameNoExt),
                    _ => panic!("Unhandled placeholder"),
                }

                has_placeholder = true;
            }

            // Without a placeholder, the argument is just fixed text.
            if tokens.is_empty() {
                args.push(TokenizedArgument::Text(arg.to_owned()));
                continue;
            }

            if start < arg.len() {
                // Trailing text after last placeholder.
                tokens.push(Token::Text(arg[start..].to_owned()));
            }

            args.push(TokenizedArgument::Tokens(tokens));
        }

        // If a placeholder token was not supplied, append one at the end of the command.
        if !has_placeholder {
            args.push(TokenizedArgument::Tokens(vec![Token::Placeholder]));
        }

        TokenizedCommand { args: args }
    }

    /// Generates a ticket that is required to execute the generated command.
    ///
    /// Using the internal `args` field, and a supplied `input` variable, arguments will be
    /// collected in a Vec. Once all arguments have been processed, the Vec will be wrapped
    /// within a `CommandTicket`, which will be responsible for executing the command.
    pub fn generate(&self, input: &Path, out_perm: Arc<Mutex<()>>) -> CommandTicket {
        let input = input
            .strip_prefix(".")
            .unwrap_or(input)
            .to_string_lossy()
            .into_owned();

        let mut args = Vec::with_capacity(self.args.len());
        for arg in &self.args {
            args.push(arg.generate(&input));
        }

        CommandTicket::new(args, out_perm)
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenizedCommand, TokenizedArgument, Token};

    #[test]
    fn tokens() {
        let expected = TokenizedCommand {
            args: vec![
                TokenizedArgument::Text("echo".into()),
                TokenizedArgument::Text("${SHELL}:".into()),
                TokenizedArgument::Tokens(vec![Token::Placeholder]),
            ],
        };

        assert_eq!(TokenizedCommand::new(&[&"echo", &"${SHELL}:"]), expected);

        assert_eq!(
            TokenizedCommand::new(&["echo", "{.}"]),
            TokenizedCommand {
                args: vec![
                    TokenizedArgument::Text("echo".into()),
                    TokenizedArgument::Tokens(vec![Token::NoExt]),
                ],
            }
        );
    }
}
