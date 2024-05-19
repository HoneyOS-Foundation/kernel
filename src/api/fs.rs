use std::{ffi::CString, str::FromStr, sync::Arc};

use honeyos_fs::{ramfs::RamFsHandler, FsLabel, FsManager};
use honeyos_process::api::{ApiModuleBuilder, ApiModuleCtx};
use uuid::Uuid;
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
        Closure::<dyn Fn(u8) -> i32>::new(move |fs_label: u8| {
            let fs_manager = FsManager::get();
            let Ok(fs_label) = FsLabel::from_str(&(fs_label as char).to_string()) else {
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

    // hapi_fs_file_create
    // Create a file at the path.
    // ### Returns
    // - `0` On success
    // - `-1` If the directory doesn't exist
    // - `-2` If a file with the name already exists
    // - `-3` If the path string is invalid
    // ### Panics
    // Panics if the filesystem is poisoned.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_file_create",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |path| {
            let memory = ctx_f.memory();
            let Some(mut path) = memory.read_str(path as u32) else {
                return -2;
            };

            let fs_manager = FsManager::get();
            let Ok(fs_label) = FsLabel::extract_from_path(&path) else {
                log::error!("Failed to get fs label from path: {}", path);
                return -1;
            };
            let Ok(fs_manager) = fs_manager.get_fs(fs_label) else {
                log::info!("Failed to get fs: {}", fs_label);
                return -1;
            };
            let Ok(mut fs_manager) = fs_manager.write() else {
                panic!("The file system manager has been poisoned");
            };

            let path = path.split_off(3);

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

    // hapi_fs_file_get
    // Find a file at disk and return it's id
    // ### Returns
    // - `0` On success
    // - `-1` if the file does not exist or if the path is incorrect.
    // - `-2` If the fs label does not correspond to an active fs
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // The destination buffer must be the size of a UUID (37 bytes),
    // otherwise the remaining bytes will be written to unallocated memory and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_file_get",
        Closure::<dyn Fn(*const u8, *mut u8) -> i32>::new(move |path, buffer| {
            let mut memory = ctx_f.memory();
            let Some(mut path) = memory.read_str(path as u32) else {
                return -1;
            };

            let Ok(label) = FsLabel::extract_from_path(&path) else {
                return -2;
            };

            let fs_manager = FsManager::get();
            let Ok(fs) = fs_manager.get_fs(label) else {
                return -2;
            };

            let fs_reader = fs.read().expect(&format!(
                "The lock for file system {}:/ has been poisoned",
                label
            ));

            // Remove the label from the path
            let path = path.split_off(3);

            let Ok(file_id) = fs_reader.get_file(&path) else {
                return -1;
            };

            let file_id = CString::new(file_id.to_string()).unwrap();
            memory.write(buffer as u32, file_id.as_bytes());
            0
        })
        .into_js_value(),
    );

    // hapi_fs_directory_create
    // Create a directory at the path.
    // ### Returns
    // - `0` On success
    // - `-1` If the directory doesn't exist
    // - `-2` If a directory with the name already exists
    // - `-3` If the path string is invalid
    // ### Panics
    // Panics if the filesystem is poisoned.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_directory_create",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |path| {
            let memory = ctx_f.memory();
            let Some(mut path) = memory.read_str(path as u32) else {
                return -2;
            };

            let fs_manager = FsManager::get();
            let Ok(fs_label) = FsLabel::extract_from_path(&path) else {
                log::error!("Failed to get fs label from path: {}", path);
                return -1;
            };
            let Ok(fs_manager) = fs_manager.get_fs(fs_label) else {
                log::info!("Failed to get fs: {}", fs_label);
                return -1;
            };
            let Ok(mut fs_manager) = fs_manager.write() else {
                panic!("The file system manager has been poisoned");
            };

            let path = path.split_off(3);

            match fs_manager.create_directory(&path) {
                Ok(_) => 0,
                Err(e) => match e {
                    honeyos_fs::error::Error::DirectoryAlreadyExists(_) => -2,
                    _ => -1,
                },
            }
        })
        .into_js_value(),
    );

    // hapi_fs_directory_get
    // Find a directory at disk and return it's id
    // ### Returns
    // - `0` On success
    // - `-1` if the directory does not exist or if the path is incorrect.
    // - `-2` If the fs label does not correspond to an active fs
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // The destination buffer must be the size of a UUID (37 bytes),
    // otherwise the remaining bytes will be written to unallocated memory and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_directory_get",
        Closure::<dyn Fn(*const u8, *mut u8) -> i32>::new(move |path, buffer| {
            let mut memory = ctx_f.memory();
            let Some(mut path) = memory.read_str(path as u32) else {
                return -1;
            };

            let Ok(label) = FsLabel::extract_from_path(&path) else {
                return -2;
            };

            let fs_manager = FsManager::get();
            let Ok(fs) = fs_manager.get_fs(label) else {
                return -2;
            };

            let fs_reader = fs.read().expect(&format!(
                "The lock for file system {}:/ has been poisoned",
                label
            ));

            // Remove the label from the path
            let path = path.split_off(3);

            let Ok(dir_id) = fs_reader.get_directory(&path) else {
                return -1;
            };

            let dir_id = CString::new(dir_id.to_string()).unwrap();
            memory.write(buffer as u32, dir_id.as_bytes());
            0
        })
        .into_js_value(),
    );

    // hapi_fs_file_get
    // Find a file at disk and return it's id
    // ### Returns
    // - `0` On success
    // - `-1` if the file does not exist or if the path is incorrect.
    // - `-2` If the fs label does not correspond to an active fs
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // The destination buffer must be the size of a UUID (37 bytes),
    // otherwise the remaining bytes will be written to unallocated memory and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_file_get",
        Closure::<dyn Fn(*const u8, *mut u8) -> i32>::new(move |path, buffer| {
            let mut memory = ctx_f.memory();
            let Some(mut path) = memory.read_str(path as u32) else {
                return -1;
            };

            let Ok(label) = FsLabel::extract_from_path(&path) else {
                return -2;
            };

            let fs_manager = FsManager::get();
            let Ok(fs) = fs_manager.get_fs(label) else {
                return -2;
            };

            let fs_reader = fs.read().expect(&format!(
                "The lock for file system {}:/ has been poisoned",
                label
            ));

            // Remove the label from the path
            let path = path.split_off(3);

            let Ok(file_id) = fs_reader.get_file(&path) else {
                return -1;
            };

            let file_id = CString::new(file_id.to_string()).unwrap();
            memory.write(buffer as u32, file_id.as_bytes());
            0
        })
        .into_js_value(),
    );

    // hapi_fs_file_write
    // Write a set amount of bytes to a file
    // ### Returns
    // - `0` On success
    // - `-1` if the file does not exist or if the path is incorrect.
    // - `-2` If the fs label does not correspond to an active fs
    // - `-3` If there is not enough space
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // If the size of the buffer is smaller than the reported, unallocated memory will be read from and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_file_write",
        Closure::<dyn Fn(u8, *const u8, u32, u32, *const u8) -> i32>::new(
            move |fs_label, file_id, offset, size, buffer| {
                let memory = ctx_f.memory();
                let Some(file_id) = memory.read_str(file_id as u32) else {
                    return -1;
                };
                let Ok(file_id) = Uuid::parse_str(&file_id) else {
                    return -1;
                };
                let Ok(fs_label) = FsLabel::from_str(&(fs_label as char).to_string()) else {
                    return -2;
                };

                let fs_manager = FsManager::get();
                let Ok(fs) = fs_manager.get_fs(fs_label) else {
                    return -2;
                };
                let mut fs_writer = fs.write().expect(&format!(
                    "The lock for file system {}:/ has been poisoned",
                    fs_label
                ));

                let bytes = memory.read(buffer as u32, size);

                let Ok(_) = fs_writer.write(file_id, offset as usize, &bytes) else {
                    return -3;
                };

                0
            },
        )
        .into_js_value(),
    );

    // hapi_fs_file_read
    // Read a set amount of bytes from the file and write it to a buffer
    // ### Returns
    // - `0` On success
    // - `-1` if the file does not exist or if the path is incorrect.
    // - `-2` If the fs label does not correspond to an active fs
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // If the size of the buffer is smaller than the reported, unallocated memory will be written to and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_fs_file_read",
        Closure::<dyn Fn(u8, *const u8, u32, u32, *mut u8) -> i32>::new(
            move |fs_label, file_id, offset, size, buffer| {
                let mut memory = ctx_f.memory();
                let Some(file_id) = memory.read_str(file_id as u32) else {
                    return -1;
                };
                let Ok(file_id) = Uuid::parse_str(&file_id) else {
                    return -1;
                };
                let Ok(fs_label) = FsLabel::from_str(&(fs_label as char).to_string()) else {
                    return -2;
                };

                let fs_manager = FsManager::get();
                let Ok(fs) = fs_manager.get_fs(fs_label) else {
                    return -2;
                };
                let fs_reader = fs.read().expect(&format!(
                    "The lock for file system {}:/ has been poisoned",
                    fs_label
                ));

                // NOTE(GetAGripGal): We should probably refactor this to not load the entire file in memory each time.
                // But for now its fine.
                let Ok(bytes) = fs_reader.read(file_id) else {
                    return -1;
                };

                let slice = &bytes[offset as usize..offset as usize + size as usize];
                memory.write(buffer as u32, slice);

                0
            },
        )
        .into_js_value(),
    );
}
