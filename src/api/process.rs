use std::{ffi::CString, str::FromStr, sync::Arc};

use honeyos_process::{
    api::{ApiModuleBuilder, ApiModuleCtx},
    ProcessManager,
};
use uuid::Uuid;
use wasm_bindgen::closure::Closure;

/// Register the process api
pub fn register_process_api(ctx: Arc<ApiModuleCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_process_get_pid
    // Returns the process id of the current process
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_get_pid",
        Closure::<dyn Fn() -> *const u8>::new(move || {
            let pid = ctx_f.pid().to_string();
            let mut memory = ctx_f.memory();
            let Some(ptr) = memory.alloc(pid.len() as u32) else {
                return std::ptr::null();
            };

            let cstring = CString::new(pid).unwrap();
            memory.write(ptr, &cstring.as_bytes());
            ptr as *const u8
        })
        .into_js_value(),
    );

    // hapi_process_spawn_subprocess
    // Spawn a wasm binary as a subprocess.
    // ### Returns
    // - The pid of the subprocess on success.
    // - NULL if the subprocess failed to spawn.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_spawn_subprocess",
        Closure::<dyn Fn(*const u8, u32) -> *const u8>::new(move |bin, bin_len| {
            let mut memory = ctx_f.memory();
            let wasm_bin = memory.read(bin as u32, bin_len);

            let mut process_manager = ProcessManager::blocking_get();
            let pid = process_manager.spawn(wasm_bin, None);

            // Return the process id
            let pid = pid.to_string();
            let Some(ptr) = memory.alloc(pid.len() as u32) else {
                return std::ptr::null();
            };

            let cstring = CString::new(pid).unwrap();
            memory.write(ptr, &cstring.as_bytes());
            ptr as *const u8
        })
        .into_js_value(),
    );

    // hapi_process_stdout
    // Return the stdout of a process
    // ### Returns
    // - The stdout of a process if successful
    // - NULL if the process does not exists, or if the memory allocation failed
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_stdout",
        Closure::<dyn Fn(*const u8, u32) -> *const u8>::new(move |id, id_len| {
            let mut memory = ctx_f.memory();
            let id = String::from_utf8_lossy(&memory.read(id as u32, id_len)).to_string();
            let Ok(id) = Uuid::from_str(&id) else {
                return std::ptr::null();
            };

            let mut process_manager = ProcessManager::blocking_get();
            let Some(process) = process_manager.process_mut(id) else {
                return std::ptr::null();
            };

            let stdout = process.stdout_mut();
            stdout.sync();
            let buffer = stdout.buffer();
            let Some(ptr) = memory.alloc(buffer.len() as u32) else {
                return std::ptr::null();
            };
            memory.write(ptr, &buffer.as_bytes());

            ptr as *const u8
        })
        .into_js_value(),
    );

    // hapi_process_alive
    // Returns true if the process is alive
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_alive",
        Closure::<dyn Fn(*const u8, u32) -> i32>::new(move |id, id_len| {
            let memory = ctx_f.memory();
            let id = String::from_utf8_lossy(&memory.read(id as u32, id_len)).to_string();
            let Ok(id) = Uuid::from_str(&id) else {
                return 0;
            };

            let process_manager = ProcessManager::blocking_get();
            let Some(process) = process_manager.process(id) else {
                return 0;
            };

            process.is_running().into()
        })
        .into_js_value(),
    );
}
