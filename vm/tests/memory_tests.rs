//=============================================
// solvra_script/vm/tests/memory_tests.rs
//=============================================
// Purpose: Validate VM memory behavior, allocator reuse, and scope reclamation.
//=============================================

use std::sync::Arc;

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use crate::vm::compiler;
use crate::vm::runtime::{MemoryTracker, RuntimeOptions, run_bytecode};
use solvra_core::vm::bytecode::{VmBytecode, VmConstant};

fn compile_program(source: &str) -> Arc<VmBytecode> {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize script");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse program");
    let bytecode = compiler::compile_program(&program).expect("compile program");
    let vm_program = VmBytecode::decode(&bytecode[..]).expect("decode vm bytecode");
    Arc::new(vm_program)
}

//=============================================
//            Phase 6.3 — Memory & Heap Tests
//=============================================
#[test]
fn constant_string_loads_are_deduplicated() {
    let program = compile_program(
        r#"
fn main() {
    println("reuse");
    println("reuse");
}
"#,
    );

    let tracker = MemoryTracker::new();
    let options = RuntimeOptions::default().with_memory_tracker(tracker.clone());
    run_bytecode(Arc::clone(&program), options).expect("run program");

    let stats = tracker.snapshot();
    let string_index = program
        .constants
        .iter()
        .enumerate()
        .find_map(|(idx, constant)| match constant {
            VmConstant::String(text) if text == "reuse" => Some(idx),
            _ => None,
        })
        .expect("string constant present");
    assert_eq!(stats.constant_hits.get(&string_index), Some(&2));
    let reuse_constant_count = program
        .constants
        .iter()
        .filter(|constant| matches!(constant, VmConstant::String(text) if text == "reuse"))
        .count();
    assert_eq!(
        reuse_constant_count, 1,
        "expected reuse string to appear once"
    );
}

#[test]
fn program_arc_counts_restore_after_execution() {
    let program = compile_program(
        r#"
fn helper(value) {
    return value + 1;
}

fn main() {
    let next = helper(10);
    println(next);
}
"#,
    );

    let baseline = Arc::strong_count(&program);
    run_bytecode(Arc::clone(&program), RuntimeOptions::default()).expect("run program");
    assert_eq!(
        Arc::strong_count(&program),
        baseline,
        "Arc strong count should reset"
    );
}

#[test]
fn stack_depth_returns_to_zero_on_scope_exit() {
    let program = compile_program(
        r#"
fn nested(level) {
    if level == 0 {
        return 1;
    }
    let next = level - 1;
    let value = nested(next);
    return value + 1;
}

fn main() {
    let result = nested(3);
    println(result);
}
"#,
    );

    let tracker = MemoryTracker::new();
    let options = RuntimeOptions::default().with_memory_tracker(tracker.clone());
    run_bytecode(Arc::clone(&program), options).expect("run program");

    let stats = tracker.snapshot();
    assert_eq!(
        stats.last_stack_depth, 0,
        "stack should be empty after execution: {stats:?}"
    );
    assert!(
        stats.max_stack_depth >= 2,
        "expected recursive calls to grow stack: {stats:?}"
    );
}
