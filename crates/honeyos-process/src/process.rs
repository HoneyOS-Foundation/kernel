use anyhow::anyhow;
use honeyos_atomics::mutex::SpinMutex;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::{Function, Reflect, WebAssembly, JSON},
    Blob, Url, Worker, WorkerOptions, WorkerType,
};

use crate::{
    context::{ApiBuilderFn, ProcessCtx},
    memory::Memory,
    requirements::WasmRequirements,
    stdout::ProcessStdOut,
    thread::ThreadPool,
    ProcessManager,
};

/// A process in honeyos
pub struct Process {
    // The process id
    id: Uuid,
    // The process title
    title: String,
    // The current working directory for the process
    cwd: Arc<RwLock<String>>,
    // The process context
    ctx: Arc<ProcessCtx>,
    // The worker for the process
    worker: Option<Worker>,
    // Flag for if the process is alive
    alive: Arc<AtomicBool>,
    // The threadpool
    thread_pool: ThreadPool,
    // The stdout
    stdout: Arc<ProcessStdOut>,
}

impl Process {
    /// Create a process
    pub fn new(
        id: Uuid,
        wasm_bin: Vec<u8>,
        title: &str,
        working_directory: &str,
        api_builder: ApiBuilderFn,
    ) -> anyhow::Result<Self> {
        let title = title.to_string();
        // The running flag
        let alive = Arc::new(AtomicBool::new(true));
        // The stdout
        let stdout = Arc::new(ProcessStdOut::new());
        // The current working directory
        let cwd = Arc::new(RwLock::new(working_directory.to_string()));
        // Create the process context
        let ctx = create_context(id, &wasm_bin, stdout.clone(), cwd.clone(), api_builder)?;
        // Create the thread pool
        let thread_pool = ThreadPool::new(id);

        Ok(Self {
            id,
            title,
            alive,
            stdout,
            cwd,
            ctx,
            thread_pool,
            worker: None,
        })
    }

    /// Spawn the process
    pub fn spawn(&mut self) -> anyhow::Result<()> {
        let mut options = WorkerOptions::new();
        options.type_(WorkerType::Module);

        let worker = Worker::new_with_options(&get_worker_script(), &options)
            .map_err(|e| anyhow::anyhow!("Failed to create worker: {:?}", e))?;
        let msg = web_sys::js_sys::Array::new();

        // Send the pid
        msg.push(&self.id.to_string().into());
        // Send the kernel module
        msg.push(&wasm_bindgen::module());
        // Send the kernel memory
        msg.push(&wasm_bindgen::memory());
        // Send the process memory
        msg.push(self.ctx().memory_nospin().inner());

        worker
            .post_message(&msg)
            .map_err(|e| anyhow::anyhow!("Failed to send message to worker: {:?}", e))?;

        // Set callbacks
        let alive_callback = self.alive.clone();
        let onmessage_callback =
            Closure::wrap(
                Box::new(move || alive_callback.store(false, Ordering::Relaxed))
                    as Box<dyn FnMut()>,
            );
        let alive_callback = self.alive.clone();
        let onerror_callback =
            Closure::wrap(
                Box::new(move || alive_callback.store(false, Ordering::Relaxed))
                    as Box<dyn FnMut()>,
            );
        worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        worker.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));

        self.worker = Some(worker);
        self.alive.store(true, Ordering::Release);

        onmessage_callback.forget();
        onerror_callback.forget();

        Ok(())
    }

    /// Spawn a thread and return it's id
    pub fn spawn_thread(&mut self, f_ptr: u32) -> anyhow::Result<u32> {
        let id = self
            .thread_pool
            .spawn(f_ptr, self.ctx().memory_nospin().inner())?;
        Ok(id)
    }

    /// Kill the process
    pub fn kill(&mut self) {
        self.thread_pool.kill_all(); // Kill all threads
        if let Some(worker) = self.worker.as_mut() {
            worker.terminate();
        }
        self.alive.store(false, Ordering::Relaxed);
    }

    /// Get the id
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the context
    pub fn ctx(&self) -> Arc<ProcessCtx> {
        self.ctx.clone()
    }

    /// Check if the process is still running
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Get the stdout
    pub fn stdout(&self) -> Arc<ProcessStdOut> {
        self.stdout.clone()
    }

    /// Get the current working directory
    pub fn cwd(&self) -> String {
        self.cwd.read().unwrap().clone()
    }
}

/// Create the instance in the worker
#[wasm_bindgen]
pub async fn create_instance(
    pid: String,
    memory: &WebAssembly::Memory,
    table: &WebAssembly::Table,
) -> WebAssembly::Instance {
    let pid = Uuid::parse_str(&pid).unwrap();
    let process_manager_lock = ProcessManager::get();
    let Ok(process_manager) = process_manager_lock.spin_lock() else {
        panic!("Process Manager Poisoned");
    };

    let process = process_manager.process(pid).unwrap();

    let ctx = process.ctx();
    let ctx = Arc::new(ctx.new_worker(memory.clone()));

    let environment = setup_environment(&ctx.memory().inner(), &table)
        .map_err(|e| log::error!("Failed to create environment: {}", e))
        .unwrap();
    let api_module = ctx.build_api();
    let imports = setup_imports(environment, &api_module)
        .map_err(|e| log::error!("Failed to setup imports: {}", e))
        .unwrap();

    init_binary(&ctx.module(), imports).await
}

