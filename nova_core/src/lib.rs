#![cfg_attr(all(not(feature = "ffi")), forbid(unsafe_code))]

//! NovaCore v0.1 – a lightweight runtime and bytecode execution engine.
//!
//! The crate exposes three major building blocks:
//!
//! * [`backend`] – architecture-specific code generation and execution backends
//!   selected at compile time via Cargo features.
//! * [`bytecode`] – definition of the NovaBytecode format and an assembler capable of
//!   lowering a small high-level AST into a compact instruction stream.
//! * [`NovaRuntime`] – an embeddable runtime with a cost-metered interpreter capable of
//!   executing the bytecode produced by the assembler.
//! * [`sys`] – cross platform utilities for IO and networking that NovaRuntime and higher
//!   level applications can depend on.
//!
//! The implementation purposely focuses on being easy to audit and extend. The public API is
//! designed so other NovaOS crates (CLI, IDE, or third party tools) can share the runtime
//! without leaking internal details.  Unsafe code is avoided by default and only available
//! behind the optional `ffi` feature flag where raw pointers are unavoidable.

pub mod backend;
pub mod bytecode;
pub mod concurrency;
pub mod ffi;
pub mod integration;
pub mod memory;
pub mod module;
pub mod novac;
pub mod sys;

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use parking_lot::RwLock;
use rand::thread_rng;
use thiserror::Error;
use tracing::instrument;

use crate::bytecode::spec::DebugSymbol;
use crate::concurrency::executor::{LoopState, TaskExecutor};
use crate::integration::RuntimeHooks;
use crate::module::{Module, ModuleLoader};
use crate::sys::drivers::DriverRegistry;
use crate::sys::hal::{HardwareAbstractionLayer, SoftwareHal};

#[derive(Clone)]
struct DebugHal(Arc<dyn HardwareAbstractionLayer>);

impl DebugHal {
    fn new(hal: Arc<dyn HardwareAbstractionLayer>) -> Self {
        Self(hal)
    }

    fn clone_arc(&self) -> Arc<dyn HardwareAbstractionLayer> {
        self.0.clone()
    }

    fn driver_registry(&self) -> DriverRegistry {
        self.0.driver_registry()
    }
}

impl fmt::Debug for DebugHal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HardwareAbstractionLayer").finish()
    }
}

/// Result type used across NovaCore.
pub type NovaResult<T> = std::result::Result<T, NovaError>;

/// Values manipulated by the NovaCore runtime.
#[derive(Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Object(memory::gc::GcRef),
}

