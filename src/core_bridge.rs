#![allow(dead_code)]

use crate::interpreter::Interpreter;
use crate::modules::ModuleLoader;
use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use crate::vm::stack_vm::StackVm;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use solvra_core::memory::{MemoryContract, MemoryError, MemoryHandle, MemoryStats};
use solvra_core::module::Module;
use solvra_core::sys::hal::HardwareAbstractionLayer;
use solvra_core::vm::loader::ModuleLoaderVm;
use solvra_core::{
    SolvraError as SolvraCoreError, SolvraResult as SolvraCoreResult, SolvraRuntime,
    Value as CoreValue,
};
use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

type ModuleTable = HashMap<MemoryHandle, Arc<Module>>;

static GLOBAL: OnceCell<Arc<CoreBridge>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct ModuleRegistration {
    pub memory: MemoryHandle,
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug)]
pub struct CoreBridge {
    runtime: Arc<SolvraRuntime>,
    memory: Arc<MemoryContract>,
    modules: Arc<Mutex<ModuleTable>>,
    vm_loader: ModuleLoaderVm,
}

impl Default for CoreBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreBridge {
    pub fn new() -> Self {
        let runtime = Arc::new(SolvraRuntime::new());
        let memory = runtime.memory_contract();
        let vm_loader = ModuleLoaderVm::new(memory.clone());
        Self {
            runtime,
            memory,
            modules: Arc::new(Mutex::new(HashMap::new())),
            vm_loader,
        }
    }

    pub fn install_global(instance: Arc<Self>) {
        let _ = GLOBAL.set(instance);
    }

    pub fn global() -> Arc<Self> {
        GLOBAL
            .get()
            .cloned()
            .expect("CoreBridge global instance not initialized")
    }

    pub fn runtime(&self) -> &SolvraRuntime {
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

    pub fn load_compiled_module(&self, path: &Path) -> Result<ModuleRegistration, SolvraCoreError> {
        let module = self.runtime.load_module_file(path)?;
        let bytecode = module.bytecode();
        let size_hint = bytecode.functions().len() * 64 + bytecode.constants().len() * 32;
        let payload: Arc<dyn Any + Send + Sync> = module.clone();
        let memory = self
            .memory
            .allocate_arc(payload, size_hint)
            .map_err(|err| SolvraCoreError::Internal(err.to_string()))?;
        self.modules.lock().insert(memory, module.clone());
        Ok(ModuleRegistration {
            memory,
            path: path.to_path_buf(),
            name: module.name().to_string(),
        })
    }

    pub fn execute_loaded_module(
        &self,
        handle: MemoryHandle,
    ) -> Result<CoreValue, SolvraCoreError> {
        let module = {
            let modules = self.modules.lock();
            modules.get(&handle).cloned().ok_or_else(|| {
                SolvraCoreError::Internal(format!("Unknown module handle {}", handle.raw()))
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

    /// Schedule a script source to execute on SolvraCore's deterministic executor.
    pub fn schedule_script(&self, source: &str) -> SolvraCoreResult<()> {
        let script = source.to_owned();
        let runtime = self.runtime.clone();
        let bridge = Arc::new(self.clone_for_spawn());
        runtime.executor().spawn(move || {
            let mut tokenizer = Tokenizer::new(&script);
            let tokens = match tokenizer.tokenize() {
                Ok(tokens) => tokens,
                Err(err) => {
                    eprintln!("Tokenizer error in scheduled script: {err}");
                    return;
                }
            };
            let mut parser = Parser::new(tokens);
            let program = match parser.parse() {
                Ok(program) => program,
                Err(err) => {
                    eprintln!("Parse error in scheduled script: {err}");
                    return;
                }
            };
            let loader = Rc::new(RefCell::new(ModuleLoader::new()));
            let mut interpreter = Interpreter::with_loader(loader, bridge.clone());
            if let Err(err) = interpreter.eval_program(&program) {
                eprintln!("Interpreter error: {err}");
            }
        });
        Ok(())
    }

    pub fn execute_module(&self, path: &str) -> SolvraCoreResult<()> {
        let loaded = self.runtime.load_vm_module(path)?;
        let bytecode = loaded.bytecode.clone();
        let handle = loaded.handle;
        let memory = self.memory.clone();
        let runtime = self.runtime.clone();
        println!("[VM] Executing module: {path}");
        runtime.executor().spawn(move || {
            let mut vm = StackVm::new(bytecode);
            if let Err(err) = vm.execute() {
                eprintln!("VM execution error: {err}");
            }
            if !memory.release(handle) {
                eprintln!(
                    "[VM] failed to release module handle {} after execution",
                    handle.raw()
                );
            }
        });
        Ok(())
    }

    pub fn load_vm_module_from_memory(&self, bytes: Vec<u8>) -> SolvraCoreResult<()> {
        let loaded = self.runtime.load_vm_module_from_memory(&bytes)?;
        let bytecode = loaded.bytecode.clone();
        let handle = loaded.handle;
        let memory = self.memory.clone();
        let runtime = self.runtime.clone();
        runtime.executor().spawn(move || {
            let mut vm = StackVm::new(bytecode);
            if let Err(err) = vm.execute() {
                eprintln!("VM execution error: {err}");
            }
            if !memory.release(handle) {
                eprintln!(
                    "[VM] failed to release module handle {} after execution",
                    handle.raw()
                );
            }
        });
        Ok(())
    }
}

impl CoreBridge {
    fn clone_for_spawn(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            memory: self.memory.clone(),
            modules: self.modules.clone(),
            vm_loader: self.vm_loader.clone(),
        }
    }
}

fn estimate_value_size(value: &CoreValue) -> usize {
    match value {
        CoreValue::Null | CoreValue::Boolean(_) => std::mem::size_of::<CoreValue>(),
        CoreValue::Integer(_) | CoreValue::Float(_) => std::mem::size_of::<CoreValue>(),
        CoreValue::String(s) => std::mem::size_of::<CoreValue>() + s.len(),
        CoreValue::Array(items) => {
            let element_bytes = items.len() * std::mem::size_of::<CoreValue>();
            std::mem::size_of::<CoreValue>() + element_bytes
        }
        CoreValue::Object(_) => std::mem::size_of::<CoreValue>() * 2,
    }
}
