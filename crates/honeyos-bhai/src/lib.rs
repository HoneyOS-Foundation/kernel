pub mod context;
pub mod syntax;

pub use rhai;
use syntax::register_syntax;

use std::sync::Arc;

use context::{Context, ScopeBuilderFn};
use rhai::Engine;
use uuid::Uuid;

/// The shell context for a process
pub struct Scope {
    context: Arc<Context>,
    engine: Engine,
}

impl Scope {
    pub fn new(pid: Uuid, setup_fn: ScopeBuilderFn) -> Self {
        let context = Arc::new(Context::new(pid));
        let mut engine = Engine::new();

        register_syntax(&mut engine).unwrap();
        setup_fn(context.clone(), &mut engine);

        Self { context, engine }
    }

    /// Run a command
    pub fn run(&self, cmd: &str) -> anyhow::Result<()> {
        self.engine.run(cmd).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    /// Get the pid
    pub fn pid(&self) -> Uuid {
        self.context.pid()
    }

    /// Get the engine
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}
