use super::Filter;

pub struct SkipRoot;

impl Filter for SkipRoot {
    fn should_skip(&self, entry: &crate::walk::DirEntry) -> bool {
        entry.depth().map(|depth| depth == 0).unwrap_or(false)
    }
}