/// Create the context
fn create_context(
    pid: Uuid,
    bin: &[u8],
    stdout: Arc<ProcessStdOut>,
    cwd: Arc<RwLock<String>>,
    api_builder: ApiBuilderFn,
) -> anyhow::Result<Arc<ProcessCtx>> {
    // Parse the wasm
    let requirements = WasmRequirements::parse(&bin).unwrap();

    // Create the memory
    let memory = Arc::new(Mutex::new(
        Memory::new(
            requirements.initial_memory,
            requirements.maximum_memory,
            requirements.shared_memory,
        )
        .expect("Failed to initialize instance's memory"),
    ));

    let bin = Arc::new(bin.to_vec());
    Ok(Arc::new(ProcessCtx::new(
        pid,
        memory.clone(),
        stdout,
        cwd,
        bin.clone(),
        api_builder,
    )))
}

/// Setup the env
pub fn setup_environment(
    memory: &WebAssembly::Memory,
    table: &WebAssembly::Table,
) -> anyhow::Result<JsValue> {
    let env = JSON::parse("{}").unwrap();
    Reflect::set(&env, &"memory".into(), memory)
        .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(&env, &"table".into(), &table)
        .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;

    setup_emscripten_environment(&env)?;
    Ok(env)
}

/// Setup the imports object
pub fn setup_imports(environment: JsValue, api_module: &JsValue) -> anyhow::Result<JsValue> {
    let imports_object = JSON::parse("{}").unwrap();
    Reflect::set(&imports_object, &"env".into(), &environment)
        .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;
    Reflect::set(&imports_object, &"hapi".into(), &api_module)
        .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;

    // This does nothing. But emscripten expects this to be there. This might be removed in the future
    setup_emscripten_imports(&imports_object)?;
    Ok(imports_object)
}

/// Add dummy methods to the env for emscripten suppoort.
/// These methods remain unimplemented as they are not needed, but emscripten still expects them
fn setup_emscripten_environment(env: &JsValue) -> anyhow::Result<()> {
    Reflect::set(
        &env,
        &"emscripten_notify_memory_growth".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"_emscripten_notify_mailbox_postmessage".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"emscripten_check_blocking_allowed".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"_emscripten_notify_mailbox_postmessage".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"_emscripten_receive_on_main_thread_js".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"__emscripten_init_main_thread_js".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"_emscripten_thread_mailbox_await".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"_emscripten_thread_set_strongref".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"emscripten_exit_with_live_runtime".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;
    Reflect::set(
        &env,
        &"__emscripten_thread_cleanup".into(),
        &Function::new_no_args("{}"),
    )
    .map_err(|e| anyhow!("Failed to setup env: {:?}", e))?;

    Ok(())
}

/// Add dummy methods to the import for emscripten suppoort.
/// These methods remain unimplemented as they are not needed, but emscripten still expects them
fn setup_emscripten_imports(imports_object: &JsValue) -> anyhow::Result<()> {
    let wasi_snapshot_preview1 = JSON::parse("{}").unwrap();
    Reflect::set(
        &wasi_snapshot_preview1,
        &"proc_exit".into(),
        &Function::new_no_args("{}").into(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;
    Reflect::set(
        &wasi_snapshot_preview1,
        &"clock_time_get".into(),
        &Function::new_no_args("{}").into(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;
    Reflect::set(
        &wasi_snapshot_preview1,
        &"fd_close".into(),
        &Function::new_no_args("{}").into(),
    )
    .map_err(|e| anyhow::anyhow!("Failed     to setup imports: {:?}", e))?;
    Reflect::set(
        &wasi_snapshot_preview1,
        &"fd_write".into(),
        &Function::new_no_args("{}").into(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;
    Reflect::set(
        &wasi_snapshot_preview1,
        &"fd_seek".into(),
        &Function::new_no_args("{}").into(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;
    Reflect::set(
        &imports_object,
        &"wasi_snapshot_preview1".into(),
        &wasi_snapshot_preview1,
    )
    .map_err(|e| anyhow::anyhow!("Failed to setup imports: {:?}", e))?;
    Ok(())
}

/// Initialize the wasm instance
pub async fn init_binary(bin: &[u8], imports: JsValue) -> WebAssembly::Instance {
    let promise = WebAssembly::instantiate_buffer(bin, &imports.unchecked_into());
    let promise = JsFuture::from(promise);
    let instance = promise
        .await
        .map_err(|e| {
            log::error!("Failed to create instance: {:?}", e);
        })
        .unwrap();

    Reflect::get(&instance, &"instance".into())
        .unwrap()
        .dyn_into()
        .unwrap()
}

/// Generate the worker script encoded blob url. (Cached for performance)
fn get_worker_script() -> String {
    static CACHED_SCRIPT: Mutex<Option<String>> = Mutex::new(None);

    // Cache the url
    if let Ok(mut cached) = CACHED_SCRIPT.try_lock() {
        if let Some(cached) = cached.as_mut() {
            return cached.clone();
        }
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
    if let Ok(mut cached) = CACHED_SCRIPT.try_lock() {
        if let Some(cached) = cached.as_mut() {
            *cached = url.clone();
        }
    }

    url
}
