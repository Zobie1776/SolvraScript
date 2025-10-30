use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use crate::memory::{MemoryContract, MemoryHandle, MemoryStats};
use crate::{SolvraError, SolvraResult};

use super::bytecode::VmBytecode;

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub handle: MemoryHandle,
    pub bytecode: Arc<VmBytecode>,
}

#[derive(Debug, Clone)]
pub struct ModuleLoaderVm {
    memory: Arc<MemoryContract>,
}

impl ModuleLoaderVm {
    pub fn new(memory: Arc<MemoryContract>) -> Self {
        Self { memory }
    }

    pub fn load_from_path(&self, path: impl AsRef<Path>) -> SolvraResult<LoadedModule> {
        let bytes = fs::read(&path).map_err(|err| SolvraError::Internal(err.to_string()))?;
        self.load_from_bytes(&bytes)
    }

    pub fn load_from_bytes(&self, bytes: &[u8]) -> SolvraResult<LoadedModule> {
        let bytecode = match VmBytecode::decode(Cursor::new(bytes)) {
            Ok(vm) => vm,
            Err(_) => {
                let solvrac = crate::solvrac::Bytecode::decode(bytes)
                    .map_err(|err| SolvraError::Bytecode(err.to_string()))?;
                super::compiler::from_solvrac(&solvrac)
            }
        };
        let arc_bc = Arc::new(bytecode);
        let serialized = arc_bc
            .serialize()
            .map_err(|err| SolvraError::Bytecode(format!("vm encode failed: {err}")))?;
        let bytes_arc = Arc::new(serialized.clone());
        let handle = self
            .memory
            .allocate_arc(bytes_arc, serialized.len())
            .map_err(|err| SolvraError::Internal(err.to_string()))?;
        Ok(LoadedModule {
            handle,
            bytecode: arc_bc,
        })
    }

    pub fn memory_stats(&self) -> MemoryStats {
        self.memory.stats()
    }
}
