use anyhow::anyhow;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, MutexGuard, RwLock,
};
use uuid::Uuid;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{Function, Reflect, WebAssembly, JSON};

use crate::{
    api::{ApiBuilderFn, ProcessCtx},
    memory::Memory,
    requirements::WasmRequirements,
    stdout::ProcessStdOut,
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

    // Flag for if the process is still running
    running: Arc<AtomicBool>,

    // The stdout
    stdout: ProcessStdOut,

    // The module binary
    bin: Arc<Vec<u8>>,

    // The api builder fn
    api_builder: ApiBuilderFn,
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
        // The running flag
        let running = Arc::new(AtomicBool::new(false));
        // The stdout
        let stdout = ProcessStdOut::new();
        // The current working directory
        let cwd = Arc::new(RwLock::new(working_directory.to_string()));
        // Clone the module
        let module = Arc::new(wasm_bin);

        Ok(Self {
            id,
            title: title.to_string(),
            running,
            stdout,
            cwd: cwd.clone(),
            bin: module,
            api_builder,
        })
    }

    /// Spawn the process
    pub fn spawn(&self) -> anyhow::Result<()> {
        crate::thread::spawn_process(self.id, &self.bin)?;
        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Get the id
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Check if the process is still running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get the stdout
    pub fn stdout(&self) -> &ProcessStdOut {
        &self.stdout
    }

    /// Get the stdout
    pub fn stdout_mut(&mut self) -> &mut ProcessStdOut {
        &mut self.stdout
    }

    /// Get the current working directory
    pub fn cwd(&self) -> String {
        self.cwd.read().unwrap().clone()
    }

    /// Kill the process
    pub fn kill(&self) {
        self.running.store(true, Ordering::Relaxed);
    }
}

/// Create the instance in the worker
#[wasm_bindgen]
pub async fn create_instance(pid: String, bin: &[u8]) -> WebAssembly::Instance {
    let pid = Uuid::parse_str(&pid).unwrap();
    let process_manager = ProcessManager::blocking_get();

    let handle = process_manager.process(pid).expect("Invalid pid");
    let stdout = handle.stdout().process_buffer();
    let cwd = handle.cwd.clone();

    // Parse the wasm
    let requirements = WasmRequirements::parse(&bin).unwrap();

    // Create the memory
    let memory = Arc::new(Mutex::new(
        Memory::new(
            requirements.initial_memory,
            requirements.maximum_memory,
            requirements.shared_memory,
        )
        .expect("Failed to init binaries memory"),
    ));

    let table = Arc::new(setup_table().unwrap());
    let bin = Arc::new(bin.to_vec());

    let ctx: Arc<ProcessCtx> = Arc::new(ProcessCtx::new(
        pid,
        memory.clone(),
        table.clone(),
        stdout,
        cwd,
        bin.clone(),
        handle.api_builder,
    ));

    let environment = setup_environment(&ctx.memory(), &ctx.table()).unwrap();
    let api_module = ctx.build_api();
    let imports = setup_imports(environment, &api_module).unwrap();

    init_binary(&*bin, imports).await
}

/// Create a new instance in order to run on a seperate thread
#[wasm_bindgen]
pub async fn create_thread_instance(
    pid: String,
    bin: &[u8],
    memory: WebAssembly::Memory,
) -> WebAssembly::Instance {
    // TODO: Refactor this to not need to recompile the wasm each time.
    // Do this by utilizing `WebAssembly::Module``

    let pid = Uuid::parse_str(&pid).unwrap();
    let process_manager = ProcessManager::blocking_get();

    let handle = process_manager.process(pid).expect("Invalid pid");
    let stdout = handle.stdout().process_buffer();
    let cwd = handle.cwd.clone();

    // Create the memory
    let memory: Arc<Mutex<Memory>> = Arc::new(Mutex::new(Memory::from_inner(memory)));
    let table = Arc::new(setup_table().unwrap());

    let bin = Arc::new(bin.to_vec());

    let ctx: Arc<ProcessCtx> = Arc::new(ProcessCtx::new(
        pid,
        memory.clone(),
        table.clone(),
        stdout,
        cwd,
        bin.clone(),
        handle.api_builder,
    ));

    let environment = setup_environment(&ctx.memory(), &ctx.table()).unwrap();
    let api_module = ctx.build_api();
    let imports = setup_imports(environment, &api_module).unwrap();

    init_binary(&*bin, imports).await
}

/// Setup table
fn setup_table() -> anyhow::Result<WebAssembly::Table> {
    const INITIAL: u32 = 4;
    const ELEMENT: &str = "anyfunc";

    let table_desc = JSON::parse("{}").unwrap();
    Reflect::set(&table_desc, &"initial".into(), &INITIAL.into())
        .map_err(|e| anyhow!("Failed to setup table: {:?}", e))?;
    Reflect::set(&table_desc, &"element".into(), &ELEMENT.into())
        .map_err(|e| anyhow!("Failed to setup table: {:?}", e))?;
    WebAssembly::Table::new(&table_desc.unchecked_into())
        .map_err(|e| anyhow!("Failed to setup table: {:?}", e))
}

/// Setup the env
pub fn setup_environment(
    memory: &MutexGuard<Memory>,
    table: &WebAssembly::Table,
) -> anyhow::Result<JsValue> {
    let env = JSON::parse("{}").unwrap();
    Reflect::set(&env, &"memory".into(), memory.inner())
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
    let instance = promise.await.unwrap();

    Reflect::get(&instance, &"instance".into())
        .unwrap()
        .dyn_into()
        .unwrap()
}
