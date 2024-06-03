use std::{ffi::c_void, sync::Arc};

use honeyos_process::api::{ApiModuleBuilder, ProcessCtx};
use wasm_bindgen::closure::Closure;

/// Register the thread api
pub fn register_thread_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_spawn_thread",
        Closure::<dyn Fn(*const c_void)>::new(move |f_ptr| {
            let f_ptr = f_ptr as u32;
            honeyos_process::thread::spawn_thread(ctx_f.pid(), ctx_f.clone(), f_ptr);
        })
        .into_js_value(),
    );
}
