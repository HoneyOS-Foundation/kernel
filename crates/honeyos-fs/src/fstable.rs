use hashbrown::HashMap;
use uuid::Uuid;

use crate::{
    error::Error,
    file::{Directory, File},
    util::{self, normalize_path},
};

/// The table that stores the locations of directories and files
#[derive(Debug, Clone)]
pub struct FsTable {
    pub files: HashMap<Uuid, File>,
    pub directories: HashMap<Uuid, Directory>,
}

impl FsTable {
    /// Create the file system table
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            directories: HashMap::new(),
        }
    }

    /// Create a directory at the path, returns it's id
    pub fn create_dir(&mut self, path: &str) -> Result<Uuid, Error> {
        let normalized_path = normalize_path(path);
        let parts = normalized_path.split("/");

        let mut dir_parts = parts.collect::<Vec<_>>();

        let final_part = dir_parts.split_off(dir_parts.len() - 1);
        let final_part = final_part.first().unwrap();

        let mut current_dir = Option::<Uuid>::None;

        // If the path contains a directory part, find the directory
        if dir_parts.len() > 0 {
            let path = dir_parts.join("/");
            current_dir = Some(self.get_directory_from_path(&path)?);
        }

        // Create the directory
        let id = Uuid::new_v4();
        let dir = Directory {
            id,
            name: final_part.to_string(),
            parent: current_dir,
            files: Vec::new(),
            children: Vec::new(),
        };
        self.directories.insert(id, dir);

        // Add the directory as a child of the parent, if needed
        if let Some(parent_id) = current_dir {
            if let Some(parent) = self.directories.get_mut(&parent_id) {
                parent.children.push(id);
            }
        }

        Ok(id)
    }

    /// Create a file
    pub fn create_file(&mut self, path: &str) -> Result<Uuid, Error> {
        let normalized_path = normalize_path(path);
        let parts = normalized_path.split("/");

        let mut dir_parts = parts.collect::<Vec<_>>();

        let file_part = dir_parts.split_off(dir_parts.len() - 1);
        let file_part = file_part.first().unwrap();

        let mut current_dir = Option::<Uuid>::None;

        // If the path contains a directory part, find the directory
        if dir_parts.len() > 0 {
            let path = dir_parts.join("/");
            current_dir = Some(self.get_directory_from_path(&path)?);
        }

        // Create the directory
        let id = Uuid::new_v4();
        let file = File {
            id,
            name: file_part.to_string(),
            dir: current_dir,
        };
        self.files.insert(id, file);

        // Add the directory as a child of the parent, if needed
        if let Some(parent_id) = current_dir {
            if let Some(parent) = self.directories.get_mut(&parent_id) {
                parent.files.push(id);
            }
        }

        Ok(id)
    }

    /// Get a dir from a path
    /// Returns the directory id
    pub fn get_directory_from_path(&self, path: &str) -> Result<Uuid, Error> {
        let normalized_path = normalize_path(path);
        let parts = normalized_path.split("/");
        let parts = parts.collect::<Vec<_>>();

        let mut current_dir = Option::<Uuid>::None;
        let mut current_path = String::new();
        for part in parts {
            current_path = format!("{}/{}", current_path, path);
            let Some((id, _)) = self
                .directories
                .iter()
                .find(|(_, dir)| dir.parent == current_dir && dir.name == *part)
            else {
                return Err(Error::NoSuchDirectory(path.to_owned()));
            };

            current_dir = Some(*id);
        }

        current_dir.ok_or(Error::NoSuchDirectory(path.to_owned()))
    }

    /// Get a file from a path.
    /// Returns the file id
    pub fn get_file_from_path(&self, path: &str) -> Result<Uuid, Error> {
        let normalized_path = normalize_path(path);

        let (dir_path, name_part) = util::split_name_path(&normalized_path);

        let mut current_dir = Option::<Uuid>::None;
        // If the path contains a directory part, find the directory
        if dir_path.len() > 0 {
            current_dir = Some(self.get_directory_from_path(&dir_path)?);
        }

        let Some((id, _)) = self
            .files
            .iter()
            .find(|(_, file)| file.dir == current_dir && file.name == *name_part)
        else {
            return Err(Error::NoSuchFile(path.to_owned()));
        };

        Ok(*id)
    }

    /// Move a file to a different directory
    pub fn move_file(&mut self, file_id: Uuid, dir_id: Option<Uuid>) -> Result<(), Error> {
        let file = self
            .files
            .get_mut(&file_id)
            .ok_or(Error::NoSuchFileWithId(file_id))?;

        let org_dir_id = file.dir;
        file.dir = dir_id;

        // Remove it from the orgininal dir if the file has one
        if let Some(org_dir_id) = org_dir_id {
            // If the file has been orphaned, we can still move it.
            // This also makes it no longer orphaned.
            if let Some(dir) = self.directories.get_mut(&org_dir_id) {
                let file_index = dir.files.iter().position(|i| *i == file_id).ok_or(
                    Error::NoSuchFileInDirectory {
                        file: file_id,
                        directory: org_dir_id,
                    },
                )?;
                dir.files.remove(file_index);
            }
        }

        // If the file is in the root dir, we can skip updating the new directory
        let Some(dir_id) = dir_id else {
            return Ok(());
        };

        // Get the new dir
        let new_dir = self
            .directories
            .get_mut(&dir_id)
            .ok_or(Error::NoSuchDirectoryWithId(dir_id))?;

        // Add the file ot the new dir
        new_dir.files.push(file_id);
        Ok(())
    }

    /// Move a directory to another directory
    pub fn move_directory(&mut self, source_id: Uuid, dest_id: Option<Uuid>) -> Result<(), Error> {
        // Update the directory
        let source_dir = self
            .directories
            .get_mut(&source_id)
            .ok_or(Error::NoSuchDirectoryWithId(source_id))?;
        let org_dir_id = source_dir.parent;
        source_dir.parent = dest_id;

        // Remove it from the orgininal dir if the file has one
        if let Some(org_dir_id) = org_dir_id {
            // If the directory has been orphaned we can still move it.
            // This also makes it no longer orphaned.
            if let Some(dir) = self.directories.get_mut(&org_dir_id) {
                let source_index = dir.children.iter().position(|c| *c == source_id).ok_or(
                    Error::NoSuchDirectoryInDirectory {
                        child: source_id,
                        directory: dir.id,
                    },
                )?;
                dir.children.remove(source_index);
            }
        }

        // If the directory is in the root dir, we can skip updating the new directory
        let Some(dir_id) = dest_id else {
            return Ok(());
        };

        // Get the new dir
        let new_dir = self
            .directories
            .get_mut(&dir_id)
            .ok_or(Error::NoSuchDirectoryWithId(dir_id))?;

        new_dir.children.push(source_id);

        Ok(())
    }

    /// Get a file
    pub fn file(&self, id: Uuid) -> Result<&File, Error> {
        self.files.get(&id).ok_or(Error::NoSuchFileWithId(id))
    }

    /// Get a file
    pub fn file_mut(&mut self, id: Uuid) -> Result<&mut File, Error> {
        self.files.get_mut(&id).ok_or(Error::NoSuchFileWithId(id))
    }

    /// Get a directory
    pub fn directory(&self, id: Uuid) -> Result<&Directory, Error> {
        self.directories
            .get(&id)
            .ok_or(Error::NoSuchDirectoryWithId(id))
    }

    /// Get a directory
    pub fn directory_mut(&mut self, id: Uuid) -> Result<&mut Directory, Error> {
        self.directories
            .get_mut(&id)
            .ok_or(Error::NoSuchDirectoryWithId(id))
    }

    /// Get the path of the directory id
    pub fn get_directory_path(&self, dir_id: Uuid) -> Result<String, Error> {
        let mut path_parts = Vec::new();
        let mut current_id = Some(dir_id);

        while let Some(id) = current_id {
            if let Some(dir) = self.directories.get(&id) {
                path_parts.push(dir.name.clone());
                current_id = dir.parent;
            } else {
                return Err(Error::NoSuchDirectoryWithId(dir_id));
            }
        }

        path_parts.reverse();
        Ok(path_parts.join("/"))
    }

    /// Get the path of the file id
    pub fn get_file_path(&self, file_id: Uuid) -> Result<String, Error> {
        let file = self
            .files
            .get(&file_id)
            .ok_or(Error::NoSuchFileWithId(file_id))?;
        let dir_path = if let Some(dir_id) = file.dir {
            self.get_directory_path(dir_id)?
        } else {
            "".to_string()
        };

        if dir_path == "" {
            return Ok(file.name.clone());
        }
        Ok(format!("{}/{}", dir_path, file.name))
    }
}
