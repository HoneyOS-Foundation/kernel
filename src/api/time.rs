use std::sync::Arc;

use honeyos_process::context::{ApiModuleBuilder, ProcessCtx};
use wasm_bindgen::closure::Closure;
use web_sys::js_sys::Date;

/// Register the time api
pub fn register_time_api(_: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_time_since_unix_epoch
    // Get the time in seconds since the start of the unix epoch
    builder.register(
        "hapi_time_since_unix_epoch",
        Closure::<dyn Fn() -> f64>::new(move || {
            let now = Date::now();
            (now as f64) / 1_000.0
        })
        .into_js_value(),
    );

    // hapi_time_since_startup
    // Get the time in seconds since the start of the process
    builder.register(
        "hapi_time_since_startup",
        Closure::<dyn Fn() -> f64>::new(move || {
            let now = web_sys::js_sys::eval("self.performance.now()")
                .unwrap()
                .unchecked_into_f64();
            (now as f64) / 1_000.0
        })
        .into_js_value(),
    );
}
