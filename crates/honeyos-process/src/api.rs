use std::sync::{Arc, Mutex, MutexGuard};

use hashbrown::HashMap;
use uuid::Uuid;
use wasm_bindgen::JsValue;
use web_sys::js_sys::{Reflect, WebAssembly, JSON};

use crate::{memory::Memory, stdout::StdoutMessage};

/// A function responsible for building the api for wasm processes
pub type ApiBuilderFn = fn(Arc<ApiModuleCtx>, &mut ApiModuleBuilder);

/// The api module
pub struct ApiModuleCtx {
    pid: Uuid,
    stdout: Arc<Mutex<Vec<StdoutMessage>>>,
    memory: Arc<Mutex<Memory>>,
    table: Arc<WebAssembly::Table>,
}

impl ApiModuleCtx {
    pub fn new(
        pid: Uuid,
        memory: Arc<Mutex<Memory>>,
        table: Arc<WebAssembly::Table>,
        stdout: Arc<Mutex<Vec<StdoutMessage>>>,
    ) -> Self {
        Self {
            pid,
            memory,
            table,
            stdout,
        }
    }

    /// Build form a builder fn
    pub fn js_from_fn(f: ApiBuilderFn, context: Arc<ApiModuleCtx>) -> JsValue {
        let mut api_module_builder = ApiModuleBuilder::new();
        f(context, &mut api_module_builder);
        api_module_builder.build()
    }

    /// Get the process id
    pub fn pid(&self) -> Uuid {
        self.pid
    }

    /// Get the memory of the wasm module
    pub fn memory<'a>(&'a self) -> MutexGuard<'a, Memory> {
        loop {
            let Ok(memory) = self.memory.try_lock() else {
                continue;
            };
            return memory;
        }
    }

    /// Get the table
    pub fn table(&self) -> Arc<WebAssembly::Table> {
        self.table.clone()
    }

    /// Get the stdout messenger of the wasm module
    pub fn stdout(&self) -> Arc<Mutex<Vec<StdoutMessage>>> {
        self.stdout.clone()
    }
}

/// The builder for an api module
#[derive(Debug, Clone)]
pub struct ApiModuleBuilder {
    values: HashMap<String, JsValue>,
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
