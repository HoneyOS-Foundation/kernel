use std::sync::Arc;

use honeyos_atomics::{mutex::SpinMutex, rwlock::SpinRwLock};
use honeyos_display::{error::Error, Display, KeyBuffer};
use honeyos_process::{
    context::{ApiModuleBuilder, ProcessCtx},
    ProcessManager,
};
use wasm_bindgen::closure::Closure;

/// Register the display api
pub fn register_display_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_display_assume_control
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_assume_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
            if let Err(Error::DisplayOccupied) = display.assume_control(ctx_f.pid()) {
                return -1;
            }
            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_loosen_control
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_loosen_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
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
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_override_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
            display.override_control(ctx_f.pid());
            0
        })
        .into_js_value(),
    );

    // hapi_display_release_control
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_release_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            display.release_control();
            0
        })
        .into_js_value(),
    );

    // hapi_display_release_control
    builder.register(
        "hapi_display_displace_control",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
            display.release_control();
            0
        })
        .into_js_value(),
    );

    // hapi_display_push_stdout
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_push_stdout",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }

            // Get stdout from the process manager
            let stdout_str = {
                let process_manager_lock = ProcessManager::get();
                let Ok(mut process_manager) = process_manager_lock.spin_lock() else {
                    return -1;
                };
                let process = process_manager.process_mut(ctx_f.pid()).unwrap();
                let stdout = process.stdout();

                // Sync stdout
                stdout.sync();
                stdout.buffer()
            };

            // Send stdout to the display
            {
                let text_mode = display.text_mode_mut();
                text_mode.clear();
                text_mode.append_str(&stdout_str);
            }

            display.notify_update();

            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_set_text
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_set_text",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |ptr: *const u8| {
            let memory = ctx_f.memory();
            let Some(string) = &memory.read_str(ptr as u32) else {
                return -2;
            };
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }

            {
                let text_mode = display.text_mode_mut();
                text_mode.clear();
                text_mode.append_str(string);
            }

            display.notify_update();

            return 0;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_buffer
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_get_key_buffer",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let display = display_lock.spin_read().unwrap();
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
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_get_key_shift",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let display = display_lock.spin_read().unwrap();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            return display.keybuffer.shift as i32;
        })
        .into_js_value(),
    );

    // hapi_display_get_key_ctrl
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_get_key_ctrl",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let display = display_lock.spin_read().unwrap();
            if !display.has_control(ctx_f.pid()) {
                return -1;
            }
            return display.keybuffer.ctrl as i32;
        })
        .into_js_value(),
    );

    // hapi_display_clear_key
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_display_clear_key",
        Closure::<dyn Fn() -> i32>::new(move || {
            let display_lock = Display::get();
            let mut display = display_lock.spin_write().unwrap();
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
