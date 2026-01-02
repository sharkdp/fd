mod input;
pub mod json;
mod template;

use lscolors::LsColors;

pub use self::template::{FormatTemplate, Token};

/// Description of how the results should be formatted in the output
pub enum OutputFormat {
    /// Default.
    /// Output as a plain path
    Plain,
    /// Output the path with color highlighting
    Color(LsColors),
    /// Use a custom template to format the results
    Template(FormatTemplate),
    /// Output in the json lines (jsonl, newline separated values) format
    Jsonl,
}

impl OutputFormat {
    /// Return true if the output format uses ANSI colors
    pub fn uses_color(&self) -> bool {
        matches!(self, OutputFormat::Color(_))
    }
}
