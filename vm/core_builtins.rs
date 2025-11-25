//==============================================
// File: solvra_script/vm/core_builtins.rs
// Author: Codex/AGENT
// License: Duality Public License (DPL v1.0)
// Goal: Shared definitions for SolvraCore bridge builtins
// Objective: Centralize CoreCall names for compiler + runtime parity
//==============================================

pub const CORE_CALL_BUILTINS: &[&str] = &["core_vm_execute", "core_vm_spawn", "core_task_info"];

pub fn is_core_builtin_name(name: &str) -> bool {
    name.starts_with("core::") || is_core_stub_call(name)
}

pub fn is_core_stub_call(name: &str) -> bool {
    CORE_CALL_BUILTINS
        .iter()
        .any(|candidate| *candidate == name)
}

pub fn core_stub_message(name: &str) -> String {
    format!("core builtin '{name}' is not implemented yet")
}

//==============================================
// End of file
//==============================================
