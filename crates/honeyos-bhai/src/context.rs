use std::sync::Arc;

use rhai::Engine;
use uuid::Uuid;

/// A method that sets up the scope
pub type ScopeBuilderFn = fn(Arc<Context>, &mut Engine);

/// The context for the api methods
#[derive(Debug)]
pub struct Context {
    pid: Uuid,
}

impl Context {
    pub fn new(pid: Uuid) -> Self {
        Self { pid }
    }

    /// Get the pid
    pub fn pid(&self) -> Uuid {
        self.pid
    }
}
