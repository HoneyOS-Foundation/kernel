use std::sync::Arc;

use honeyos_display::{error::Error, Display, KeyBuffer};
use honeyos_process::{
    context::{ApiModuleBuilder, ProcessCtx},
    ProcessManager,
};
use wasm_bindgen::closure::Closure;

/// Register the display api
pub fn register_display_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_display_assume_control
    // Attempt to take control of the display
    // ### Returns
    // - `0` On success
    // - `-1` If the display is occupied
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_assume_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let mut display = Display::blocking_get_writer();
            if let Err(Error::DisplayOccupied) = display.assume_control(ctx_f.pid()) {
                return -1;
            }
            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_loosen_control
    // Loosen the control over display.
    // In this state, the process still controls the display, but allows other processes to take it over without override.
    // ### Returns
    // - `0` On success
    // - `-1` If the process doesn't have control over the display
    // - `-2` If the display control is already loose
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_loosen_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let mut display = Display::blocking_get_writer();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            match display.loosen_control() {
                Err(_) => -2,
                Ok(_) => 0,
            }
        })
        .into_js_value(),
    );

    // hapi_display_override_control
    // Override the control over the display
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_override_control",
        Closure::<dyn Fn()>::new(move || {
            let mut display = Display::blocking_get_writer();
            display.override_control(ctx_f.pid());
        })
        .into_js_value(),
    );

    // hapi_display_release_control
    // Release the control over the display.
    // ### Returns
    // - `0` On success
    // - `-1` If the display is occupied
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_release_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let mut display = Display::blocking_get_writer();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            display.release_control();
            0
        })
        .into_js_value(),
    );

    // hapi_display_release_control
    // Take away the control over the display from the currently controling process,
    // regardless of whether the process has control.
    builder.register(
        "hapi_display_displace_control",
        Closure::<dyn Fn()>::new(move || {
            let mut display = Display::blocking_get_writer();
            display.release_control();
        })
        .into_js_value(),
    );

    // hapi_display_push_stdout
    // Push stdout to the display's text-mode buffer.
    // Do nothing if the process does not have control of the display.
    // ### Returns
    // - `0` On success
    // - `-1` If the process doesn't have control over the display
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_push_stdout",
        Closure::<dyn Fn() -> i32>::new(move || {
            let mut display = Display::blocking_get_writer();

            if !display.has_control(ctx_f.pid()) {
                return -1;
            }

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
    // Do nothing if the process does not have control of the display.
    // ### Returns
    // - `0` On success
    // - `-1` If the process doesn't have control over the display
    // - `-2` If the string is invalid
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_set_text",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |ptr: *const u8| {
            let memory = ctx_f.memory();
            let Some(string) = &memory.read_str(ptr as u32) else {
                return -2;
            };
            let mut display = Display::blocking_get_writer();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            display.set_text(string);
            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_buffer
    // Get the key in the displays key buffer. Clears it afterwards.
    // Do nothing if the process does not have control of the display.
    // ### Returns
    // - The key stored in the key buffer
    // - `-1` If the process doesn't have control over the display
    // - `-2` or if the key buffer is empty.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_get_key_buffer",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let display = Display::blocking_get_reader();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            if display.keybuffer.key < 0 {
                return -2;
            }
            return display.keybuffer.key;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_shift
    // Whether or not the shift key is in the key buffer
    // Do nothing if the process does not have control of the display.
    // ### Returns
    // - Whether the shift key is pressed in the key buffer
    // - `-1` If the process doesn't have control over the display
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_get_key_shift",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let display = Display::blocking_get_reader();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            return display.keybuffer.shift as i32;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_ctrl
    // Whether or not the control key is in the key buffer
    // Do nothing if the process does not have control of the display.
    // ### Returns
    // - Whether the ctrl key is pressed in the key buffer
    // - `-1` If the process doesn't have control over the display
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_get_key_ctrl",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let display = Display::blocking_get_reader();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            return display.keybuffer.ctrl as i32;
        })
        .into_js_value(),
    );

    // hapi_display_clear_key
    // Clears the key buffer of the display
    // Do nothing if the process does not have control of the display.
    // ### Returns
    // - `0` on success
    // - `-1` If the process doesn't have control over the display
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_clear_key",
        Closure::<dyn Fn() -> i32>::new(move || loop {
            let mut display = Display::blocking_get_writer();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
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
