use anyhow::Result;
use regex::{Regex, RegexBuilder};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ContextFilter {
    regex: Regex,
}

impl ContextFilter {
    pub fn from_string(input: &str) -> Result<Self> {
        Ok(ContextFilter {
            regex: RegexBuilder::new(input).build()?,
        })
    }

    pub fn matches(&self, path: &Path) -> bool {
        let Some(raw_context) = xattr::get(path, "security.selinux").unwrap_or(None) else { return false };

        let context = String::from_utf8_lossy(&raw_context);

        self.regex.is_match(&context)
    }
}
