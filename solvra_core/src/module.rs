use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::bytecode::spec::SolvraBytecode;
use crate::{SolvraError, SolvraResult};

/// Loaded SolvraCore module.
#[derive(Debug, Clone)]
pub struct Module {
    name: String,
    bytecode: Arc<SolvraBytecode>,
}

impl Module {
    pub fn new(name: impl Into<String>, bytecode: SolvraBytecode) -> Self {
        Self {
            name: name.into(),
            bytecode: Arc::new(bytecode),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bytecode(&self) -> Arc<SolvraBytecode> {
        self.bytecode.clone()
    }
}

/// Registry keeping track of modules and their dependencies.
#[derive(Debug, Default)]
pub struct ModuleLoader {
    modules: HashMap<String, Arc<Module>>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn load_bytes(
        &mut self,
        name: impl Into<String>,
        bytes: &[u8],
    ) -> SolvraResult<Arc<Module>> {
        let name = name.into();
        let bytecode = SolvraBytecode::from_bytes(bytes)?;
        let module = Arc::new(Module::new(name.clone(), bytecode));
        self.modules.insert(name, module.clone());
        Ok(module)
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> SolvraResult<Arc<Module>> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|err| SolvraError::Internal(err.to_string()))?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("module")
            .to_string();
        self.load_bytes(name, &bytes)
    }

    pub fn resolve(&self, name: &str) -> Option<Arc<Module>> {
        self.modules.get(name).cloned()
    }

    pub fn modules(&self) -> impl Iterator<Item = &Arc<Module>> {
        self.modules.values()
    }
}
