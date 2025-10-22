use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::{NovaError, Value};

/// Hooks consumed by NovaShell and NovaIDE for runtime integration.
///
/// The hooks are lightweight observer callbacks invoked for debugger events,
/// structured logging, and telemetry updates.  Callbacks are optional and can
/// be registered or cleared at runtime without requiring mutable access to the
/// [`NovaRuntime`](crate::NovaRuntime) instance.
#[derive(Default, Clone)]
pub struct RuntimeHooks {
    debugger: Arc<RwLock<Option<DebuggerCallback>>>,
    logger: Arc<RwLock<Option<LoggerCallback>>>,
    telemetry: Arc<RwLock<Option<TelemetryCallback>>>,
}

type DebuggerCallback = Arc<dyn Fn(&DebuggerEvent) + Send + Sync + 'static>;
type LoggerCallback = Arc<dyn Fn(&RuntimeLog) + Send + Sync + 'static>;
type TelemetryCallback = Arc<dyn Fn(&TelemetryEvent) + Send + Sync + 'static>;

impl RuntimeHooks {
    /// Registers a debugger callback.
    pub fn set_debugger<F>(&self, hook: F)
    where
        F: Fn(&DebuggerEvent) + Send + Sync + 'static,
    {
        *self.debugger.write() = Some(Arc::new(hook));
    }

    /// Registers a structured logger callback.
    pub fn set_logger<F>(&self, hook: F)
    where
        F: Fn(&RuntimeLog) + Send + Sync + 'static,
    {
        *self.logger.write() = Some(Arc::new(hook));
    }

    /// Registers a telemetry callback.
    pub fn set_telemetry<F>(&self, hook: F)
    where
        F: Fn(&TelemetryEvent) + Send + Sync + 'static,
    {
        *self.telemetry.write() = Some(Arc::new(hook));
    }

    /// Clears the registered debugger callback.
    pub fn clear_debugger(&self) {
        *self.debugger.write() = None;
    }

    /// Clears the registered logger callback.
    pub fn clear_logger(&self) {
        *self.logger.write() = None;
    }

    /// Clears the registered telemetry callback.
    pub fn clear_telemetry(&self) {
        *self.telemetry.write() = None;
    }

    /// Emits a debugger event to the registered callback.
    pub fn emit_debugger(&self, event: DebuggerEvent) {
        if let Some(callback) = self.debugger.read().as_ref().cloned() {
            callback(&event);
        }
    }

    /// Emits a runtime log message.
    pub fn emit_log(&self, log: RuntimeLog) {
        if let Some(callback) = self.logger.read().as_ref().cloned() {
            callback(&log);
        }
    }

    /// Emits a telemetry event.
    pub fn emit_telemetry(&self, event: TelemetryEvent) {
        if let Some(callback) = self.telemetry.read().as_ref().cloned() {
            callback(&event);
        }
    }
}

impl fmt::Debug for RuntimeHooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeHooks").finish_non_exhaustive()
    }
}

/// Debugger oriented lifecycle events emitted by the runtime.
#[derive(Debug, Clone)]
pub enum DebuggerEvent {
    ExecutionStarted { module: String },
    ExecutionFinished { module: String, result: Value },
    ExecutionFailed { module: String, error: NovaError },
}

/// Structured log entry surfaced to host tooling.
#[derive(Debug, Clone)]
pub struct RuntimeLog {
    pub source: &'static str,
    pub message: String,
}

impl RuntimeLog {
    pub fn new(source: &'static str, message: impl Into<String>) -> Self {
        Self {
            source,
            message: message.into(),
        }
    }
}

/// Telemetry events published for NovaShell / NovaIDE dashboards.
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    ModuleLoaded {
        name: String,
    },
    ShellLoaded {
        path: PathBuf,
    },
    DriverRegistered {
        name: String,
        registers: usize,
    },
    RegisterWrite {
        name: String,
        register: usize,
        value: u32,
    },
    InterruptRaised {
        name: String,
        irq: u32,
        payload: Option<u32>,
    },
}
