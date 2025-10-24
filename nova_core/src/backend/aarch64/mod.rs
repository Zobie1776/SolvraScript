use std::sync::Arc;

use parking_lot::RwLock;
use tracing::debug;

use crate::backend::{ArchitectureBackend, BackendArtifact, TargetArch};
use crate::bytecode::vm::Vm;
use crate::module::ModuleLoader;
use crate::{NovaError, NovaResult, RuntimeConfig, Value};

#[derive(Debug, Default)]
pub struct Aarch64Backend;

impl ArchitectureBackend for Aarch64Backend {
    fn name(&self) -> &'static str {
        "aarch64"
    }

    fn target(&self) -> TargetArch {
        TargetArch::AArch64
    }

    fn compile(
        &self,
        bytecode: Arc<crate::bytecode::spec::NovaBytecode>,
    ) -> NovaResult<BackendArtifact> {
        debug!("aarch64 backend compiling bytecode artifact");
        Ok(BackendArtifact::Bytecode(bytecode))
    }

    fn execute(
        &self,
        artifact: BackendArtifact,
        config: RuntimeConfig,
        modules: Arc<RwLock<ModuleLoader>>,
    ) -> NovaResult<Value> {
        match artifact {
            BackendArtifact::Bytecode(bytecode) => {
                debug!("aarch64 backend executing via interpreter");
                let mut vm = Vm::new(config, bytecode, modules);
                vm.execute()
            }
            BackendArtifact::NativeBlob(_) => Err(NovaError::Internal(
                "aarch64 backend does not support native blob execution".into(),
            )),
        }
    }
}
