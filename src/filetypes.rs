use crate::filesystem;
use crate::walk;

/// Whether or not to show
#[derive(Default)]
pub struct FileTypes {
    pub files: bool,
    pub directories: bool,
    pub symlinks: bool,
    pub sockets: bool,
    pub pipes: bool,
    pub executables_only: bool,
    pub empty_only: bool,
}

impl FileTypes {
    pub fn should_ignore(&self, entry: &walk::DirEntry) -> bool {
        let is_symlink = entry.is_symlink();
        if let Some(ref entry_type) = entry.file_type() {
            (!self.files && entry_type.is_file() && !is_symlink)
                || (!self.directories && entry_type.is_dir() && !is_symlink)
                || (!self.symlinks && is_symlink)
                || (!self.sockets && filesystem::is_socket(*entry_type))
                || (!self.pipes && filesystem::is_pipe(*entry_type))
                || (self.executables_only
                    && !entry
                        .metadata()
                        .map(filesystem::is_executable)
                        .unwrap_or(false))
                || (self.empty_only && !filesystem::is_empty(entry))
                || !(entry_type.is_file()
                    || entry_type.is_dir()
                    || entry_type.is_symlink()
                    || filesystem::is_socket(*entry_type)
                    || filesystem::is_pipe(*entry_type))
        } else {
            true
        }
    }
}
