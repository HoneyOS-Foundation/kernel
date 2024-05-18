use hashbrown::HashMap;
use uuid::Uuid;

use crate::{
    error::Error,
    file::{Directory, File},
    util::normalize_path,
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

        let final_part = dir_parts.split_off(dir_parts.len());
        let final_part = final_part.first().unwrap();

        let mut current_dir = Option::<Uuid>::None;
        let mut current_path = String::new();
        for part in &dir_parts {
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

        let file_part = dir_parts.split_off(dir_parts.len());
        let file_part = file_part.first().unwrap();

        let mut current_dir = Option::<Uuid>::None;
        let mut current_path = String::new();
        for part in &dir_parts {
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
    pub fn get_dir(&self, path: &str) -> Result<Uuid, Error> {
        let normalized_path = normalize_path(path);
        let parts = normalized_path.split("/");

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
    pub fn get_file(&self, path: &str) -> Result<Uuid, Error> {
        let normalized_path = normalize_path(path);
        let parts = normalized_path.split("/");

        let mut dir_parts = parts.collect::<Vec<_>>();
        let mut dir = Option::<Uuid>::None;

        let file_part = dir_parts.split_off(dir_parts.len());
        let file_part = file_part.first().unwrap();

        // If the path contains a directory part, find the directory
        if dir_parts.len() > 1 {
            dir = Some(self.get_dir(&normalized_path)?);
        }

        let Some((id, _)) = self
            .files
            .iter()
            .find(|(_, file)| file.dir == dir && file.name == *file_part)
        else {
            return Err(Error::NoSuchFile(path.to_owned()));
        };

        Ok(*id)
    }
}
