use crate::fshandler::FsHandler;

/// The ram file system handler
/// ### Limits
/// Due to the limitations of wasm32, the maximum size of the ramfs is 4GB.
/// This is excluding the ram occupied by the os itself.
pub struct RamFsHandler {}

impl FsHandler for RamFsHandler {}
