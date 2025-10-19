//! JIT compilation entry points (feature gated).

use super::spec::NovaBytecode;
use anyhow::{anyhow, Result};

/// Compiles the provided bytecode using Cranelift when available.
pub fn compile(_bytecode: &NovaBytecode) -> Result<()> {
    Err(anyhow!("JIT compilation is not enabled in this build"))
}
