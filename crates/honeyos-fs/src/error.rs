use crate::FsLabel;

/// An error for honeyos-fs
#[derive(Debug)]
pub enum Error {
    NoSuchFile(String),
    NoSuchDirectory(String),
    IsFile(String),
    IsDirectory(String),
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
            Self::NoSuchFile(s) | Self::NoSuchDirectory(s) => {
                writeln!(f, "No such file or directory: {}", s)
            }
            Self::IsFile(s) => writeln!(f, "{} is a file", s),
            Self::IsDirectory(s) => writeln!(f, "{} is a directory", s),
            Self::FsManagerPoisoned => writeln!(f, "The fs writer has been poisoned"),
        }
    }
}
