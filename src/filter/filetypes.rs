use crate::filesystem;
use crate::walk;

use super::Filter;

/// Whether or not to show
#[derive(Clone)]
pub struct FileTypes {
    pub files: bool,
    pub directories: bool,
    pub symlinks: bool,
    pub sockets: bool,
    pub pipes: bool,
    pub executables_only: bool,
    pub empty_only: bool,
}

impl Default for FileTypes {
    fn default() -> FileTypes {
        FileTypes {
            files: false,
            directories: false,
            symlinks: false,
            sockets: false,
            pipes: false,
            executables_only: false,
            empty_only: false,
        }
    }
}

impl Filter for FileTypes {
    fn should_skip(&self, entry: &walk::DirEntry) -> bool {
        entry
            .file_type()
            .map(|ref entry_type| {
                (!self.files && entry_type.is_file())
                    || (!self.directories && entry_type.is_dir())
                    || (!self.symlinks && entry_type.is_symlink())
                    || (!self.sockets && filesystem::is_socket(*entry_type))
                    || (!self.pipes && filesystem::is_pipe(*entry_type))
                    || (self.executables_only
                        && !entry
                            .metadata()
                            .map(|m| filesystem::is_executable(&m))
                            .unwrap_or(false))
                    || (self.empty_only && !filesystem::is_empty(&entry))
                    || !(entry_type.is_file()
                        || entry_type.is_dir()
                        || entry_type.is_symlink()
                        || filesystem::is_socket(*entry_type)
                        || filesystem::is_pipe(*entry_type))
            })
            .unwrap_or(true)
    }
}
