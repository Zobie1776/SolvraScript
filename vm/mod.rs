mod async_control;
mod builtins;
pub mod compiler;
mod core_builtins;
mod legacy_builtins;
mod metrics;
pub mod profiling;
pub mod runtime;

#[allow(unused_imports)]
pub use solvra_core::vm::{bytecode, instruction, stack_vm};

#[allow(unused_imports)]
pub use metrics::{
    TelemetryCollector, TelemetryEvent, TelemetryEventKind, TelemetryHook, TelemetryRecord,
};

#[cfg(test)]
mod tests;
