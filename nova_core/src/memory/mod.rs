//! Memory management utilities used by NovaRuntime.

pub mod arena;
#[cfg(feature = "gc")]
pub mod gc;
