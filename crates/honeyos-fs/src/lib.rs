use std::{
    fmt::Display,
    str::FromStr,
    sync::{Arc, Once, RwLock},
};

use error::Error;
use fshandler::FsHandler;
use hashbrown::HashMap;
use uuid::Uuid;

pub mod error;
pub mod file;
pub mod fshandler;
pub mod fstable;
pub mod ramfs;
pub mod tests;
pub mod util;

static mut FS_MANAGER: Option<Arc<FsManager>> = None;

/// The label for a mounted file system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[rustfmt::skip]
pub enum FsLabel {
    A,B,C,D,E,
    F,G,H,I,J,
    K,L,M,N,O,
    P,Q,R,S,T,
    U,V,W,X,Y,
    Z,
}

/// The result of a file lookup
#[derive(Debug)]
pub enum FileResult {
    File(Uuid),
    Directory(Uuid),
}

/// Filesystem managers
pub struct FsManager {
    handlers: Arc<RwLock<HashMap<FsLabel, Arc<RwLock<dyn FsHandler>>>>>,
}

impl FsManager {
    /// Initialize the file manager
    pub fn init_once() {
        static SET_HOOK: Once = Once::new();
        SET_HOOK.call_once(|| unsafe {
            FS_MANAGER = Some(Arc::new(FsManager {
                handlers: Arc::new(RwLock::new(HashMap::new())),
            }));
        });
    }

    /// Get the file system
    pub fn get() -> Arc<FsManager> {
        unsafe { FS_MANAGER.clone().unwrap() }
    }

    /// Register the file system
    pub fn register_fs<T>(&self, label: FsLabel, file_system: T) -> Result<(), Error>
    where
        T: FsHandler + 'static,
    {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|_| Error::FsManagerPoisoned)?;

        if handlers.contains_key(&label) {
            return Err(Error::LabelInUse(label));
        }
        handlers.insert(label, Arc::new(RwLock::new(file_system)));
        Ok(())
    }

    /// Get a file system.
    /// Blocks until the fs is available.
    pub fn get_fs(&self, label: FsLabel) -> Result<Arc<RwLock<dyn FsHandler>>, Error> {
        loop {
            let Ok(handlers) = self.handlers.try_read() else {
                continue;
            };

            let Some(handler) = handlers.get(&label).cloned() else {
                return Err(Error::NoFsMounted(label));
            };
            return Ok(handler);
        }
    }

    /// Perform a file/directory lookup.
    /// Blocks until the fs is available.
    pub fn lookup(&self, path: &str) -> Result<FileResult, Error> {
        let label = FsLabel::extract_from_path(path)?;
        let fs = self.get_fs(label)?;
        loop {
            let Ok(fs) = fs.try_read() else {
                continue;
            };
            if let Ok(file) = fs.get_file(path) {
                return Ok(FileResult::File(file));
            }
            if let Ok(directory) = fs.get_directory(path) {
                return Ok(FileResult::Directory(directory));
            }
            return Err(Error::NoSuchFileOrDirectory(path.to_string()));
        }
    }
}

impl FsLabel {
    /// Extract the fs label from a path
    pub fn extract_from_path(path: &str) -> Result<Self, Error> {
        let (fs_label_str, _) = path.split_at(3);
        if !path.contains(':') {
            return Err(Error::NoFsLabel(path.to_owned()));
        }

        let fs_char = fs_label_str
            .get(0..1)
            .ok_or(Error::NoFsLabel(path.to_owned()))?;
        fs_char.parse()
    }
}

impl Display for FsLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl FromStr for FsLabel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "a" => Ok(Self::A),
            "b" => Ok(Self::B),
            "c" => Ok(Self::C),
            "d" => Ok(Self::D),
            "e" => Ok(Self::E),
            "f" => Ok(Self::F),
            "g" => Ok(Self::G),
            "h" => Ok(Self::H),
            "i" => Ok(Self::I),
            "j" => Ok(Self::J),
            "k" => Ok(Self::K),
            "l" => Ok(Self::L),
            "m" => Ok(Self::M),
            "n" => Ok(Self::N),
            "o" => Ok(Self::O),
            "p" => Ok(Self::P),
            "q" => Ok(Self::Q),
            "r" => Ok(Self::R),
            "s" => Ok(Self::S),
            "t" => Ok(Self::T),
            "u" => Ok(Self::U),
            "v" => Ok(Self::V),
            "w" => Ok(Self::W),
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "z" => Ok(Self::Z),
            _ => Err(Error::NotAFsLabel(s.to_owned())),
        }
    }
}
