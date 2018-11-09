// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

// TODO: Possible optimization could avoid pushing characters on a buffer.
mod command;
mod input;
mod job;
mod token;

use std::borrow::Cow;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use regex::Regex;

use self::command::execute_command;
use self::input::{basename, dirname, remove_extension};
pub use self::job::job;
use self::token::Token;

/// Represents a template that is utilized to generate command strings.
///
/// The template is meant to be coupled with an input in order to generate a command. The
/// `generate_and_execute()` method will be used to generate a command and execute it.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandTemplate {
    args: Vec<ArgumentTemplate>,
}

impl CommandTemplate {
    pub fn new<I, S>(input: I) -> CommandTemplate
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        lazy_static! {
            static ref PLACEHOLDER_PATTERN: Regex = Regex::new(r"\{(/?\.?|//)\}").unwrap();
        }

        let mut args = Vec::new();
        let mut has_placeholder = false;

        for arg in input {
            let arg = arg.as_ref();

            let mut tokens = Vec::new();
            let mut start = 0;

            for placeholder in PLACEHOLDER_PATTERN.find_iter(arg) {
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
                args.push(ArgumentTemplate::Text(arg.to_owned()));
                continue;
            }

            if start < arg.len() {
                // Trailing text after last placeholder.
                tokens.push(Token::Text(arg[start..].to_owned()));
            }

            args.push(ArgumentTemplate::Tokens(tokens));
        }

        // If a placeholder token was not supplied, append one at the end of the command.
        if !has_placeholder {
            args.push(ArgumentTemplate::Tokens(vec![Token::Placeholder]));
        }

        CommandTemplate { args }
    }

    /// Generates and executes a command.
    ///
    /// Using the internal `args` field, and a supplied `input` variable, a `Command` will be
    /// build. Once all arguments have been processed, the command is executed.
    pub fn generate_and_execute(&self, input: &Path, out_perm: Arc<Mutex<()>>) {
        let input = input
            .strip_prefix(".")
            .unwrap_or(input)
            .to_string_lossy()
            .into_owned();

        let mut cmd = Command::new(self.args[0].generate(&input).as_ref());
        for arg in &self.args[1..] {
            cmd.arg(arg.generate(&input).as_ref());
        }

        execute_command(cmd, out_perm)
    }
}

/// Represents a template for a single command argument.
///
/// The argument is either a collection of `Token`s including at least one placeholder variant, or
/// a fixed text.
#[derive(Clone, Debug, PartialEq)]
enum ArgumentTemplate {
    Tokens(Vec<Token>),
    Text(String),
}

impl ArgumentTemplate {
    pub fn generate<'a>(&'a self, path: &str) -> Cow<'a, str> {
        use self::Token::*;

        match *self {
            ArgumentTemplate::Tokens(ref tokens) => {
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
            ArgumentTemplate::Text(ref text) => Cow::Borrowed(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_with_placeholder() {
        assert_eq!(
            CommandTemplate::new(&[&"echo", &"${SHELL}:"]),
            CommandTemplate {
                args: vec![
                    ArgumentTemplate::Text("echo".into()),
                    ArgumentTemplate::Text("${SHELL}:".into()),
                    ArgumentTemplate::Tokens(vec![Token::Placeholder]),
                ],
            }
        );
    }

    #[test]
    fn tokens_with_no_extension() {
        assert_eq!(
            CommandTemplate::new(&["echo", "{.}"]),
            CommandTemplate {
                args: vec![
                    ArgumentTemplate::Text("echo".into()),
                    ArgumentTemplate::Tokens(vec![Token::NoExt]),
                ],
            }
        );
    }

    #[test]
    fn tokens_with_basename() {
        assert_eq!(
            CommandTemplate::new(&["echo", "{/}"]),
            CommandTemplate {
                args: vec![
                    ArgumentTemplate::Text("echo".into()),
                    ArgumentTemplate::Tokens(vec![Token::Basename]),
                ],
            }
        );
    }

    #[test]
    fn tokens_with_parent() {
        assert_eq!(
            CommandTemplate::new(&["echo", "{//}"]),
            CommandTemplate {
                args: vec![
                    ArgumentTemplate::Text("echo".into()),
                    ArgumentTemplate::Tokens(vec![Token::Parent]),
                ],
            }
        );
    }

    #[test]
    fn tokens_with_basename_no_extension() {
        assert_eq!(
            CommandTemplate::new(&["echo", "{/.}"]),
            CommandTemplate {
                args: vec![
                    ArgumentTemplate::Text("echo".into()),
                    ArgumentTemplate::Tokens(vec![Token::BasenameNoExt]),
                ],
            }
        );
    }

    #[test]
    fn tokens_multiple() {
        assert_eq!(
            CommandTemplate::new(&["cp", "{}", "{/.}.ext"]),
            CommandTemplate {
                args: vec![
                    ArgumentTemplate::Text("cp".into()),
                    ArgumentTemplate::Tokens(vec![Token::Placeholder]),
                    ArgumentTemplate::Tokens(vec![
                        Token::BasenameNoExt,
                        Token::Text(".ext".into())
                    ]),
                ],
            }
        );
    }
}
