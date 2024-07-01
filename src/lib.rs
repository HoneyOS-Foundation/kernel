pub mod api;
pub mod boot;

use std::rc::Rc;

use anyhow::anyhow;
use honeyos_display::Display;
use honeyos_fs::FsManager;
use honeyos_networking::NetworkingManager;
use honeyos_process::ProcessManager;
use wasm_bindgen::prelude::{wasm_bindgen, Closure, JsCast};
use web_sys::Window;

/// Some delay to prevent the os from using too much cpu
const EXECUTION_TIMEOUT: i32 = 25;

// To prevent GC invocations every cycle, the callbacks is stored in a thread_local static variable
thread_local! {
    static EXECUTION_CALLBACK: Rc<Closure<dyn FnMut()>> = Rc::new(Closure::wrap(Box::new(|| execution_loop().unwrap())));
    static DISPLAY_CALLBACK: Rc<Closure<dyn FnMut()>> = Rc::new(Closure::wrap(Box::new(|| display_loop().unwrap())));
}

/// The kernel entrypoint
#[wasm_bindgen]
pub async fn main() {
    console_log::init_with_level(log::Level::Info).unwrap();
    set_panic_hook();

    // Initialize kernel systems*
    Display::init_once();
    FsManager::init_once();
    ProcessManager::init_once(api::register_api);
    NetworkingManager::init_once();

    // Request the boot excutable and execute it once fetched
    let window = get_window().unwrap();
    boot::request_boot_executable(&window).await.unwrap();

    start_execution_loop(&window).unwrap();
    display_loop().unwrap();
}

/// The execution loop
fn execution_loop() -> anyhow::Result<()> {
    update_process_manager();
    update_network_manager();
    Ok(())
}

/// The display loop of the OS
fn display_loop() -> anyhow::Result<()> {
    let window = get_window()?;

    render_display_server();
    window
        .request_animation_frame(
            DISPLAY_CALLBACK
                .with(|f| f.clone())
                .as_ref()
                .as_ref()
                .unchecked_ref(),
        )
        .map_err(|e| anyhow!("Failed to request animation frame: {:?}", e))?;
    Ok(())
}

/// Set the execution loop timeout
fn start_execution_loop(window: &Window) -> anyhow::Result<i32> {
    window
        .set_interval_with_callback_and_timeout_and_arguments_0(
            EXECUTION_CALLBACK
                .with(|f| f.clone())
                .as_ref()
                .as_ref()
                .unchecked_ref(),
            EXECUTION_TIMEOUT,
        )
        .map_err(|e| anyhow!("Failed to set execution loop to interval: {:?}", e))
}

/// Handle the spawn requests
fn update_process_manager() {
    let process_manager_lock = ProcessManager::get();
    if let Ok(mut process_manager) = process_manager_lock.try_lock() {
        process_manager.update();
    };
}

/// Update the network manager
fn update_network_manager() {
    // Update the network manager
    let networking_manager_lock = NetworkingManager::get();
    if let Ok(mut network_manager) = networking_manager_lock.try_write() {
        if let Err(e) = network_manager.update() {
            log::error!("Failed to complete network request: {}", e);
        }
    };
}

/// Render the display server
fn render_display_server() {
    // Render the display server
    let display_lock: std::sync::Arc<std::sync::RwLock<Display>> = Display::get();
    loop {
        let Ok(mut display_server) = display_lock.try_write() else {
            continue;
        };
        display_server.render();
        break;
    }
}

/// The panic hook for the WASM module
fn set_panic_hook() {
    static SET_HOOK: std::sync::Once = std::sync::Once::new();
    SET_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|panic_info| {
            log::error!("Kernel Panic: {}", panic_info);

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let body = document.body().unwrap();
            body.set_inner_text(&format!("Kernel Panic: {}", panic_info));
        }));
    });
}

/// Get the window
fn get_window() -> anyhow::Result<web_sys::Window> {
    web_sys::window().ok_or(anyhow!(
        "Failed to get window. Kernel must be executed in main thread."
    ))
}
