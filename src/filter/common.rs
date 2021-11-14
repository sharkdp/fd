use crate::walk::DirEntry;

pub trait Filter: Send + Sync {
    /// Whether the entry should be skipped or not.
    fn should_skip(&self, entry: &DirEntry) -> bool;
}