impl Value {
    pub(crate) fn from_constant(constant: &bytecode::spec::Constant) -> Self {
        use bytecode::spec::Constant;
        match constant {
            Constant::Null => Value::Null,
            Constant::Boolean(b) => Value::Boolean(*b),
            Constant::Integer(i) => Value::Integer(*i),
            Constant::Float(f) => Value::Float(*f),
            Constant::String(s) => Value::String(s.clone()),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Boolean(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Object(_) => "object",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Integer(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Object(_) => true,
        }
    }

    pub(crate) fn as_number(&self) -> Result<f64, String> {
        match self {
            Value::Integer(value) => Ok(*value as f64),
            Value::Float(value) => Ok(*value),
            Value::Boolean(value) => Ok(if *value { 1.0 } else { 0.0 }),
            Value::Null => Ok(0.0),
            other => Err(format!(
                "{} cannot be converted to number",
                other.type_name()
            )),
        }
    }

    pub(crate) fn trace(&self, visitor: &mut dyn FnMut(memory::gc::GcRef)) {
        if let Value::Object(reference) = self {
            visitor(*reference);
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Boolean(b) => write!(f, "Boolean({b})"),
            Value::Integer(i) => write!(f, "Integer({i})"),
            Value::Float(val) => write!(f, "Float({val})"),
            Value::String(s) => write!(f, "String({s:?})"),
            Value::Object(obj) => write!(f, "Object({:?})", obj.handle()),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

/// Configuration for the runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum amount of interpreter steps before execution aborts.
    pub cost_limit: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self { cost_limit: 10_000 }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NovaError {
    #[error("execution blocked by fail-safe gate")]
    FailSafeLocked,
    #[error("fail-safe passphrase mismatch")]
    FailSafeAuthFailed,
    #[error("bytecode error: {0}")]
    Bytecode(String),
    #[error("runtime stack underflow")]
    StackUnderflow,
    #[error("runtime exceeded cost limit")]
    CostLimitExceeded,
    #[error("invalid cost limit: {0}")]
    InvalidCostLimit(String),
    #[error("unexpected runtime state: {0}")]
    Internal(String),
    #[error("{message}")]
    RuntimeException {
        message: String,
        stack: Vec<StackFrame>,
    },
    #[error("native error: {0}")]
    Native(String),
}

impl From<bytecode::spec::NovaBytecodeError> for NovaError {
    fn from(err: bytecode::spec::NovaBytecodeError) -> Self {
        NovaError::Bytecode(err.to_string())
    }
}

/// Captured stack frame used to report runtime errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    pub function: String,
    pub location: Option<DebugSymbol>,
}

/// Fail-safe state shared by the runtime.
#[derive(Default, Debug)]
struct FailSafeState {
    inner: RwLock<Option<FailSafeConfig>>,
}

#[derive(Debug, Clone)]
struct FailSafeConfig {
    hash: String,
    unlocked: bool,
}

impl FailSafeState {
    fn enable(&self, passphrase: &str) -> Result<()> {
        let salt = SaltString::generate(&mut thread_rng());
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(passphrase.as_bytes(), &salt)
            .map_err(|err| anyhow!(err.to_string()))?
            .to_string();
        *self.inner.write() = Some(FailSafeConfig {
            hash,
            unlocked: false,
        });
        Ok(())
    }

    fn clear(&self) {
        *self.inner.write() = None;
    }

    fn authenticate(&self, passphrase: &str) -> Result<(), NovaError> {
        let mut guard = self.inner.write();
        let Some(config) = guard.as_mut() else {
            return Err(NovaError::FailSafeLocked);
        };
        let parsed =
            PasswordHash::new(&config.hash).map_err(|err| NovaError::Internal(err.to_string()))?;
        if Argon2::default()
            .verify_password(passphrase.as_bytes(), &parsed)
            .is_ok()
        {
            config.unlocked = true;
            Ok(())
        } else {
            Err(NovaError::FailSafeAuthFailed)
        }
    }

    fn ensure_unlocked(&self) -> Result<(), NovaError> {
        let guard = self.inner.read();
        match guard.as_ref() {
            Some(cfg) if cfg.unlocked => Ok(()),
            Some(_) => Err(NovaError::FailSafeLocked),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeBootstrap {
    stack_bytes: usize,
    heap_bytes: usize,
    worker_count: Option<usize>,
}

impl RuntimeBootstrap {
    pub fn deterministic() -> Self {
        Self {
            stack_bytes: 256 * 1024,
            heap_bytes: 4 * 1024 * 1024,
            worker_count: None,
        }
    }

    pub fn with_stack_bytes(mut self, stack_bytes: usize) -> Self {
        self.stack_bytes = stack_bytes.max(64 * 1024);
        self
    }

    pub fn with_heap_bytes(mut self, heap_bytes: usize) -> Self {
        self.heap_bytes = heap_bytes.max(1024 * 1024);
        self
    }

    pub fn with_worker_count(mut self, workers: usize) -> Self {
        self.worker_count = Some(workers.max(1));
        self
    }

    pub fn build(self) -> RuntimeBootstrapState {
        RuntimeBootstrapState::from_config(self)
    }
}

#[derive(Debug)]
pub struct RuntimeBootstrapState {
    stack: Arc<StackReservation>,
    memory: Arc<memory::MemoryContract>,
    worker_count: usize,
}

impl RuntimeBootstrapState {
    fn from_config(config: RuntimeBootstrap) -> Self {
        let worker_count = config.worker_count.unwrap_or_else(default_worker_count);
        Self {
            stack: Arc::new(StackReservation::new(config.stack_bytes)),
            memory: Arc::new(memory::MemoryContract::new(config.heap_bytes)),
            worker_count,
        }
    }

    pub fn stack_bytes(&self) -> usize {
        self.stack.size()
    }

    pub fn memory_contract(&self) -> Arc<memory::MemoryContract> {
        self.memory.clone()
    }

    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    fn into_parts(self) -> (Arc<StackReservation>, Arc<memory::MemoryContract>, usize) {
        (self.stack, self.memory, self.worker_count)
    }
}

#[derive(Debug)]
struct StackReservation {
    guard: Vec<u8>,
}

impl StackReservation {
    fn new(size: usize) -> Self {
        let size = size.max(64 * 1024);
        Self {
            guard: vec![0; size],
        }
    }

    fn size(&self) -> usize {
        self.guard.len()
    }
}

fn default_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// NovaRuntime orchestrates backend selection, execution, and provides fail-safe gating.
#[derive(Debug, Clone)]
pub struct NovaRuntime {
    config: RuntimeConfig,
    failsafe: Arc<FailSafeState>,
    modules: Arc<RwLock<ModuleLoader>>,
    hooks: Arc<RuntimeHooks>,
    hal: DebugHal,
    stack: Arc<StackReservation>,
    memory: Arc<memory::MemoryContract>,
    executor: TaskExecutor,
}

impl Default for NovaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl NovaRuntime {
    /// Creates a new runtime with default configuration.
    pub fn new() -> Self {
        Self::from_bootstrap(RuntimeBootstrap::deterministic().build())
    }

    /// Returns a bootstrap builder for constructing deterministic runtime instances.
    pub fn bootstrap() -> RuntimeBootstrap {
        RuntimeBootstrap::deterministic()
    }

    /// Constructs the runtime from an explicit bootstrap state.
    pub fn from_bootstrap(state: RuntimeBootstrapState) -> Self {
        let (stack, memory, worker_count) = state.into_parts();
        let hooks = Arc::new(RuntimeHooks::default());
        let target = backend::active_target();
        let hal_impl = SoftwareHal::new(target, hooks.clone());
        let _ = hal_impl.register_builtin_devices();
        let hal = Arc::new(hal_impl) as Arc<dyn HardwareAbstractionLayer>;
        let executor = TaskExecutor::new(worker_count);
        Self {
            config: RuntimeConfig::default(),
            failsafe: Arc::new(FailSafeState::default()),
            modules: Arc::new(RwLock::new(ModuleLoader::new())),
            hooks,
            hal: DebugHal::new(hal),
            stack,
            memory,
            executor,
        }
    }

    /// Overrides the cost limit used by the interpreter.
    pub fn with_cost_limit(mut self, cost_limit: u64) -> NovaResult<Self> {
        if cost_limit == 0 {
            return Err(NovaError::InvalidCostLimit(
                "cost limit must be greater than zero".into(),
            ));
        }
        self.config.cost_limit = cost_limit;
        Ok(self)
    }

    /// Enables the fail-safe and stores the hashed passphrase.
    pub fn enable_failsafe(&self, passphrase: &str) -> NovaResult<()> {
        self.failsafe
            .enable(passphrase)
            .map_err(|err| NovaError::Internal(err.to_string()))
    }

    /// Clears the stored passphrase and unlocks execution.
    pub fn disable_failsafe(&self) {
        self.failsafe.clear();
    }

    /// Authenticates against the fail-safe gate.
    pub fn authenticate(&self, passphrase: &str) -> NovaResult<()> {
        self.failsafe.authenticate(passphrase)
    }

    /// Executes the provided bytecode.
    #[instrument(skip_all)]
    pub fn execute(&self, bytes: &[u8]) -> NovaResult<Value> {
        self.failsafe.ensure_unlocked()?;
        let module = self.modules.write().load_bytes("__entry", bytes)?;
        let backend = backend::active_backend();
        let artifact = backend.compile(module.bytecode())?;
        backend.execute(artifact, self.config.clone(), self.modules.clone())
    }

    /// Returns the active backend implementation.
    pub fn backend(&self) -> &'static dyn backend::ArchitectureBackend {
        backend::active_backend()
    }

    /// Returns the target architecture selected at compile time.
    pub fn target_arch(&self) -> backend::TargetArch {
        backend::active_target()
    }

    /// Loads a module into the runtime from raw bytes.
    pub fn load_module_bytes(
        &self,
        name: impl Into<String>,
        bytes: &[u8],
    ) -> NovaResult<Arc<Module>> {
        self.modules.write().load_bytes(name, bytes)
    }

    /// Loads a module from the filesystem.
    pub fn load_module_file(&self, path: impl AsRef<Path>) -> NovaResult<Arc<Module>> {
        self.modules.write().load_file(path)
    }

    /// Returns a list of loaded modules.
    pub fn modules(&self) -> Vec<Arc<Module>> {
        self.modules.read().modules().cloned().collect()
    }

    /// Returns the runtime hooks used for debugger/logging integrations.
    pub fn hooks(&self) -> Arc<RuntimeHooks> {
        self.hooks.clone()
    }

    /// Returns the hardware abstraction layer backing the runtime.
    pub fn hal(&self) -> Arc<dyn HardwareAbstractionLayer> {
        self.hal.clone_arc()
    }

    /// Convenience accessor for the driver registry.
    pub fn driver_registry(&self) -> DriverRegistry {
        self.hal.driver_registry()
    }

    /// Returns the deterministic stack reservation (in bytes) used by the runtime.
    pub fn stack_bytes(&self) -> usize {
        self.stack.size()
    }

    /// Returns the shared memory contract used for manual allocations.
    pub fn memory_contract(&self) -> Arc<memory::MemoryContract> {
        self.memory.clone()
    }

    /// Returns a task executor backed by the runtime's work-stealing scheduler.
    pub fn executor(&self) -> TaskExecutor {
        self.executor.clone()
    }

    /// Runs the unified runtime loop until all scheduled tasks have completed.
    pub fn run_loop(&self) -> NovaResult<()> {
        loop {
            if self.executor.poll_once() == LoopState::Idle {
                break;
            }
        }
        Ok(())
    }

    /// Executes a module that has already been loaded into the runtime.
    pub fn execute_module(&self, module: Arc<Module>) -> NovaResult<Value> {
        self.failsafe.ensure_unlocked()?;
        let backend = backend::active_backend();
        let artifact = backend.compile(module.bytecode())?;
        backend.execute(artifact, self.config.clone(), self.modules.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::assemble;

    #[test]
    fn runtime_executes_simple_program() {
        let ast = bytecode::ast::Ast::from_expr(bytecode::ast::Expr::binary(
            bytecode::ast::BinaryOp::Add,
            bytecode::ast::Expr::number(1.0),
            bytecode::ast::Expr::number(2.0),
        ));
        let bytecode = assemble(&ast).expect("assemble");
        let runtime = NovaRuntime::new();
        let value = runtime
            .execute(&bytecode.into_bytes())
            .expect("execution succeeds");
        assert_eq!(value, Value::Float(3.0));
    }

    #[test]
    fn failsafe_blocks_execution() {
        let ast = bytecode::ast::Ast::from_expr(bytecode::ast::Expr::number(0.0));
        let bytecode = assemble(&ast).expect("assemble");
        let runtime = NovaRuntime::new();
        runtime.enable_failsafe("nova").unwrap();
        let bytes = bytecode.into_bytes();
        let err = runtime.execute(&bytes).unwrap_err();
        assert_eq!(err, NovaError::FailSafeLocked);
        runtime.authenticate("nova").unwrap();
        runtime.execute(&bytes).unwrap();
    }

    #[test]
    fn cost_limit_validation() {
        assert!(NovaRuntime::new().with_cost_limit(0).is_err());
    }

    #[test]
    fn exposes_backend_target() {
        let runtime = NovaRuntime::new();
        let backend = runtime.backend();
        assert_eq!(backend.name(), runtime.target_arch().as_str());
    }

    #[test]
    fn exposes_memory_contract() {
        let runtime = NovaRuntime::new();
        let stats = runtime.memory_contract().stats();
        assert_eq!(stats.used_bytes, 0);
        assert!(stats.capacity_bytes >= 1024 * 1024);
    }

    #[test]
    fn executor_runs_jobs() {
        let runtime = NovaRuntime::new();
        let executor = runtime.executor();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = flag.clone();
        let handle = executor.spawn(move || {
            done.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        handle.wait();
        assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn run_loop_returns_when_idle() {
        let runtime = NovaRuntime::new();
        runtime.run_loop().expect("loop finishes immediately");
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = flag.clone();
        runtime.executor().spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            done.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        runtime.run_loop().expect("loop drains pending tasks");
        assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
    }
}
