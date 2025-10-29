use nova_core::memory::{MemoryContract, MemoryError, MemoryHandle, MemoryStats};
use nova_core::module::Module;
use nova_core::sys::hal::HardwareAbstractionLayer;
use nova_core::{NovaError, NovaRuntime, Value as CoreValue};
use parking_lot::Mutex;
use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ModuleRegistration {
    pub memory: MemoryHandle,
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug)]
pub struct CoreBridge {
    runtime: NovaRuntime,
    memory: Arc<MemoryContract>,
    modules: Mutex<HashMap<MemoryHandle, Arc<Module>>>,
}

impl CoreBridge {
    pub fn new() -> Self {
        let runtime = NovaRuntime::new();
        let memory = runtime.memory_contract();
        Self {
            runtime,
            memory,
            modules: Mutex::new(HashMap::new()),
        }
    }

    pub fn runtime(&self) -> &NovaRuntime {
        &self.runtime
    }

    pub fn hal(&self) -> Arc<dyn HardwareAbstractionLayer> {
        self.runtime.hal()
    }

    pub fn memory(&self) -> Arc<MemoryContract> {
        self.memory.clone()
    }

    pub fn memory_stats(&self) -> MemoryStats {
        self.memory.stats()
    }

    pub fn load_compiled_module(&self, path: &Path) -> Result<ModuleRegistration, NovaError> {
        let module = self.runtime.load_module_file(path)?;
        let bytecode = module.bytecode();
        let size_hint = bytecode.functions().len() * 64 + bytecode.constants().len() * 32;
        let payload: Arc<dyn Any + Send + Sync> = module.clone();
        let memory = self
            .memory
            .allocate_arc(payload, size_hint)
            .map_err(|err| NovaError::Internal(err.to_string()))?;
        self.modules.lock().insert(memory, module.clone());
        Ok(ModuleRegistration {
            memory,
            path: path.to_path_buf(),
            name: module.name().to_string(),
        })
    }

    pub fn execute_module(&self, handle: MemoryHandle) -> Result<CoreValue, NovaError> {
        let module = {
            let modules = self.modules.lock();
            modules.get(&handle).cloned().ok_or_else(|| {
                NovaError::Internal(format!("Unknown module handle {}", handle.raw()))
            })?
        };
        self.runtime.execute_module(module)
    }

    pub fn allocate_value(&self, value: CoreValue) -> Result<MemoryHandle, MemoryError> {
        let size_hint = estimate_value_size(&value);
        let payload: Arc<dyn Any + Send + Sync> = Arc::new(value);
        self.memory.allocate_arc(payload, size_hint)
    }

    pub fn value(&self, handle: MemoryHandle) -> Result<Arc<CoreValue>, MemoryError> {
        self.memory.downcast_arc::<CoreValue>(handle)
    }

    pub fn release(&self, handle: MemoryHandle) -> bool {
        self.modules.lock().remove(&handle);
        self.memory.release(handle)
    }

    /// Enqueues a NovaScript task onto NovaCore's executor.
    ///
    /// This bridge is the foundation for the unified runtime loop; call sites can
    /// submit interpreter work that will be driven by [`NovaRuntime::run_loop`].
    pub fn execute_async<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.runtime.executor().spawn(job);
    }
}

fn estimate_value_size(value: &CoreValue) -> usize {
    match value {
        CoreValue::Null | CoreValue::Boolean(_) => std::mem::size_of::<CoreValue>(),
        CoreValue::Integer(_) | CoreValue::Float(_) => std::mem::size_of::<CoreValue>(),
        CoreValue::String(s) => std::mem::size_of::<CoreValue>() + s.len(),
        CoreValue::Object(_) => std::mem::size_of::<CoreValue>() * 2,
    }
}
