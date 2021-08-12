use regex::bytes::RegexSet;

use crate::filesystem;

use super::common::Filter;

pub struct Extensions {
    extensions: Option<RegexSet>,
}

impl Extensions {
    pub fn new(extensions: Option<RegexSet>) -> Self {
        Self { extensions }
    }
}

impl Filter for Extensions {
    fn should_skip(&self, entry: &crate::walk::DirEntry) -> bool {
        self.extensions
            .as_ref()
            .map(|exts_regex| {
                entry
                    .path()
                    .file_name()
                    .map(|path_str| !exts_regex.is_match(&filesystem::osstr_to_bytes(path_str)))
                    .unwrap_or(true)
            })
            .unwrap_or_default()
    }
}
