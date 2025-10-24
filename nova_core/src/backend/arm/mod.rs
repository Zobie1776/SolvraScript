use std::sync::Arc;

use parking_lot::RwLock;
use tracing::debug;

use crate::backend::{ArchitectureBackend, BackendArtifact, TargetArch};
use crate::bytecode::vm::Vm;
use crate::module::ModuleLoader;
use crate::{NovaError, NovaResult, RuntimeConfig, Value};

#[derive(Debug, Default)]
pub struct Armv7Backend;

impl ArchitectureBackend for Armv7Backend {
    fn name(&self) -> &'static str {
        "armv7"
    }

    fn target(&self) -> TargetArch {
        TargetArch::Armv7
    }

    fn compile(
        &self,
        bytecode: Arc<crate::bytecode::spec::NovaBytecode>,
    ) -> NovaResult<BackendArtifact> {
        debug!("armv7 backend compiling bytecode artifact");
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
                debug!("armv7 backend executing via interpreter");
                let mut vm = Vm::new(config, bytecode, modules);
                vm.execute()
            }
            BackendArtifact::NativeBlob(_) => Err(NovaError::Internal(
                "armv7 backend does not support native blob execution".into(),
            )),
        }
    }
}
