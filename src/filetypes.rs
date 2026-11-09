use crate::cli::FileType;
use crate::dir_entry;
use crate::filesystem;

use faccess::PathExt;

/// Whether or not to show
#[derive(Default, Debug, PartialEq, Eq)]
pub struct FileTypes {
    pub files: bool,
    pub directories: bool,
    pub symlinks: bool,
    pub block_devices: bool,
    pub char_devices: bool,
    pub sockets: bool,
    pub pipes: bool,
    pub executables_only: bool,
    pub empty_only: bool,
}

impl FileTypes {
    pub fn from_types(types: &[FileType]) -> Self {
        let mut file_types = FileTypes::default();
        for value in types {
            match value {
                FileType::File => file_types.files = true,
                FileType::Directory => file_types.directories = true,
                FileType::Symlink => file_types.symlinks = true,
                FileType::Executable => {
                    file_types.executables_only = true;
                    file_types.files = true;
                }
                FileType::Empty => file_types.empty_only = true,
                FileType::BlockDevice => file_types.block_devices = true,
                FileType::CharDevice => file_types.char_devices = true,
                FileType::Socket => file_types.sockets = true,
                FileType::Pipe => file_types.pipes = true,
            }
        }

        // If only 'empty' was specified, search for both files and directories:
        if file_types.empty_only && !(file_types.files || file_types.directories) {
            file_types.files = true;
            file_types.directories = true;
        }

        file_types
    }

    pub fn should_ignore(&self, entry: &dir_entry::DirEntry) -> bool {
        if let Some(ref entry_type) = entry.file_type() {
            (!self.files && entry_type.is_file())
                || (!self.directories && entry_type.is_dir())
                || (!self.symlinks && entry_type.is_symlink())
                || (!self.block_devices && filesystem::is_block_device(*entry_type))
                || (!self.char_devices && filesystem::is_char_device(*entry_type))
                || (!self.sockets && filesystem::is_socket(*entry_type))
                || (!self.pipes && filesystem::is_pipe(*entry_type))
                || (self.executables_only && !entry.path().executable())
                || (self.empty_only && !filesystem::is_empty(entry))
                || !(entry_type.is_file()
                    || entry_type.is_dir()
                    || entry_type.is_symlink()
                    || filesystem::is_block_device(*entry_type)
                    || filesystem::is_char_device(*entry_type)
                    || filesystem::is_socket(*entry_type)
                    || filesystem::is_pipe(*entry_type))
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_only_implies_files_and_directories() {
        let ft = FileTypes::from_types(&[FileType::Empty]);
        assert!(ft.empty_only);
        assert!(ft.files);
        assert!(ft.directories);
    }

    #[test]
    fn empty_with_explicit_file_keeps_directories_off() {
        let ft = FileTypes::from_types(&[FileType::Empty, FileType::File]);
        assert!(ft.empty_only);
        assert!(ft.files);
        assert!(!ft.directories);
    }

    #[test]
    fn executable_implies_files() {
        let ft = FileTypes::from_types(&[FileType::Executable]);
        assert!(ft.executables_only);
        assert!(ft.files);
        assert!(!ft.directories);
    }

    #[test]
    fn union_of_multiple_types() {
        let ft = FileTypes::from_types(&[FileType::File, FileType::Symlink]);
        assert!(ft.files);
        assert!(ft.symlinks);
        assert!(!ft.directories);
        assert!(!ft.executables_only);
        assert!(!ft.empty_only);
    }
}
