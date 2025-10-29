mod command;
mod job;

use std::ffi::OsString;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Result, bail};
use argmax::Command;

use crate::exec::command::OutputBuffer;
use crate::exit_codes::{ExitCode, merge_exitcodes};
use crate::fmt::{FormatTemplate, Token};

use self::command::{execute_commands, handle_cmd_error};
pub use self::job::{batch, job};

/// Execution mode of the command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Command is executed for each search result
    OneByOne,
    /// Command is run for a batch of results at once
    Batch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandSet {
    mode: ExecutionMode,
    commands: Vec<CommandTemplate>,
}

impl CommandSet {
    pub fn new<I, T, S>(input: I) -> Result<CommandSet>
    where
        I: IntoIterator<Item = T>,
        T: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Ok(CommandSet {
            mode: ExecutionMode::OneByOne,
            commands: input
                .into_iter()
                .map(CommandTemplate::new)
                .collect::<Result<_>>()?,
        })
    }

    pub fn new_batch<I, T, S>(input: I) -> Result<CommandSet>
    where
        I: IntoIterator<Item = T>,
        T: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Ok(CommandSet {
            mode: ExecutionMode::Batch,
            commands: input
                .into_iter()
                .map(|args| {
                    let cmd = CommandTemplate::new(args)?;
                    if cmd.number_of_tokens() > 1 {
                        bail!("Only one placeholder allowed for batch commands");
                    }
                    if cmd.args[0].has_tokens() {
                        bail!("First argument of exec-batch is expected to be a fixed executable");
                    }
                    Ok(cmd)
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn in_batch_mode(&self) -> bool {
        self.mode == ExecutionMode::Batch
    }

    pub fn execute(
        &self,
        input: &Path,
        path_separator: Option<&str>,
        null_separator: bool,
        buffer_output: bool,
    ) -> ExitCode {
        let commands = self
            .commands
            .iter()
            .map(|c| c.generate(input, path_separator));
        execute_commands(commands, OutputBuffer::new(null_separator), buffer_output)
    }

    pub fn execute_batch<I>(&self, paths: I, limit: usize, path_separator: Option<&str>) -> ExitCode
    where
        I: Iterator<Item = PathBuf>,
    {
        let builders: io::Result<Vec<_>> = self
            .commands
            .iter()
            .map(|c| CommandBuilder::new(c, limit))
            .collect();

        match builders {
            Ok(mut builders) => {
                for path in paths {
                    for builder in &mut builders {
                        if let Err(e) = builder.push(&path, path_separator) {
                            return handle_cmd_error(Some(&builder.cmd), e);
                        }
                    }
                }

                for builder in &mut builders {
                    if let Err(e) = builder.finish() {
                        return handle_cmd_error(Some(&builder.cmd), e);
                    }
                }

                merge_exitcodes(builders.iter().map(|b| b.exit_code()))
            }
            Err(e) => handle_cmd_error(None, e),
        }
    }
}

/// Represents a multi-exec command as it is built.
#[derive(Debug)]
struct CommandBuilder {
    pre_args: Vec<OsString>,
    path_arg: FormatTemplate,
    post_args: Vec<OsString>,
    cmd: Command,
    count: usize,
    limit: usize,
    exit_code: ExitCode,
}

impl CommandBuilder {
    fn new(template: &CommandTemplate, limit: usize) -> io::Result<Self> {
        let mut pre_args = vec![];
        let mut path_arg = None;
        let mut post_args = vec![];

        for arg in &template.args {
            if arg.has_tokens() {
                path_arg = Some(arg.clone());
            } else if path_arg.is_none() {
                pre_args.push(arg.generate("", None));
            } else {
                post_args.push(arg.generate("", None));
            }
        }

        let cmd = Self::new_command(&pre_args)?;

        Ok(Self {
            pre_args,
            path_arg: path_arg.unwrap(),
            post_args,
            cmd,
            count: 0,
            limit,
            exit_code: ExitCode::Success,
        })
    }

    fn new_command(pre_args: &[OsString]) -> io::Result<Command> {
        let mut cmd = Command::new(&pre_args[0]);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        cmd.try_args(&pre_args[1..])?;
        Ok(cmd)
    }

    fn push(&mut self, path: &Path, separator: Option<&str>) -> io::Result<()> {
        if self.limit > 0 && self.count >= self.limit {
            self.finish()?;
        }

        let arg = self.path_arg.generate(path, separator);
        if !self
            .cmd
            .args_would_fit(iter::once(&arg).chain(&self.post_args))
        {
            self.finish()?;
        }

        self.cmd.try_arg(arg)?;
        self.count += 1;
        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.count > 0 {
            self.cmd.try_args(&self.post_args)?;
            if !self.cmd.status()?.success() {
                self.exit_code = ExitCode::GeneralError;
            }

            self.cmd = Self::new_command(&self.pre_args)?;
            self.count = 0;
        }

        Ok(())
    }

    fn exit_code(&self) -> ExitCode {
        self.exit_code
    }
}

/// Represents a template that is utilized to generate command strings.
///
/// The template is meant to be coupled with an input in order to generate a command. The
/// `generate_and_execute()` method will be used to generate a command and execute it.
#[derive(Debug, Clone, PartialEq)]
struct CommandTemplate {
    args: Vec<FormatTemplate>,
}

impl CommandTemplate {
    fn new<I, S>(input: I) -> Result<CommandTemplate>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut args = Vec::new();
        let mut has_placeholder = false;

        for arg in input {
            let arg = arg.as_ref();

            let tmpl = FormatTemplate::parse(arg);
            has_placeholder |= tmpl.has_tokens();
            args.push(tmpl);
        }

        // We need to check that we have at least one argument, because if not
        // it will try to execute each file and directory it finds.
        //
        // Sadly, clap can't currently handle this for us, see
        // https://github.com/clap-rs/clap/issues/3542
        if args.is_empty() {
            bail!("No executable provided for --exec or --exec-batch");
        }

        // If a placeholder token was not supplied, append one at the end of the command.
        if !has_placeholder {
            args.push(FormatTemplate::Tokens(vec![Token::Placeholder]));
        }

        Ok(CommandTemplate { args })
    }

    fn number_of_tokens(&self) -> usize {
        self.args.iter().filter(|arg| arg.has_tokens()).count()
    }

    /// Generates and executes a command.
    ///
    /// Using the internal `args` field, and a supplied `input` variable, a `Command` will be
    /// build.
    fn generate(&self, input: &Path, path_separator: Option<&str>) -> io::Result<Command> {
        let mut cmd = Command::new(self.args[0].generate(input, path_separator));
        for arg in &self.args[1..] {
            cmd.try_arg(arg.generate(input, path_separator))?;
        }
        Ok(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_str(template: &CommandTemplate, input: &str) -> Vec<String> {
        template
            .args
            .iter()
            .map(|arg| arg.generate(input, None).into_string().unwrap())
            .collect()
    }

    #[test]
    fn tokens_with_placeholder() {
        assert_eq!(
            CommandSet::new(vec![vec![&"echo", &"${SHELL}:"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("echo".into()),
                        FormatTemplate::Text("${SHELL}:".into()),
                        FormatTemplate::Tokens(vec![Token::Placeholder]),
                    ]
                }],
                mode: ExecutionMode::OneByOne,
            }
        );
    }

    #[test]
    fn tokens_with_no_extension() {
        assert_eq!(
            CommandSet::new(vec![vec!["echo", "{.}"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("echo".into()),
                        FormatTemplate::Tokens(vec![Token::NoExt]),
                    ],
                }],
                mode: ExecutionMode::OneByOne,
            }
        );
    }

    #[test]
    fn tokens_with_basename() {
        assert_eq!(
            CommandSet::new(vec![vec!["echo", "{/}"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("echo".into()),
                        FormatTemplate::Tokens(vec![Token::Basename]),
                    ],
                }],
                mode: ExecutionMode::OneByOne,
            }
        );
    }

    #[test]
    fn tokens_with_parent() {
        assert_eq!(
            CommandSet::new(vec![vec!["echo", "{//}"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("echo".into()),
                        FormatTemplate::Tokens(vec![Token::Parent]),
                    ],
                }],
                mode: ExecutionMode::OneByOne,
            }
        );
    }

    #[test]
    fn tokens_with_basename_no_extension() {
        assert_eq!(
            CommandSet::new(vec![vec!["echo", "{/.}"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("echo".into()),
                        FormatTemplate::Tokens(vec![Token::BasenameNoExt]),
                    ],
                }],
                mode: ExecutionMode::OneByOne,
            }
        );
    }

    #[test]
    fn tokens_with_literal_braces() {
        let template = CommandTemplate::new(vec!["{{}}", "{{", "{.}}"]).unwrap();
        assert_eq!(
            generate_str(&template, "foo"),
            vec!["{}", "{", "{.}", "foo"]
        );
    }

    #[test]
    fn tokens_with_literal_braces_and_placeholder() {
        let template = CommandTemplate::new(vec!["{{{},end}"]).unwrap();
        assert_eq!(generate_str(&template, "foo"), vec!["{foo,end}"]);
    }

    #[test]
    fn tokens_multiple() {
        assert_eq!(
            CommandSet::new(vec![vec!["cp", "{}", "{/.}.ext"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("cp".into()),
                        FormatTemplate::Tokens(vec![Token::Placeholder]),
                        FormatTemplate::Tokens(vec![
                            Token::BasenameNoExt,
                            Token::Text(".ext".into())
                        ]),
                    ],
                }],
                mode: ExecutionMode::OneByOne,
            }
        );
    }

    #[test]
    fn tokens_single_batch() {
        assert_eq!(
            CommandSet::new_batch(vec![vec!["echo", "{.}"]]).unwrap(),
            CommandSet {
                commands: vec![CommandTemplate {
                    args: vec![
                        FormatTemplate::Text("echo".into()),
                        FormatTemplate::Tokens(vec![Token::NoExt]),
                    ],
                }],
                mode: ExecutionMode::Batch,
            }
        );
    }

    #[test]
    fn tokens_multiple_batch() {
        assert!(CommandSet::new_batch(vec![vec!["echo", "{.}", "{}"]]).is_err());
    }

    #[test]
    fn template_no_args() {
        assert!(CommandTemplate::new::<Vec<_>, &'static str>(vec![]).is_err());
    }

    #[test]
    fn command_set_no_args() {
        assert!(CommandSet::new(vec![vec!["echo"], vec![]]).is_err());
    }

    #[test]
    fn generate_custom_path_separator() {
        let arg = FormatTemplate::Tokens(vec![Token::Placeholder]);
        macro_rules! check {
            ($input:expr, $expected:expr) => {
                assert_eq!(arg.generate($input, Some("#")), OsString::from($expected));
            };
        }

        check!("foo", "foo");
        check!("foo/bar", "foo#bar");
        check!("/foo/bar/baz", "#foo#bar#baz");
    }

    #[cfg(windows)]
    #[test]
    fn generate_custom_path_separator_windows() {
        let arg = FormatTemplate::Tokens(vec![Token::Placeholder]);
        macro_rules! check {
            ($input:expr, $expected:expr) => {
                assert_eq!(arg.generate($input, Some("#")), OsString::from($expected));
            };
        }

        // path starting with a drive letter
        check!(r"C:\foo\bar", "C:#foo#bar");
        // UNC path
        check!(r"\\server\share\path", "##server#share#path");
        // Drive Relative path - no separator after the colon omits the RootDir path component.
        // This is uncommon, but valid
        check!(r"C:foo\bar", "C:foo#bar");

        // forward slashes should get normalized and interpreted as separators
        check!("C:/foo/bar", "C:#foo#bar");
        check!("C:foo/bar", "C:foo#bar");

        // Rust does not interpret "//server/share" as a UNC path, but rather as a normal
        // absolute path that begins with RootDir, and the two slashes get combined together as
        // a single path separator during normalization.
        //check!("//server/share/path", "##server#share#path");
    }
}
