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

    // hapi_process_get_cwd
    // Get the current working directory
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_get_cwd",
        Closure::<dyn Fn() -> *const u8>::new(move || {
            let cwd = ctx_f.cwd().clone();
            let mut memory = ctx_f.memory();
            let Some(ptr) = memory.alloc(cwd.len() as u32) else {
                return std::ptr::null();
            };

            let cstring = CString::new(cwd).unwrap();
            memory.write(ptr, &cstring.as_bytes());
            ptr as *const u8
        })
        .into_js_value(),
    );

    // hapi_process_set_cwd
    // Sets the current working directory for the process.
    // ### Note
    // There are no checks to see if the working directory is valid
    // ### Returns
    // - `0` On success
    // - `-1` If the path is invalid
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_set_cwd",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |dir| {
            let memory = ctx_f.memory();
            let Some(path) = memory.read_str(dir as u32) else {
                return -1;
            };

            ctx_f.set_cwd(&path);
            0
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
            let cwd = ctx_f.cwd();
            let pid = process_manager.spawn(wasm_bin, None, &cwd);

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
        Closure::<dyn Fn(*const u8) -> *const u8>::new(move |id| {
            let mut memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Some(id) = id else {
                return std::ptr::null();
            };
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
        Closure::<dyn Fn(*const u8) -> i32>::new(move |id| {
            let memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Some(id) = id else {
                return 0;
            };
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
