use std::{mem, str::FromStr, sync::Arc};

use honeyos_fs::{ramfs::RamFsHandler, FsLabel, FsManager};
use honeyos_process::api::{ApiModuleBuilder, ApiModuleCtx};
use wasm_bindgen::closure::Closure;

/// Register the fs api
pub fn register_fs_api(ctx: Arc<ApiModuleCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_fs_init_ramfs
    // Register a ram filesystem with the provided label.
    // ### Returns
    // - `0` On success
    // - `-1` If the label char is invalid
    // - `-2` If the label is already occupied
    // ### Panics
    // Panics if the filesystem is poisoned.
    builder.register(
        "hapi_fs_init_ramfs",
        Closure::<dyn Fn(char) -> i32>::new(move |fs_label: char| {
            let fs_manager = FsManager::get();
            let Ok(fs_label) = FsLabel::from_str(&fs_label.to_string()) else {
                return -1;
            };

            match fs_manager.register_fs(fs_label, RamFsHandler::new()) {
                Ok(_) => 0,
                Err(e) => match e {
                    honeyos_fs::error::Error::FsManagerPoisoned => {
                        panic!("The file system manager has been poisoned");
                    }
                    _ => -2,
                },
            }
        })
        .into_js_value(),
    );

    // hapi_fs_create_file
    // Create a file at the path.
    // ### Returns
    // - `0` On success
    // - `1` If the directory doesn't exist
    // - `2` If a file with the name already exists
    // ### Panics
    // Panics if the filesystem is poisoned.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_create_file",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |path| {
            let memory = ctx_f.memory();
            let path = memory.read_str(path as u32);

            let fs_manager = FsManager::get();
            let Ok(fs_label) = FsLabel::extract_from_path(&path) else {
                return -1;
            };
            let Ok(fs_manager) = fs_manager.get_fs(fs_label) else {
                return -1;
            };
            let Ok(mut fs_manager) = fs_manager.write() else {
                panic!("The file system manager has been poisoned");
            };

            match fs_manager.create_file(&path) {
                Ok(_) => 0,
                Err(e) => match e {
                    honeyos_fs::error::Error::FileAlreadyExists(_) => -2,
                    _ => -1,
                },
            }
        })
        .into_js_value(),
    );
    // hapi_fs_get_file
    // Find a file at disk and return it's id
    // ### Returns
    // - `0` On success
    // - `-1` if the file does not exist or if the path is incorrect.
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // The destination buffer must be the size of a UUID (36 bytes),
    // otherwise the remaining bytes will be written to unallocated memory and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_get_file",
        Closure::<dyn Fn(*const u8, u32, *mut u8) -> i32>::new(move |path, path_len, buffer| {
            let mut memory = ctx_f.memory();
            let path = memory.read(path as u32, path_len);
            let path = String::from_utf8_lossy(&path).to_string();

            let Ok(label) = FsLabel::extract_from_path(&path) else {
                return -1;
            };

            let fs_manager = FsManager::get();
            let Ok(fs) = fs_manager.get_fs(label) else {
                return -1;
            };

            let fs_reader = fs.read().expect(&format!(
                "The lock for file system {}:/ has been poisoned",
                label
            ));

            let Ok(file_id) = fs_reader.get_file(&path) else {
                return -1;
            };

            let file_id = file_id.to_string();
            memory.write(buffer as u32, file_id.as_bytes());
            0
        })
        .into_js_value(),
    );
}
