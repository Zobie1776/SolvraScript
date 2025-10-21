use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::bytecode::spec::NovaBytecode;
use crate::module::ModuleLoader;
use crate::{NovaResult, RuntimeConfig, Value};

#[cfg(all(feature = "backend-x86_64", feature = "backend-armv7"))]
compile_error!("backend-x86_64 and backend-armv7 features are mutually exclusive");
#[cfg(all(feature = "backend-x86_64", feature = "backend-aarch64"))]
compile_error!("backend-x86_64 and backend-aarch64 features are mutually exclusive");
#[cfg(all(feature = "backend-armv7", feature = "backend-aarch64"))]
compile_error!("backend-armv7 and backend-aarch64 features are mutually exclusive");

/// Supported backend targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    Armv7,
    AArch64,
}

impl TargetArch {
    /// Returns the canonical architecture name.
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64",
            TargetArch::Armv7 => "armv7",
            TargetArch::AArch64 => "aarch64",
        }
    }
}

/// Artifact produced by a backend.
#[derive(Debug, Clone)]
pub enum BackendArtifact {
    Bytecode(Arc<NovaBytecode>),
    NativeBlob(Vec<u8>),
}

impl BackendArtifact {
    /// Returns the inner bytecode if the artifact is interpreter-based.
    pub fn bytecode(&self) -> Option<Arc<NovaBytecode>> {
        match self {
            BackendArtifact::Bytecode(code) => Some(code.clone()),
            BackendArtifact::NativeBlob(_) => None,
        }
    }
}

/// Common interface implemented by all architecture backends.
pub trait ArchitectureBackend: Send + Sync {
    /// Human readable backend identifier.
    fn name(&self) -> &'static str;
    /// Returns the target architecture metadata.
    fn target(&self) -> TargetArch;
    /// Lowers Nova bytecode into a backend specific artifact.
    fn compile(&self, bytecode: Arc<NovaBytecode>) -> NovaResult<BackendArtifact>;
    /// Executes a backend artifact with the provided runtime configuration.
    fn execute(
        &self,
        artifact: BackendArtifact,
        config: RuntimeConfig,
        modules: Arc<RwLock<ModuleLoader>>,
    ) -> NovaResult<Value>;
    /// Optional optimisation hook run prior to emission.
    fn optimise(&self, _bytecode: &mut NovaBytecode) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "backend-aarch64")]
pub mod aarch64;
#[cfg(feature = "backend-armv7")]
pub mod arm;
#[cfg(feature = "backend-x86_64")]
pub mod x86_64;

pub fn active_backend() -> &'static dyn ArchitectureBackend {
    #[cfg(feature = "backend-x86_64")]
    {
        static BACKEND: Lazy<x86_64::X86Backend> = Lazy::new(x86_64::X86Backend::default);
        &*BACKEND
    }

    #[cfg(all(not(feature = "backend-x86_64"), feature = "backend-armv7"))]
    {
        static BACKEND: Lazy<arm::Armv7Backend> = Lazy::new(arm::Armv7Backend::default);
        &*BACKEND
    }

    #[cfg(all(
        not(feature = "backend-x86_64"),
        not(feature = "backend-armv7"),
        feature = "backend-aarch64"
    ))]
    {
        static BACKEND: Lazy<aarch64::Aarch64Backend> = Lazy::new(aarch64::Aarch64Backend::default);
        &*BACKEND
    }

    #[cfg(not(any(
        feature = "backend-x86_64",
        feature = "backend-armv7",
        feature = "backend-aarch64"
    )))]
    {
        compile_error!("At least one backend feature must be enabled");
    }
}

pub fn active_target() -> TargetArch {
    active_backend().target()
}
