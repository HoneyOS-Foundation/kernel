use uuid::Uuid;

use crate::FsLabel;

/// An error for honeyos-fs
#[derive(Debug)]
pub enum Error {
    NoSuchFile(String),
    NoSuchFileWithId(Uuid),
    NoSuchDirectoryWithId(Uuid),
    NoSuchDirectory(String),
    NoSuchFileInDirectory {
        file: Uuid,
        directory: Uuid,
    },
    NoSuchDirectoryInDirectory {
        child: Uuid,
        directory: Uuid,
    },
    IsFile(String),
    IsDirectory(String),
    FileAlreadyExists(String),
    DirectoryAlreadyExists(String),
    DirectoryOrphaned(Uuid),
    FileOrphaned(Uuid),
    IndexOutOfRange {
        file: Uuid,
        index: usize,
        size: usize,
    },
    LabelInUse(FsLabel),
    NoFsMounted(FsLabel),
    NotAFsLabel(String),
    NoFsLabel(String),
    FsManagerPoisoned,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFsMounted(label) => writeln!(f, "No filesystem mounted at: {:?}", label),
            Self::LabelInUse(label) => writeln!(
                f,
                "Could not register file system. Label: {:?} already in use",
                label
            ),
            Self::NotAFsLabel(s) => writeln!(f, "The string {} is not a valid fs label", s),
            Self::NoFsLabel(s) => writeln!(f, "The path: {} does not contain an fs label", s),
            Self::NoSuchFile(s) => writeln!(f, "No such file: {}", s),
            Self::NoSuchDirectory(s) => writeln!(f, "No such directory: {}", s),
            Self::NoSuchFileWithId(id) => writeln!(f, "No file with id: {}", id),
            Self::NoSuchDirectoryWithId(id) => writeln!(f, "No directory with id: {}", id),
            Self::NoSuchFileInDirectory { file, directory } => writeln!(
                f,
                "Directory {} contains no file with id: {}",
                directory, file
            ),
            Self::NoSuchDirectoryInDirectory { child, directory } => writeln!(
                f,
                "Directory {} contains no directory with id: {}",
                directory, child
            ),
            Self::IsFile(s) => writeln!(f, "{} is a file", s),
            Self::IsDirectory(s) => writeln!(f, "{} is a directory", s),
            Self::FileOrphaned(s) => writeln!(f, "File {} is orphaned", s),
            Self::DirectoryOrphaned(s) => writeln!(f, "Directory: {} is orphaned", s),
            Self::FileAlreadyExists(file) => {
                writeln!(f, "File \"{}\" already exists in specified path", file)
            }
            Self::DirectoryAlreadyExists(directory) => {
                writeln!(
                    f,
                    "Directory \"{}\" already exists in specified path",
                    directory
                )
            }
            Self::IndexOutOfRange { file, size, index } => writeln!(
                f,
                "Index {} is higher than {} bytes size of file {}",
                index, size, file
            ),
            Self::FsManagerPoisoned => writeln!(f, "The fs writer has been poisoned"),
        }
    }
}
