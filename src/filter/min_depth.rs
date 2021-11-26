use crate::walk::DirEntry;

use super::common::Filter;

pub struct MinDepth {
    min_depth: Option<usize>,
}

impl MinDepth {
    pub fn new(min_depth: Option<usize>) -> Self {
        Self { min_depth }
    }
}

impl Filter for MinDepth {
    fn should_skip(&self, entry: &DirEntry) -> bool {
        self.min_depth
            .map(|min_depth| entry.depth().map_or(true, |d| d < min_depth))
            .unwrap_or_default()
    }
}
