#![cfg_attr(all(not(feature = "ffi")), forbid(unsafe_code))]

//! NovaCore v0.1 – a lightweight runtime and bytecode execution engine.
//!
//! The crate exposes three major building blocks:
//!
//! * [`backend`] – architecture-specific code generation and execution backends
//!   selected at compile time via Cargo features.
//! * [`bytecode`] – definition of the NovaBytecode format and an assembler capable of
//!   lowering a small high-level AST into a compact instruction stream.
//! * [`runtime`] – an embeddable runtime with a cost-metered interpreter capable of
//!   executing the bytecode produced by the assembler, including a developer friendly REPL.
//! * [`sys`] – cross platform utilities for IO and networking that NovaRuntime and higher
//!   level applications can depend on.
//!
//! The implementation purposely focuses on being easy to audit and extend. The public API is
//! designed so other NovaOS crates (CLI, IDE, or third party tools) can share the runtime
//! without leaking internal details.  Unsafe code is avoided by default and only available
//! behind the optional `ffi` feature flag where raw pointers are unavoidable.

#[path = "../backend/mod.rs"]
pub mod backend;
pub mod bytecode;
pub mod concurrency;
pub mod ffi;
pub mod integration;
pub mod memory;
pub mod module;
pub mod novac;
pub mod sys;

#[path = "../runtime/mod.rs"]
pub mod runtime;

pub use integration::{DebuggerEvent, RuntimeHooks, RuntimeLog, TelemetryEvent};
pub use runtime::repl::RuntimeRepl;
pub use runtime::{NovaError, NovaRuntime, RuntimeConfig, StackFrame, Value};
pub use sys::drivers::{DriverDescriptor, DriverRegistry, Interrupt};

/// Result type used across NovaCore.
pub type NovaResult<T> = std::result::Result<T, NovaError>;
