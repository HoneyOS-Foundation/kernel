use rhai::Engine;
use uuid::Uuid;

/// The shell context for a process
pub struct ShellContext {
    pid: Uuid,
    engine: Engine,
}

impl ShellContext {
    pub fn new(pid: Uuid) -> Self {
        Self {
            pid,
            engine: Engine::new(),
        }
    }

    /// Run a command
    pub fn run(&self, cmd: &str) {
        self.engine.run(cmd).unwrap()
    }

    /// Get the pid
    pub fn pid(&self) -> Uuid {
        self.pid
    }
}
