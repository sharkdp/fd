use clap::ValueEnum;

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum SortKey {
    /// Sort by path
    Path,
    /// Sort by file size
    Size,
    /// Sort by creation time
    Created,
    /// Sort by modification time
    Modified,
}
