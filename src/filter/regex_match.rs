use std::{borrow::Cow, ffi::OsStr, sync::Arc};

use regex::bytes::Regex;

use crate::filesystem;

use super::common::Filter;

pub struct RegexMatch {
    pattern: Arc<Regex>,
    search_full_path: bool,
}

impl RegexMatch {
    pub fn new(pattern: Arc<Regex>, search_full_path: bool) -> Self {
        Self {
            pattern,
            search_full_path,
        }
    }
}

impl Filter for RegexMatch {
    fn should_skip(&self, entry: &crate::walk::DirEntry) -> bool {
        let entry_path = entry.path();

        let search_str: Cow<OsStr> = if self.search_full_path {
            let path_abs_buf = filesystem::path_absolute_form(entry_path)
                .expect("Retrieving absolute path succeeds");
            Cow::Owned(path_abs_buf.as_os_str().to_os_string())
        } else {
            match entry_path.file_name() {
                Some(filename) => Cow::Borrowed(filename),
                None => unreachable!(
                    "Encountered file system entry without a file name. This should only \
                     happen for paths like 'foo/bar/..' or '/' which are not supposed to \
                     appear in a file system traversal."
                ),
            }
        };

        !self
            .pattern
            .is_match(&filesystem::osstr_to_bytes(search_str.as_ref()))
    }
}
