use std::{ffi::CString, str::FromStr, sync::Arc};

use honeyos_process::{
    context::{ApiModuleBuilder, ProcessCtx},
    ProcessManager,
};
use uuid::Uuid;
use wasm_bindgen::closure::Closure;

/// Register the process api
pub fn register_process_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_process_get_pid
    // Write the proccess id to the buffer
    // ### Safety
    // - The buffer size must be at least 37-bytes or unallocated memory will be written to.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_get_pid",
        Closure::<dyn Fn(*const u8)>::new(move |buffer| {
            let pid = ctx_f.pid().to_string();
            let mut memory = ctx_f.memory();
            let cstring = CString::new(pid).unwrap();
            memory.write(buffer as u32, &cstring.as_bytes());
        })
        .into_js_value(),
    );

    // hapi_process_get_cwd
    // Write the current working directory to the buffer
    // ### Safety
    // - The buffer size must be at least the size of `hapi_process_get_cwd_length` or unallocated memory will be written to.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_get_cwd",
        Closure::<dyn Fn(*mut u8)>::new(move |buffer| {
            let cwd = ctx_f.cwd().clone();
            let mut memory = ctx_f.memory();
            let cstring = CString::new(cwd).unwrap();
            memory.write(buffer as u32, &cstring.as_bytes());
        })
        .into_js_value(),
    );

    // hapi_process_get_cwd_length
    // Get the string length of current working directory
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_get_cwd_length",
        Closure::<dyn Fn() -> u32>::new(move || {
            let cwd = ctx_f.cwd().clone();
            cwd.len() as u32 + 1
        })
        .into_js_value(),
    );

    // hapi_process_set_cwd
    // Sets the current working directory for the process.
    // ### Note
    // There are no checks to see if the working directory is valid.
    // ### Safety
    // - The dir string must be a valid string or unallocated memory will be written to
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
    // Writes the pid of the process to the provided buffer, unless null.
    // ### Safety
    // - The provided buffer must be at least 37-bytes of length or unallocated memory will be written to
    // ### Returns
    // - `0` On success
    // - `-1` On failure
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_spawn_subprocess",
        Closure::<dyn Fn(*const u8, u32, *mut u8) -> i32>::new(move |bin, bin_len, pid_out| {
            let mut memory = ctx_f.memory();
            let wasm_bin = memory.read(bin as u32, bin_len);

            let mut process_manager = ProcessManager::blocking_get();
            let cwd = ctx_f.cwd();
            let pid = match process_manager.spawn(wasm_bin, None, &cwd) {
                Ok(pid) => pid,
                Err(e) => {
                    log::error!("Failed to spawn subprocess: {}", e);
                    return -1;
                }
            };

            if pid_out == std::ptr::null_mut() {
                return 0;
            }

            let pid = pid.to_string();
            let cstring = CString::new(pid).unwrap();
            memory.write(pid_out as u32, &cstring.as_bytes());
            0
        })
        .into_js_value(),
    );

    // hapi_process_stdout
    // Write the stoud of a process to a buffer
    // ### Safety
    // - The out buffer must be equal to `hapi_process_stdout_length` or unallocated memory will be written to.
    // - The id must be at least 37-bytes in length and a valid string or unallocated memory will be read from.
    // ### Returns
    // - `0` On success
    // - `-1` on failure
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_stdout",
        Closure::<dyn Fn(*const u8, *mut u8) -> i32>::new(move |id, out_buffer| {
            let mut memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Some(id) = id else {
                return -1;
            };
            let Ok(id) = Uuid::from_str(&id) else {
                return -1;
            };

            let mut process_manager = ProcessManager::blocking_get();
            let Some(process) = process_manager.process_mut(id) else {
                return -1;
            };

            let stdout = process.stdout_mut();
            stdout.sync();
            let buffer = stdout.buffer();
            memory.write(out_buffer as u32, &buffer.as_bytes());
            0
        })
        .into_js_value(),
    );

    // hapi_process_stdout_length
    // Returns the current length of the stdout buffer
    // ### Returns
    // - `0` On success
    // - `-1` If the id cannot be read from memory
    // ### Safety
    // - The id must be at least 37-bytes in length and a valid string or unallocated memory will be read from.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_process_stdout_length",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |id| {
            let memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Some(id) = id else {
                return -1;
            };
            let Ok(id) = Uuid::from_str(&id) else {
                return -1;
            };

            let mut process_manager = ProcessManager::blocking_get();
            let Some(process) = process_manager.process_mut(id) else {
                return -1;
            };

            let stdout = process.stdout_mut();
            stdout.sync();
            let buffer = stdout.buffer();
            buffer.len() as i32
        })
        .into_js_value(),
    );

    // hapi_process_alive
    // Returns true if the process is alive
    // ### Safety
    // - The id must be at least 37-bytes in length and a valid string or unallocated memory will be read from.
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

            process.is_alive().into()
        })
        .into_js_value(),
    );
}
