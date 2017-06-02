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

    fn parse_decoration(code: &str) -> Option<fn(Colour) -> Style> {
        match code {
            "0" | "00" => Some(Colour::normal),
            "1" | "01" => Some(Colour::bold),
            "3" | "03" => Some(Colour::italic),
            "4" | "04" => Some(Colour::underline),
            _ => None
        }
    }

    /// Parse ANSI escape sequences like `38;5;10;1`.
    fn parse_style(code: &str) -> Option<Style> {
        let mut split = code.split(";");

        if let Some(first) = split.next() {
            // Try to match the first part as a text-decoration argument
            let mut decoration = LsColors::parse_decoration(first);

            let c1 = if decoration.is_none() { Some(first) } else { split.next() };
            let c2 = split.next();
            let c3 = split.next();

            let color =
                if c1 == Some("38") && c2 == Some("5") {
                    let n_white = 7;
                    let n = if let Some(num) = c3 {
                        u8::from_str_radix(num, 10).unwrap_or(n_white)
                    } else {
                        n_white
                    };

                    Colour::Fixed(n)
                } else {
                    if let Some(color_s) = c1 {
                        match color_s {
                            "30" => Colour::Black,
                            "31" => Colour::Red,
                            "32" => Colour::Green,
                            "33" => Colour::Yellow,
                            "34" => Colour::Blue,
                            "35" => Colour::Purple,
                            "36" => Colour::Cyan,
                            _    => Colour::White
                        }
                    } else {
                        Colour::White
                    }
                };

            if decoration.is_none() {
                // Try to find a decoration somewhere in the sequence
                decoration = code.split(";")
                                 .flat_map(LsColors::parse_decoration)
                                 .next();
            }

            let ansi_style = decoration.unwrap_or(Colour::normal)(color);

            Some(ansi_style)
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
                        self.filenames.insert(filename, style);
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
fn test_parse_simple() {
    assert_eq!(Some(Colour::Red.normal()),
               LsColors::parse_style("31"));
}

#[test]
fn test_parse_decoration() {
    assert_eq!(Some(Colour::Red.normal()),
               LsColors::parse_style("00;31"));

    assert_eq!(Some(Colour::Blue.italic()),
               LsColors::parse_style("03;34"));

    assert_eq!(Some(Colour::Cyan.bold()),
               LsColors::parse_style("01;36"));
}

#[test]
fn test_parse_decoration_backwards() {
    assert_eq!(Some(Colour::Blue.italic()),
               LsColors::parse_style("34;03"));

    assert_eq!(Some(Colour::Cyan.bold()),
               LsColors::parse_style("36;01"));

    assert_eq!(Some(Colour::Red.normal()),
               LsColors::parse_style("31;00"));
}

#[test]
fn test_parse_256() {
    assert_eq!(Some(Colour::Fixed(115).normal()),
               LsColors::parse_style("38;5;115"));

    assert_eq!(Some(Colour::Fixed(115).normal()),
               LsColors::parse_style("00;38;5;115"));

    assert_eq!(Some(Colour::Fixed(119).bold()),
               LsColors::parse_style("01;38;5;119"));

    assert_eq!(Some(Colour::Fixed(119).bold()),
               LsColors::parse_style("38;5;119;01"));
}

#[test]
fn test_from_string() {
    assert_eq!(LsColors::default(), LsColors::from_string(&String::new()));

    let result = LsColors::from_string(
        &String::from("rs=0:di=03;34:ln=01;36:*.foo=01;35:*README=33"));

    assert_eq!(Colour::Blue.italic(), result.directory);
    assert_eq!(Colour::Cyan.bold(), result.symlink);
    assert_eq!(Some(&Colour::Purple.bold()), result.extensions.get("foo"));
    assert_eq!(Some(&Colour::Yellow.normal()), result.filenames.get("README"));
}
