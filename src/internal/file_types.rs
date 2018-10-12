/// Whether or not to show
pub struct FileTypes {
    pub files: bool,
    pub directories: bool,
    pub symlinks: bool,
    pub executables_only: bool,
    pub empty_only: bool,
}

impl Default for FileTypes {
    fn default() -> FileTypes {
        FileTypes {
            files: false,
            directories: false,
            symlinks: false,
            executables_only: false,
            empty_only: false,
        }
    }
}
