use std::sync::Arc;

use parking_lot::RwLock;

use crate::backend::{ArchitectureBackend, BackendArtifact, TargetArch};
use crate::bytecode::vm::Vm;
use crate::module::ModuleLoader;
use crate::{RuntimeConfig, SolvraError, SolvraResult, Value};

#[derive(Debug, Default)]
pub struct X86Backend;

impl ArchitectureBackend for X86Backend {
    fn name(&self) -> &'static str {
        "x86_64"
    }

    fn target(&self) -> TargetArch {
        TargetArch::X86_64
    }

    fn compile(
        &self,
        bytecode: Arc<crate::bytecode::spec::SolvraBytecode>,
    ) -> SolvraResult<BackendArtifact> {
        Ok(BackendArtifact::Bytecode(bytecode))
    }

    fn execute(
        &self,
        artifact: BackendArtifact,
        config: RuntimeConfig,
        modules: Arc<RwLock<ModuleLoader>>,
    ) -> SolvraResult<Value> {
        match artifact {
            BackendArtifact::Bytecode(bytecode) => {
                let mut vm = Vm::new(config, bytecode, modules);
                vm.execute()
            }
            BackendArtifact::SativeBlob(_) => Err(SolvraError::Internal(
                "x86_64 backend does not support native blob execution".into(),
            )),
        }
    }
}
