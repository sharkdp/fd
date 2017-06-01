use std::collections::HashMap;

use ansi_term::{Style, Colour};

/// Maps file extensions to ANSI colors / styles.
pub type ExtensionStyles = HashMap<String, Style>;

/// Maps filenames to ANSI colors / styles.
pub type FilenameStyles = HashMap<String, Style>;

const LS_CODES: &'static [&'static str] =
    &["no", "no", "fi", "rs", "di", "ln", "ln", "ln", "or", "mi", "pi", "pi",
      "so", "bd", "bd", "cd", "cd", "do", "ex", "lc", "lc", "rc", "rc", "ec",
      "ec", "su", "su", "sg", "sg", "st", "ow", "ow", "tw", "tw", "ca", "mh",
      "cl"];

#[derive(Debug, PartialEq)]
pub struct LsColors {
    pub directory: Style,
    pub symlink: Style,
    pub extensions: ExtensionStyles,
    pub filenames: FilenameStyles,
}

impl LsColors {
    /// Get a default LsColors structure.
    pub fn default() -> LsColors {
        LsColors {
            directory: Colour::Blue.bold(),
            symlink: Colour::Cyan.normal(),
            extensions: HashMap::new(),
            filenames: HashMap::new()
        }
    }

    /// Parse a single ANSI style like `38;5;10`.
    fn parse_style(code: &str) -> Option<Style> {
        let mut split = code.split(";");

        if let Some(first) = split.next() {
            let second = split.next();
            let third = split.next();

            let style =
                if first == "38" && second == Some("5") {
                    let n_white = 7;
                    let n = if let Some(num) = third {
                        u8::from_str_radix(num, 10).unwrap_or(n_white)
                    } else {
                        n_white
                    };

                    Colour::Fixed(n).normal()
                } else {
                    let style_s = if second.is_some() { first } else { "" };
                    let color_s = second.unwrap_or(first);

                    let color = match color_s {
                        "30" => Colour::Black,
                        "31" => Colour::Red,
                        "32" => Colour::Green,
                        "33" => Colour::Yellow,
                        "34" => Colour::Blue,
                        "35" => Colour::Purple,
                        "36" => Colour::Cyan,
                        _    => Colour::White
                    };

                    match style_s {
                        "1"  => color.bold(),
                        "01" => color.bold(),
                        "3"  => color.italic(),
                        "03" => color.italic(),
                        "4"  => color.underline(),
                        "04" => color.underline(),
                        _    => color.normal()
                    }
                };

            Some(style)
        } else {
            None
        }
    }

    /// Add a new LS_COLORS entry
    fn add_entry(&mut self, input: &str) {
        let mut parts = input.trim().split("=");
        if let Some(pattern) = parts.next() {
            if let Some(style_code) = parts.next() {
                // Ensure that the input was split into exactly two parts:
                if !parts.next().is_none() {
                    return;
                }

                if let Some(style) = LsColors::parse_style(style_code) {
                    // Try to match against one of the known codes
                    let res = LS_CODES.iter().find(|&&c| c == pattern);

                    if let Some(code) = res {
                        match code.as_ref() {
                            "di" => self.directory = style,
                            "ln" => self.symlink = style,
                            _ => return
                        }
                    } else if pattern.starts_with("*.") {
                        let extension = String::from(pattern).split_off(2);
                        self.extensions.insert(extension, style);
                    }
                    else if pattern.starts_with("*") {
                        let filename = String::from(pattern).split_off(1);
                        self.extensions.insert(filename, style);
                    } else {
                        // Unknown/corrupt pattern
                        return;
                    }
                }
            }
        }
    }

    /// Generate a `LsColors` structure from a string.
    pub fn from_string(input: &String) -> LsColors {
        let mut lscolors = LsColors::default();

        for s in input.split(":") {
            lscolors.add_entry(&s);
        }

        lscolors
    }
}

#[test]
fn test_parse_style() {
    assert_eq!(Some(Colour::Red.normal()),
               LsColors::parse_style("31"));

    assert_eq!(Some(Colour::Red.normal()),
               LsColors::parse_style("00;31"));

    assert_eq!(Some(Colour::Blue.italic()),
               LsColors::parse_style("03;34"));

    assert_eq!(Some(Colour::Cyan.bold()),
               LsColors::parse_style("01;36"));

    assert_eq!(Some(Colour::Fixed(115).normal()),
               LsColors::parse_style("38;5;115"));
}

#[test]
fn test_lscolors() {
    assert_eq!(LsColors::default(), LsColors::from_string(&String::new()));

    let result = LsColors::from_string(
        &String::from("rs=0:di=03;34:ln=01;36:*.foo=01;31:"));

    assert_eq!(Colour::Blue.italic(), result.directory);
    assert_eq!(Colour::Cyan.bold(), result.symlink);
}
