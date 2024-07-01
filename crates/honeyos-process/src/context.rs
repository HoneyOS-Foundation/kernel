use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use hashbrown::HashMap;
use honeyos_atomics::{mutex::SpinMutex, rwlock::SpinRwLock};
use uuid::Uuid;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::js_sys::{Reflect, WebAssembly, JSON};

use crate::{memory::Memory, stdout::ProcessStdOut};

/// A function responsible for building the api for wasm processes
pub type ApiBuilderFn = fn(Arc<ProcessCtx>, &mut ApiModuleBuilder);

/// The context for the process
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ProcessCtx {
    pid: Uuid,
    stdout: Arc<ProcessStdOut>,
    memory: Arc<Mutex<Memory>>,
    cwd: Arc<RwLock<String>>,
    module: Arc<Vec<u8>>,
    api_builder: ApiBuilderFn,
}

/// The builder for an api module
#[derive(Debug, Clone)]
pub struct ApiModuleBuilder {
    values: HashMap<String, JsValue>,
}

impl ProcessCtx {
    pub fn new(
        pid: Uuid,
        memory: Arc<Mutex<Memory>>,
        stdout: Arc<ProcessStdOut>,
        cwd: Arc<RwLock<String>>,
        module: Arc<Vec<u8>>,
        api_builder: ApiBuilderFn,
    ) -> Self {
        Self {
            pid,
            memory,
            stdout,
            cwd,
            module,
            api_builder,
        }
    }

    /// Build form a builder fn
    pub fn build_api(self: &Arc<Self>) -> JsValue {
        let mut api_module_builder = ApiModuleBuilder::new();
        (self.api_builder)(self.clone(), &mut api_module_builder);
        api_module_builder.build()
    }

    /// Get the process id
    pub fn pid(&self) -> Uuid {
        self.pid
    }

    /// Get the memory of the wasm module
    pub fn memory<'a>(&'a self) -> MutexGuard<'a, Memory> {
        let memory = self.memory.lock().unwrap(); // Spin locking blocks the entire process. Figure out a way to not block the entire process
        memory
    }

    /// Get the memory of the wasm module without spinning
    pub fn memory_nospin<'a>(&'a self) -> MutexGuard<'a, Memory> {
        self.memory.lock().unwrap()
    }

    /// Get the stdout messenger of the wasm module
    pub fn stdout(&self) -> Arc<ProcessStdOut> {
        self.stdout.clone()
    }

    /// Get the working directory
    pub fn cwd(&self) -> String {
        self.cwd.read().unwrap().clone()
    }

    /// Get the module
    pub fn module(&self) -> Arc<Vec<u8>> {
        self.module.clone()
    }

    /// Set the working directory
    pub fn set_cwd(&self, wd: &str) {
        let wd = honeyos_fs::util::normalize_path(wd);
        let mut cwd = self.cwd.spin_write().unwrap();
        *cwd = wd;
    }

    /// Create a new copy for this worker
    pub fn new_worker(&self, memory_inner: WebAssembly::Memory) -> Self {
        let memory = self.memory.spin_lock().unwrap();
        let new_memory = Arc::new(Mutex::new(memory.new_inner(memory_inner)));
        let mut clone = self.clone();
        clone.memory = new_memory;
        clone
    }
}

impl ApiModuleBuilder {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Register an item
    pub fn register(&mut self, name: impl Into<String>, value: JsValue) -> &mut Self {
        let name: String = name.into();
        if self.values.contains_key(&name) {
            self.values.remove(&name);
        }
        self.values.insert(name, value);
        self
    }

    /// Build the module object
    pub fn build(self) -> JsValue {
        let imports = JSON::parse("{}").unwrap();
        for (name, value) in self.values.iter() {
            Reflect::set(&imports, &name.into(), value).unwrap();
        }
        imports
    }
}
