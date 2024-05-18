use uuid::Uuid;

use crate::error::Error;

/// The trait for a file system handler
pub trait FsHandler {
    /// Get a file at the path. Return it's id
    fn get_file(&self, path: &str) -> Result<Uuid, Error>;
    /// Get a directory at the path. Return it's id
    fn get_dir(&self, path: &str) -> Result<Uuid, Error>;

    /// Create a file at the path. Return it's id.
    fn create_file(&mut self, path: &str) -> Result<Uuid, Error>;
    /// Create a directory at the path. Return it's id.
    fn create_dir(&mut self, path: &str) -> Result<Uuid, Error>;

    /// Move a file to path
    fn move_file(&mut self, src: &str, dest: &str) -> Result<(), Error>;
    /// Move a directory to path
    fn move_directory(&mut self, src: &str, dest: &str) -> Result<(), Error>;
    /// Copy a file to a path.
    /// Return the copy's id
    fn copy_file(&mut self, src: &str, dest: &str) -> Result<Uuid, Error>;
    /// Copy a directory to a path.
    /// Return the copy's id
    fn copy_directory(&mut self, src: &str, dest: &str) -> Result<Uuid, Error>;

    /// Read a file
    fn read(&self, file: Uuid) -> Result<Vec<u8>, Error>;
    /// Read the size of a file
    fn file_size(&self, file: Uuid) -> Result<usize, Error>;
    /// Write data to a file
    fn write(&mut self, file: Uuid, at: usize, data: &[u8]) -> Result<(), Error>;
}
