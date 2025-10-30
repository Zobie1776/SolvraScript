use std::sync::Arc;

use parking_lot::RwLock;
use tracing::debug;

use crate::backend::{ArchitectureBackend, BackendArtifact, TargetArch};
use crate::bytecode::vm::Vm;
use crate::integration::RuntimeHooks;
use crate::module::ModuleLoader;
use crate::sys::drivers::DriverRegistry;
use crate::{SolvraError, SolvraResult, RuntimeConfig, Value};

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
        bytecode: Arc<crate::bytecode::spec::SolvraBytecode>,
    ) -> SolvraResult<BackendArtifact> {
        debug!("armv7 backend compiling bytecode artifact");
        Ok(BackendArtifact::Bytecode(bytecode))
    }

    fn execute(
        &self,
        artifact: BackendArtifact,
        config: RuntimeConfig,
        modules: Arc<RwLock<ModuleLoader>>,
        drivers: Arc<DriverRegistry>,
        hooks: Arc<RuntimeHooks>,
    ) -> SolvraResult<Value> {
        match artifact {
            BackendArtifact::Bytecode(bytecode) => {
                debug!("armv7 backend executing via interpreter");
                let mut vm = Vm::new(config, bytecode, modules, drivers, hooks);
                vm.execute()
            }
            BackendArtifact::SativeBlob(_) => Err(SolvraError::Internal(
                "armv7 backend does not support native blob execution".into(),
            )),
        }
    }
}
