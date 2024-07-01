use std::{ffi::c_void, sync::Arc};

use honeyos_atomics::mutex::SpinMutex;
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
            let process_manager_lock = ProcessManager::get();
            let Ok(mut process_manager) = process_manager_lock.spin_lock() else {
                return;
            };
            process_manager.spawn_thread(ctx_f.pid(), f_ptr);
        })
        .into_js_value(),
    );
}
