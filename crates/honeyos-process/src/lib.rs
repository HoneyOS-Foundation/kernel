use std::sync::{Arc, Mutex, Once};

use context::ApiBuilderFn;
use hashbrown::{
    hash_map::{Values, ValuesMut},
    HashMap,
};
use honeyos_bhai::context::ScopeBuilderFn;
use process::Process;
use thread::ThreadRequest;
use uuid::Uuid;

pub mod context;
pub mod memory;
pub mod process;
pub mod requirements;
pub mod stdout;
pub mod thread;

static mut PROCESS_MANAGER: Option<Arc<Mutex<ProcessManager>>> = None;

/// A manager for the seperate processes in honeyos
pub struct ProcessManager {
    api_builder: ApiBuilderFn,
    processes: HashMap<Uuid, Process>,
    spawn_requests: Vec<Uuid>,           // Spawns are handled by the kernel
    thread_requests: Vec<ThreadRequest>, // Thread spawn requests are also handled by the kernel as chrome does not support nested web workers
}

impl ProcessManager {
    /// Initialize the process manager.
    /// Should only be called once.
    pub fn init_once(api_builder: ApiBuilderFn) {
        static SET_HOOK: Once = Once::new();
        SET_HOOK.call_once(|| unsafe {
            PROCESS_MANAGER = Some(Arc::new(Mutex::new(ProcessManager {
                api_builder,
                processes: HashMap::new(),
                spawn_requests: Vec::new(),
                thread_requests: Vec::new(),
            })));
        });
    }

    /// Get the static instance
    pub fn get() -> Arc<Mutex<ProcessManager>> {
        unsafe {
            PROCESS_MANAGER
                .as_ref()
                .expect("Process manager has not been initialized")
                .clone()
        }
    }

    /// Spawn a process
    pub fn spawn(
        &mut self,
        wasm_bin: Vec<u8>,
        title: Option<&str>,
        working_directory: &str,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        let title = if let Some(title) = title {
            title.to_string()
        } else {
            id.to_string()
        };
        // Insert the process into the hashmap
        let process =
            Process::new(id, wasm_bin, &title, working_directory, self.api_builder).unwrap();
        self.processes.insert(id, process);

        // Spawn the process
        self.spawn_requests.push(id);
        Ok(id)
    }

    /// Spawn a thread for a process
    pub fn spawn_thread(&mut self, pid: Uuid, fptr: u32) {
        self.thread_requests.push(ThreadRequest { pid, fptr });
    }

    /// Check for the status of each process and remove those no longer running
    pub fn update(&mut self) {
        // Remove dead processes
        let mut dead = Vec::new();
        for (id, process) in self.processes.iter_mut() {
            if !process.is_alive() {
                dead.push(*id);
            }
        }
        for id in dead {
            self.processes.remove(&id);
        }

        // Handle spawn requests
        for request in self.spawn_requests.iter() {
            let process = self.processes.get_mut(request).unwrap();
            process.spawn().unwrap();
        }
        self.spawn_requests.clear();

        // Handle thread requests
        for request in self.thread_requests.iter() {
            let Some(process) = self.processes.get_mut(&request.pid) else {
                continue;
            };
            if let Err(e) = process.spawn_thread(request.fptr) {
                log::error!(
                    "Failed to spawn thread for process `{}`: {}",
                    request.pid,
                    e
                );
            }
        }
        self.thread_requests.clear();
    }
}

impl ProcessManager {
    /// Get all the processes
    pub fn processes(&self) -> Values<Uuid, Process> {
        self.processes.values()
    }

    /// Get all the processes
    pub fn processes_mut(&mut self) -> ValuesMut<Uuid, Process> {
        self.processes.values_mut()
    }

    /// Get a process
    pub fn process(&self, id: Uuid) -> Option<&Process> {
        self.processes.get(&id)
    }

    /// Get a process
    pub fn process_mut(&mut self, id: Uuid) -> Option<&mut Process> {
        self.processes.get_mut(&id)
    }

    /// Get the current api builder function
    pub fn api_builder(&self) -> ApiBuilderFn {
        self.api_builder
    }

    /// Get the spawn requests
    pub fn requests(&self) -> &[Uuid] {
        &self.spawn_requests
    }

    /// Get the spawn requests
    pub fn requests_mut(&mut self) -> &mut Vec<Uuid> {
        &mut self.spawn_requests
    }
}
