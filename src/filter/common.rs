use crate::walk::DirEntry;

use super::{Extensions, FileTypes, MinDepth, RegexMatch, SkipRoot};

pub trait Filter: Send + Sync {
    /// Whether the entry should be skipped or not.
    fn should_skip(&self, entry: &DirEntry) -> bool;
}

pub enum FilterKind<'a> {
    SkipRoot(SkipRoot),
    MinDepth(MinDepth),
    RegexMatch(RegexMatch),
    Extensions(Extensions<'a>),
    FileTypes(FileTypes),
}

impl<'a> FilterKind<'a> {
    pub fn should_skip(&self, entry: &DirEntry) -> bool {
        match self {
            FilterKind::SkipRoot(f) => f.should_skip(entry),
            FilterKind::MinDepth(f) => f.should_skip(entry),
            FilterKind::RegexMatch(f) => f.should_skip(entry),
            FilterKind::Extensions(f) => f.should_skip(entry),
            FilterKind::FileTypes(f) => f.should_skip(entry),
        }
    }
}
