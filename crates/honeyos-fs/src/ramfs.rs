use hashbrown::HashMap;
use uuid::Uuid;

use crate::{error::Error, fshandler::FsHandler, fstable::FsTable, util};

/// The ram file system handler
/// ### Limits
/// Due to the limitations of wasm32, the maximum size of the ramfs is 4GB.
/// This is excluding the ram occupied by the os itself.
#[derive(Debug)]
pub struct RamFsHandler {
    table: FsTable,
    data: HashMap<Uuid, Vec<u8>>,
}

impl RamFsHandler {
    pub fn new() -> Self {
        Self {
            table: FsTable::new(),
            data: HashMap::new(),
        }
    }

    /// Copy a directory recursivly
    fn copy_directory_recursive(
        &mut self,
        src_dir_id: Uuid,
        dest_dir_id: Uuid,
    ) -> Result<(), Error> {
        // Copy all files in the dir
        let src_dir = self.table.directory(src_dir_id)?;
        let files = src_dir.files.clone();
        for file_id in files {
            let file = self.table.file(file_id)?;
            let new_file_path = format!(
                "{}/{}",
                self.table.get_directory_path(dest_dir_id)?,
                file.name
            );
            let new_file_id = self.create_file(&new_file_path)?;
            let file_data = self.read(file_id)?;
            self.write(new_file_id, 0, &file_data)?;
        }

        // Copy all subdirectories recursively
        let src_dir = self.table.directory(src_dir_id)?;
        let directories = src_dir.children.clone();
        for subdir_id in directories {
            let subdir = self.table.directory(subdir_id)?;
            let new_subdir_path = format!(
                "{}/{}",
                self.table.get_directory_path(dest_dir_id)?,
                subdir.name
            );
            let new_subdir_id = self.create_dir(&new_subdir_path)?;
            self.copy_directory_recursive(subdir_id, new_subdir_id)?;
        }

        Ok(())
    }
}

impl FsHandler for RamFsHandler {
    fn get_file(&self, path: &str) -> Result<Uuid, Error> {
        self.table.get_file_from_path(path)
    }

    fn get_dir(&self, path: &str) -> Result<Uuid, Error> {
        self.table.get_directory_from_path(path)
    }

    fn create_file(&mut self, path: &str) -> Result<Uuid, Error> {
        let id = self.table.create_file(path)?;
        self.data.insert(id, Vec::new());
        Ok(id)
    }

    fn create_dir(&mut self, path: &str) -> Result<Uuid, Error> {
        self.table.create_dir(path)
    }

    fn read(&self, file: Uuid) -> Result<Vec<u8>, Error> {
        self.data
            .get(&file)
            .cloned()
            .ok_or(Error::NoSuchFileWithId(file))
    }

    fn file_size(&self, file: Uuid) -> Result<usize, Error> {
        self.data
            .get(&file)
            .map(|d| d.len())
            .ok_or(Error::NoSuchFileWithId(file))
    }

    fn write(&mut self, file: Uuid, at: usize, data: &[u8]) -> Result<(), crate::error::Error> {
        let file_data = self
            .data
            .get_mut(&file)
            .ok_or(Error::NoSuchFileWithId(file))?;

        let size = file_data.len();
        if at > size {
            return Err(Error::IndexOutOfRange {
                file,
                index: at,
                size,
            });
        }

        if at + data.len() > size {
            file_data.resize(at + data.len(), 0);
        }

        let new_size = file_data.len();
        file_data[at..at + new_size].copy_from_slice(data);
        Ok(())
    }

    fn move_file(&mut self, src: &str, dest: &str) -> Result<(), Error> {
        let file_id = self.get_file(src)?;

        let (dir_path, name_part) = util::split_name_path(dest);

        let dest_dir = if dir_path.len() > 0 {
            Some(self.table.get_directory_from_path(&dir_path)?)
        } else {
            None
        };

        self.table.move_file(file_id, dest_dir)?;

        // Rename the file
        let file = self.table.file_mut(file_id)?;
        file.name = name_part.to_string();
        Ok(())
    }

    fn move_directory(&mut self, src: &str, dest: &str) -> Result<(), Error> {
        let dir_id = self.get_dir(src)?;

        let (dir_path, name_part) = util::split_name_path(dest);

        let dest_dir = if dir_path.len() > 0 {
            Some(self.table.get_directory_from_path(&dir_path)?)
        } else {
            None
        };

        self.table.move_directory(dir_id, dest_dir)?;

        // Rename the directory
        let directory = self.table.directory_mut(dir_id)?;
        directory.name = name_part.to_string();
        Ok(())
    }

    fn copy_file(&mut self, src: &str, dest: &str) -> Result<Uuid, Error> {
        let file_id = self.get_file(src)?;

        // Create the copy
        let new_file = self.create_file(dest)?;
        let file_data = self.data.get(&file_id).cloned().expect(&format!(
            "No data associated with file in file table: {}",
            file_id
        ));
        self.write(new_file, 0, &file_data)?;
        Ok(new_file)
    }

    fn copy_directory(&mut self, src: &str, dest: &str) -> Result<Uuid, Error> {
        let src_dir_id = self.get_dir(src)?;
        let dest_dir_id = self.create_dir(dest)?;

        // Recursivly copy directory
        self.copy_directory_recursive(src_dir_id, dest_dir_id)?;
        Ok(dest_dir_id)
    }
}
