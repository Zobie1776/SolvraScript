#![cfg_attr(all(not(feature = "ffi")), forbid(unsafe_code))]

//! NovaCore v0.1 – a lightweight runtime and bytecode execution engine.
//!
//! The crate exposes three major building blocks:
//!
//! * [`bytecode`] – definition of the NovaBytecode format and an assembler capable of
//!   lowering a small high-level AST into a compact instruction stream.
//! * [`NovaRuntime`] – an embeddable runtime with a cost-metered interpreter capable of
//!   executing the bytecode produced by the assembler.
//! * [`sys`] – cross platform utilities for IO and networking that NovaRuntime and higher
//!   level applications can depend on.
//!
//! The implementation purposely focuses on being easy to audit and extend. The public API is
//! designed so other NovaOS crates (CLI, IDE, or third party tools) can share the runtime
//! without leaking internal details.  Unsafe code is avoided by default and only available
//! behind the optional `ffi` feature flag where raw pointers are unavoidable.

pub mod bytecode;
pub mod concurrency;
pub mod ffi;
pub mod memory;
pub mod sys;

use std::sync::Arc;

use anyhow::{anyhow, Result};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use parking_lot::RwLock;
use rand::thread_rng;
use thiserror::Error;
use tracing::instrument;

use crate::bytecode::{spec::NovaBytecode, vm::Vm};

/// Result type used across NovaCore.
pub type NovaResult<T> = std::result::Result<T, NovaError>;

/// Values that NovaRuntime can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl Value {
    pub(crate) fn from_constant(constant: &bytecode::spec::Constant) -> Self {
        use bytecode::spec::Constant;
        match constant {
            Constant::Null => Value::Null,
            Constant::Boolean(b) => Value::Boolean(*b),
            Constant::Integer(i) => Value::Integer(*i),
            Constant::Float(f) => Value::Float(*f),
            Constant::String(s) => Value::String(s.clone()),
        }
    }
}

/// Configuration for the runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum amount of interpreter steps before execution aborts.
    pub cost_limit: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self { cost_limit: 10_000 }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NovaError {
    #[error("execution blocked by fail-safe gate")]
    FailSafeLocked,
    #[error("fail-safe passphrase mismatch")]
    FailSafeAuthFailed,
    #[error("bytecode error: {0}")]
    Bytecode(String),
    #[error("runtime stack underflow")]
    StackUnderflow,
    #[error("runtime exceeded cost limit")]
    CostLimitExceeded,
    #[error("invalid cost limit: {0}")]
    InvalidCostLimit(String),
    #[error("unexpected runtime state: {0}")]
    Internal(String),
}

impl From<bytecode::spec::NovaBytecodeError> for NovaError {
    fn from(err: bytecode::spec::NovaBytecodeError) -> Self {
        NovaError::Bytecode(err.to_string())
    }
}

/// Fail-safe state shared by the runtime.
#[derive(Default, Debug)]
struct FailSafeState {
    inner: RwLock<Option<FailSafeConfig>>,
}

#[derive(Debug, Clone)]
struct FailSafeConfig {
    hash: String,
    unlocked: bool,
}

impl FailSafeState {
    fn enable(&self, passphrase: &str) -> Result<()> {
        let salt = SaltString::generate(&mut thread_rng());
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(passphrase.as_bytes(), &salt)
            .map_err(|err| anyhow!(err.to_string()))?
            .to_string();
        *self.inner.write() = Some(FailSafeConfig {
            hash,
            unlocked: false,
        });
        Ok(())
    }

    fn clear(&self) {
        *self.inner.write() = None;
    }

    fn authenticate(&self, passphrase: &str) -> Result<(), NovaError> {
        let mut guard = self.inner.write();
        let Some(config) = guard.as_mut() else {
            return Err(NovaError::FailSafeLocked);
        };
        let parsed =
            PasswordHash::new(&config.hash).map_err(|err| NovaError::Internal(err.to_string()))?;
        if Argon2::default()
            .verify_password(passphrase.as_bytes(), &parsed)
            .is_ok()
        {
            config.unlocked = true;
            Ok(())
        } else {
            Err(NovaError::FailSafeAuthFailed)
        }
    }

    fn ensure_unlocked(&self) -> Result<(), NovaError> {
        let guard = self.inner.read();
        match guard.as_ref() {
            Some(cfg) if cfg.unlocked => Ok(()),
            Some(_) => Err(NovaError::FailSafeLocked),
            None => Ok(()),
        }
    }
}

/// NovaRuntime orchestrates the interpreter and provides fail-safe gating.
#[derive(Debug, Clone)]
pub struct NovaRuntime {
    config: RuntimeConfig,
    failsafe: Arc<FailSafeState>,
}

impl Default for NovaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl NovaRuntime {
    /// Creates a new runtime with default configuration.
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            failsafe: Arc::new(FailSafeState::default()),
        }
    }

    /// Overrides the cost limit used by the interpreter.
    pub fn with_cost_limit(mut self, cost_limit: u64) -> NovaResult<Self> {
        if cost_limit == 0 {
            return Err(NovaError::InvalidCostLimit(
                "cost limit must be greater than zero".into(),
            ));
        }
        self.config.cost_limit = cost_limit;
        Ok(self)
    }

    /// Enables the fail-safe and stores the hashed passphrase.
    pub fn enable_failsafe(&self, passphrase: &str) -> NovaResult<()> {
        self.failsafe
            .enable(passphrase)
            .map_err(|err| NovaError::Internal(err.to_string()))
    }

    /// Clears the stored passphrase and unlocks execution.
    pub fn disable_failsafe(&self) {
        self.failsafe.clear();
    }

    /// Authenticates against the fail-safe gate.
    pub fn authenticate(&self, passphrase: &str) -> NovaResult<()> {
        self.failsafe.authenticate(passphrase)
    }

    /// Executes the provided bytecode.
    #[instrument(skip_all)]
    pub fn execute(&self, bytes: &[u8]) -> NovaResult<Value> {
        self.failsafe.ensure_unlocked()?;
        let bytecode = NovaBytecode::from_bytes(bytes)?;
        let mut vm = Vm::new(self.config.cost_limit, bytecode.constants().to_vec());
        let value = vm.execute(bytecode.instructions())?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::assemble;

    #[test]
    fn runtime_executes_simple_program() {
        let ast = bytecode::ast::Ast::from_expr(bytecode::ast::Expr::binary(
            bytecode::ast::BinaryOp::Add,
            bytecode::ast::Expr::number(1.0),
            bytecode::ast::Expr::number(2.0),
        ));
        let bytecode = assemble(&ast).expect("assemble");
        let runtime = NovaRuntime::new();
        let value = runtime
            .execute(&bytecode.into_bytes())
            .expect("execution succeeds");
        assert_eq!(value, Value::Float(3.0));
    }

    #[test]
    fn failsafe_blocks_execution() {
        let ast = bytecode::ast::Ast::from_expr(bytecode::ast::Expr::number(0.0));
        let bytecode = assemble(&ast).expect("assemble");
        let runtime = NovaRuntime::new();
        runtime.enable_failsafe("nova").unwrap();
        let bytes = bytecode.into_bytes();
        let err = runtime.execute(&bytes).unwrap_err();
        assert_eq!(err, NovaError::FailSafeLocked);
        runtime.authenticate("nova").unwrap();
        runtime.execute(&bytes).unwrap();
    }

    #[test]
    fn cost_limit_validation() {
        assert!(NovaRuntime::new().with_cost_limit(0).is_err());
    }
}
