use std::{ffi::CString, sync::Arc};

use honeyos_process::context::{ApiModuleBuilder, ProcessCtx};
use wasm_bindgen::closure::Closure;
use web_sys::js_sys::JSON;

/// Register the js-console api
pub fn register_js_console_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_js_console_log_info
    // Logs a string to the js console as info
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_js_console_log_info",
        Closure::<dyn Fn(*const u8)>::new(move |ptr| {
            let memory = ctx_f.memory();
            let string = memory.read_str(ptr as u32);
            let Some(string) = string else {
                log::warn!("PID: {} - Memory: Failed to read string", ctx_f.pid());
                return;
            };
            log::info!("PID: {} - {}", ctx_f.pid(), string);
        })
        .into_js_value(),
    );

    // hapi_js_console_log_warn
    // Logs a string to the js console as a warning
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_js_console_log_warn",
        Closure::<dyn Fn(*const u8)>::new(move |ptr| {
            let memory = ctx_f.memory();
            let string = memory.read_str(ptr as u32);
            let Some(string) = string else {
                log::warn!("PID: {} - Memory: Failed to read string", ctx_f.pid());
                return;
            };
            log::warn!("PID: {} - {}", ctx_f.pid(), string);
        })
        .into_js_value(),
    );

    // hapi_js_console_log_error
    // Logs a string to the js console as an error
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_js_console_log_error",
        Closure::<dyn Fn(*const u8)>::new(move |ptr| {
            let memory = ctx_f.memory();
            let string = memory.read_str(ptr as u32);
            let Some(string) = string else {
                log::warn!("PID: {} - Memory: Failed to read string", ctx_f.pid());
                return;
            };
            log::error!("PID: {} - {}", ctx_f.pid(), string);
        })
        .into_js_value(),
    );

    // hapi_js_console_eval
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_js_console_eval",
        Closure::<dyn Fn(*const u8) -> *const u8>::new(move |ptr| {
            let string = {
                let memory = ctx_f.memory();
                let string = memory.read_str(ptr as u32);
                let Some(string) = string else {
                    return std::ptr::null();
                };
                string
            };

            // Evaluate the js code
            let result = web_sys::js_sys::eval(&string).unwrap();
            let Ok(result) = JSON::stringify(&result) else {
                return std::ptr::null();
            };
            let Some(result) = result.as_string() else {
                return std::ptr::null();
            };

            // Return the result as a string
            let mut memory = ctx_f.memory();
            let Ok(cstring) = CString::new(result) else {
                return std::ptr::null();
            };
            let bytes = cstring.as_bytes_with_nul();
            let Some(ptr) = memory.alloc(bytes.len() as u32) else {
                return std::ptr::null();
            };

            memory.write(ptr, bytes);
            ptr as *const u8
        })
        .into_js_value(),
    );
}
