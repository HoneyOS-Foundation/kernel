//! Glue code to handle multithreading
use std::sync::{Arc, Mutex};

use hashbrown::HashMap;
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, prelude::JsValue, JsCast};
use web_sys::{js_sys::WebAssembly, Blob, Url, Worker, WorkerOptions, WorkerType};

/// The error types for threads
#[derive(Debug)]
pub enum ThreadError {
    NoSuchThread(u32),
    WorkerCreation(String),
    WorkerMessaging(String),
}

/// The request for spawning a thread
#[derive(Debug)]
pub struct ThreadRequest {
    pub pid: Uuid,
    pub fptr: u32,
}

/// Represents a thread
#[derive(Debug)]
pub struct Thread {
    worker: Worker,
    alive: bool,
}

/// The threadpool for a process
#[derive(Debug)]
pub struct ThreadPool {
    pid: Uuid,
    threads: Arc<Mutex<HashMap<u32, Thread>>>,
    thread_amount: u32,
}

impl ThreadPool {
    pub fn new(pid: Uuid) -> Self {
        Self {
            pid,
            threads: Arc::new(Mutex::new(HashMap::new())),
            thread_amount: 0,
        }
    }

    /// Spawn a thread
    pub fn spawn(&mut self, f_ptr: u32, memory: &WebAssembly::Memory) -> Result<u32, ThreadError> {
        let id = self.thread_amount;
        let worker = spawn_worker(self.pid, f_ptr, &memory)?;

        let threads = self.threads.clone();

        // Register callbacks
        let threads_callback = threads.clone();
        let onmessage_callback = Closure::wrap(Box::new(move || loop {
            let Ok(mut threads) = threads_callback.try_lock() else {
                continue;
            };
            let thread = threads.get_mut(&id).unwrap();
            thread.alive = false;
            break;
        }) as Box<dyn FnMut()>);
        let threads_callback = threads.clone();
        let onerror_callback = Closure::wrap(Box::new(move || loop {
            let Ok(mut threads) = threads_callback.try_lock() else {
                continue;
            };
            let thread = threads.get_mut(&id).unwrap();
            thread.alive = false;
            break;
        }) as Box<dyn FnMut()>);

        worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        worker.set_onmessage(Some(onerror_callback.as_ref().unchecked_ref()));

        // Wait till the lock is free
        loop {
            let Ok(mut threads) = threads.try_lock() else {
                continue;
            };
            threads.insert(
                id,
                Thread {
                    worker,
                    alive: true,
                },
            );
            break;
        }
        self.thread_amount += 1;

        onmessage_callback.forget();
        onerror_callback.forget();
        Ok(id)
    }

    /// Check if a thread is alive.
    /// Also returns false if the id is invalid
    pub fn alive(&self, id: u32) -> bool {
        loop {
            let Ok(threads) = self.threads.try_lock() else {
                continue;
            };
            let thread = threads.get(&id).unwrap();
            return thread.alive;
        }
    }

    /// Kill a thread
    pub fn kill(&mut self, id: u32) -> Result<(), ThreadError> {
        loop {
            let Ok(threads) = self.threads.try_lock() else {
                continue;
            };
            let thread = threads.get(&id).unwrap();
            thread.worker.terminate();
            break;
        }
        Ok(())
    }

    /// Kill all threads
    pub fn kill_all(&mut self) {
        loop {
            let Ok(mut threads) = self.threads.try_lock() else {
                continue;
            };
            for (_, thread) in threads.iter_mut() {
                thread.worker.terminate();
                thread.alive = false;
            }
            break;
        }
    }
}

/// Spawn a thread as a subprocess
fn spawn_worker(
    pid: Uuid,
    f_ptr: u32,
    memory: &WebAssembly::Memory,
) -> Result<Worker, ThreadError> {
    let mut options = WorkerOptions::new();
    options.type_(WorkerType::Module);

    let script = generate_worker_script();
    let worker = Worker::new_with_options(&script, &options)
        .map_err(|e| ThreadError::WorkerCreation(format!("{:?}", e)))?;
    let msg = web_sys::js_sys::Array::new();

    // Send the pid
    msg.push(&JsValue::from(pid.to_string()));
    // Send the kernel module
    msg.push(&wasm_bindgen::module());
    // Send the kernel memory
    msg.push(&wasm_bindgen::memory());
    // Send the instance memory
    msg.push(&memory);
    // The function pointer
    msg.push(&JsValue::from(f_ptr));

    worker
        .post_message(&msg)
        .map_err(|e| ThreadError::WorkerMessaging(format!("{:?}", e)))?;

    Ok(worker)
}

/// Generate the worker script encoded blob url. (Cached for performance)
fn generate_worker_script() -> String {
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

impl std::error::Error for ThreadError {}

impl std::fmt::Display for ThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadError::NoSuchThread(id) => writeln!(f, "No such thread with id: {}", id),
            ThreadError::WorkerCreation(e) => writeln!(f, "Failed to create worker: {:?}", e),
            ThreadError::WorkerMessaging(e) => {
                writeln!(f, "Failed to post message to worker: {:?}", e)
            }
        }
    }
}
