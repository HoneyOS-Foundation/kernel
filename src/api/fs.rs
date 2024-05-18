use std::sync::Arc;

use honeyos_fs::{FsLabel, FsManager};
use honeyos_process::api::{ApiModuleBuilder, ApiModuleCtx};
use wasm_bindgen::closure::Closure;

/// Register the fs api
pub fn register_fs_api(ctx: Arc<ApiModuleCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_get_file
    // Find a file at disk and return it's id
    // ### Returns
    // - -1 if the file does not exist or if the path is incorrect.
    // ### Panics
    // Panics if the filesystem is poisoned.
    // ### Safety
    // The destination buffer must be the size of a UUID (36 bytes),
    // otherwise the remaining bytes will be written to unallocated memory and can cause UB.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_get_file",
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

            // let Some(file) = fs_reader.file_descriptor(&path) else {
            //     return -1;
            // };

            // // Attempt the write to the buffer
            // memory.write(buffer as u32, file.id.to_string().as_bytes());
            0
        })
        .into_js_value(),
    );
}
