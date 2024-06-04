//! Glue code to handle multithreading
use std::sync::{Arc, Mutex};

use uuid::Uuid;
use wasm_bindgen::prelude::JsValue;
use web_sys::{js_sys::Uint8Array, Blob, Url, Worker, WorkerOptions, WorkerType};

use crate::api::ProcessCtx;

/// Spawn a process on a thread
pub(crate) fn spawn_process(pid: Uuid, module: &[u8]) -> anyhow::Result<Worker> {
    let mut options = WorkerOptions::new();
    options.type_(WorkerType::Module);

    let worker = Worker::new_with_options(&get_worker_script_process(), &options)
        .map_err(|e| anyhow::anyhow!("Failed to create worker: {:?}", e))?;
    let msg = web_sys::js_sys::Array::new();

    // Send the pid
    msg.push(&JsValue::from(pid.to_string()));
    // Send the kernel module
    msg.push(&wasm_bindgen::module());
    // Send the kernel memory
    msg.push(&wasm_bindgen::memory());
    // Send the module binary
    msg.push(&Uint8Array::from(module));

    worker
        .post_message(&msg)
        .map_err(|e| anyhow::anyhow!("Failed to send message to worker: {:?}", e))?;

    Ok(worker)
}

/// Spawn a thread as a subprocess
pub fn spawn_thread(pid: Uuid, ctx: Arc<ProcessCtx>, f_ptr: u32) {
    let ctx = ctx.clone();
    let mut options = WorkerOptions::new();
    options.type_(WorkerType::Module);

    let worker = Worker::new_with_options(&get_worker_script_thread(), &options)
        .map_err(|e| anyhow::anyhow!("Failed to create worker: {:?}", e))
        .unwrap();
    let msg = web_sys::js_sys::Array::new();

    // Send the pid
    msg.push(&JsValue::from(pid.to_string()));
    // Send the kernel module
    msg.push(&wasm_bindgen::module());
    // Send the kernel memory
    msg.push(&wasm_bindgen::memory());
    // The module
    msg.push(&Uint8Array::from(&ctx.module()[..]));
    // The memory
    msg.push(&ctx.memory().inner());
    // The function pointer
    msg.push(&JsValue::from(f_ptr as u32));

    worker
        .post_message(&msg)
        .map_err(|e| anyhow::anyhow!("Failed to send message to worker: {:?}", e))
        .unwrap();
}

fn get_worker_script_process() -> String {
    static CACHED_SCRIPT: Mutex<Option<String>> = Mutex::new(None);

    let cached: Option<String>;
    loop {
        if let Ok(url) = CACHED_SCRIPT.try_lock() {
            cached = url.clone();
            break;
        }
    }

    if let Some(url) = cached {
        return url;
    }

    // Aquire the script path by generating a stack trace and parsing the path from it.
    let wasm_shim_url = web_sys::js_sys::eval(include_str!("js/script_path.js"))
        .unwrap()
        .as_string()
        .unwrap();

    let template = include_str!("js/worker_executable.js");
    let script = template.replace("BINDGEN_SHIM_URL", &wasm_shim_url);

    // Create url encoded blob
    let arr = web_sys::js_sys::Array::new();
    arr.set(0, JsValue::from_str(&script));
    let blob = Blob::new_with_str_sequence(&arr).unwrap();
    let url = Url::create_object_url_with_blob(
        &blob
            .slice_with_f64_and_f64_and_content_type(0.0, blob.size(), "text/javascript")
            .unwrap(),
    )
    .unwrap();

    // Cache the url
    loop {
        if let Ok(mut cached) = CACHED_SCRIPT.try_lock() {
            *cached = Some(url.clone());
            break;
        }
    }

    url
}

/// Generate the worker script encoded blob url. (Cached for performance)
fn get_worker_script_thread() -> String {
    static CACHED_SCRIPT: Mutex<Option<String>> = Mutex::new(None);

    let cached: Option<String>;
    loop {
        if let Ok(url) = CACHED_SCRIPT.try_lock() {
            cached = url.clone();
            break;
        }
    }

    if let Some(url) = cached {
        return url;
    }

    // Aquire the script path by generating a stack trace and parsing the path from it.
    let wasm_shim_url = web_sys::js_sys::eval(include_str!("js/script_path.js"))
        .unwrap()
        .as_string()
        .unwrap();

    let template = include_str!("js/worker.js");
    let script = template.replace("BINDGEN_SHIM_URL", &wasm_shim_url);

    // Create url encoded blob
    let arr = web_sys::js_sys::Array::new();
    arr.set(0, JsValue::from_str(&script));
    let blob = Blob::new_with_str_sequence(&arr).unwrap();
    let url = Url::create_object_url_with_blob(
        &blob
            .slice_with_f64_and_f64_and_content_type(0.0, blob.size(), "text/javascript")
            .unwrap(),
    )
    .unwrap();

    // Cache the url
    loop {
        if let Ok(mut cached) = CACHED_SCRIPT.try_lock() {
            *cached = Some(url.clone());
            break;
        }
    }

    url
}
