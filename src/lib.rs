#![feature(async_fn_traits)]

pub mod api;
pub mod boot;

use anyhow::anyhow;
use honeyos_display::Display;
use honeyos_fs::FsManager;
use honeyos_networking::NetworkingManager;
use honeyos_process::ProcessManager;
use wasm_bindgen::prelude::{wasm_bindgen, Closure, JsCast};

/// The kernel entrypoint
#[wasm_bindgen(start)]
async fn main() {
    console_log::init_with_level(log::Level::Info).unwrap();
    set_panic_hook();

    // Initialize kernel systems
    Display::init_once();
    FsManager::init_once();
    ProcessManager::init_once(api::register_api);
    NetworkingManager::init_once();

    // Request the boot excutable and execute it once fetched
    let window = get_window().unwrap();
    boot::request_boot_executable(&window).await.unwrap();

    execution_loop(0.0).unwrap();
}

/// The main execution loop of the OS
/// This function will be called repeatedly to execute the OS
fn execution_loop(_time_stamp: f64) -> anyhow::Result<()> {
    let window = get_window()?;

    handle_spawn_requests();
    update_network_manager();
    render_display_server();

    window
        .request_animation_frame(
            &Closure::once_into_js(|t| execution_loop(t).unwrap())
                .as_ref()
                .unchecked_ref(),
        )
        .map_err(|e| anyhow!("Failed to request animation frame: {:?}", e))?;
    Ok(())
}

/// Handle the spawn requests
fn handle_spawn_requests() {
    if let Some(mut process_manager) = ProcessManager::get() {
        for request in process_manager.requests() {
            if let Some(process) = process_manager.process(*request) {
                if let Err(e) = process.spawn() {
                    log::error!("Failed to spawn process: {}", e);
                }
            }
        }
        process_manager.requests_mut().clear();
    }
}

/// Update the network manager
fn update_network_manager() {
    // Update the network manager
    if let Some(mut network_manager) = NetworkingManager::get_writer() {
        if let Err(e) = network_manager.update() {
            log::error!("Failed to complete network request: {}", e);
        }
    }
}

/// Render the display server
fn render_display_server() {
    // Render the display server
    if let Some(mut display_server) = Display::get_writer() {
        display_server.render();
    } else {
        log::info!("Missed display frame");
    }
}

/// The panic hook for the WASM module
fn set_panic_hook() {
    static SET_HOOK: std::sync::Once = std::sync::Once::new();
    SET_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|panic_info| {
            log::error!("Kernel Panic: {}", panic_info);

            let display_server = Display::get_reader().unwrap();
            let root = display_server.root().unwrap();
            root.set_inner_text(&format!("Kernel Panic: {}", panic_info));
        }));
    });
}

/// Get the window
fn get_window() -> anyhow::Result<web_sys::Window> {
    web_sys::window().ok_or(anyhow!(
        "Failed to get window. Kernel must be executed in main thread."
    ))
}
