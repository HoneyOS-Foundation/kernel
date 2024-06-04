use std::sync::Arc;

use honeyos_display::{Display, KeyBuffer};
use honeyos_process::{
    api::{ApiModuleBuilder, ProcessCtx},
    ProcessManager,
};
use wasm_bindgen::closure::Closure;

/// Register the display api
pub fn register_display_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_display_push_stdout
    // Push stdout to the display's text-mode buffer.
    // ### Returns
    // - `0` on success
    // - `-1` if no display is registered
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_push_stdout",
        Closure::<dyn Fn() -> i32>::new(move || {
            let mut display = Display::blocking_get();

            // Get stdout from the process manager
            let mut process_manager = ProcessManager::blocking_get();
            let process = process_manager.process_mut(ctx_f.pid()).unwrap();
            let stdout = process.stdout_mut();
            // Sync stdout
            stdout.sync();

            // Send stdout to the display
            display.set_text(stdout.buffer());
            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_set_text
    // Set the text in the displays text-mode buffer.
    // ### Returns
    // - `0` on success
    // - `-1` if the string is invalid
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_set_text",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |ptr: *const u8| {
            let memory = ctx_f.memory();
            let Some(string) = &memory.read_str(ptr as u32) else {
                return -1;
            };
            let mut display = Display::blocking_get();
            display.set_text(string);
            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_buffer
    // Get the key in the displays key buffer. Clears it afterwards.
    // ### Returns
    // - `-1` or if the key buffer is empty.
    builder.register(
        "hapi_display_get_key_buffer",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let display = Display::blocking_get();
            return display.keybuffer.key;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_shift
    // Whether or not the shift key is in the key buffer
    // ### Returns
    // - `-1` if no display is registered.
    builder.register(
        "hapi_display_get_key_shift",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let display = Display::blocking_get();
            return display.keybuffer.shift as i32;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_ctrl
    // Whether or not the control key is in the key buffer
    // ### Returns
    // - `-1` if no display is registered.
    builder.register(
        "hapi_display_get_key_ctrl",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let display = Display::blocking_get();
            return display.keybuffer.ctrl as i32;
        })
        .into_js_value(),
    );

    // hapi_display_clear_key
    // Clears the key buffer of the display
    // ### Returns
    // - `-1` if no display is registered.
    builder.register(
        "hapi_display_clear_key",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let mut display = Display::blocking_get();
            display.keybuffer = KeyBuffer {
                key: -1,
                shift: false,
                ctrl: false,
            };
            return 0;
        })
        .into_js_value(),
    );
}
