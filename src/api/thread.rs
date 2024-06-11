use std::{ffi::c_void, sync::Arc};

use honeyos_process::{
    context::{ApiModuleBuilder, ProcessCtx},
    ProcessManager,
};
use wasm_bindgen::closure::Closure;

/// Register the thread api
pub fn register_thread_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    let ctx_f = ctx.clone();
    // hapi_thread_spawn
    // Spawn a function pointer on a new thread
    builder.register(
        "hapi_thread_spawn",
        Closure::<dyn Fn(*const c_void)>::new(move |f_ptr| {
            let f_ptr = f_ptr as u32;
            let mut process_manager = ProcessManager::blocking_get();
            process_manager.spawn_thread(ctx_f.pid(), f_ptr);
        })
        .into_js_value(),
    );
}
