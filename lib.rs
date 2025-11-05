//=====================================================
// File: lib.rs
//=====================================================
// Author: ZobieLabs
// License: Duality Public License (DPL v1.0)
// Goal: SolvraScript library main interface
// Objective: Export core modules including AST, interpreter, parser, tokenizer,
//            VM runtime, and standard library registry for SolvraScript execution
//=====================================================

// Added by Claude for Zobie.format compliance
pub mod ast;
pub mod core_bridge;
pub mod interpreter;
pub mod modules;
pub mod parser;
pub mod platform;
pub mod stdlib_registry;
pub mod tokenizer;
pub mod vm;

pub use stdlib_registry::{StdlibContext, StdlibRegistry};
pub use vm::{
    TelemetryCollector, TelemetryEvent, TelemetryEventKind, TelemetryHook, TelemetryRecord,
};

//=====================================================
// End of file
//=====================================================
// Added by Claude for Zobie.format compliance
