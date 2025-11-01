//=============================================
// solvra_script/vm/tests/async_timeout_tests.rs
//=============================================
// Purpose: Validate deterministic async timeout handling.
//=============================================

use std::sync::Arc;

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use crate::vm::compiler;
use crate::vm::runtime::{MemoryTracker, RuntimeOptions, run_bytecode};
use solvra_core::SolvraError;
use solvra_core::vm::bytecode::VmBytecode;

fn compile_timeout_example() -> Arc<VmBytecode> {
    let source = include_str!("../../examples/async_timeout.svs");
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer
        .tokenize()
        .expect("tokenize async timeout example");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse async timeout example");
    let bytecode = compiler::compile_program(&program).expect("compile async timeout example");
    let vm_program = VmBytecode::decode(&bytecode[..]).expect("decode bytecode");
    Arc::new(vm_program)
}

//=============================================
//            Phase 6.3A — Async Timeout Validation
//=============================================
#[test]
fn async_timeout_emits_runtime_exception() {
    let program = compile_timeout_example();
    let tracker = MemoryTracker::new();
    let options = RuntimeOptions::with_trace(false)
        .with_async_timeout(10)
        .with_memory_tracker(tracker.clone());
    let result = run_bytecode(program, options);

    let err = result.expect_err("expected runtime timeout error");
    match err {
        SolvraError::RuntimeException { message, stack } => {
            assert!(
                message.contains("RuntimeException::Timeout"),
                "expected timeout label in message: {message}"
            );
            assert!(
                stack
                    .iter()
                    .any(|frame| frame.function.contains("long_task")),
                "expected stack to include async function frame: {stack:?}"
            );
            assert!(
                stack.iter().any(|frame| frame.function.contains("main")),
                "expected stack to include main frame: {stack:?}"
            );
            assert!(
                !stack.is_empty(),
                "expected captured stack trace to be non-empty"
            );
        }
        other => panic!("expected runtime exception, received {other:?}"),
    }

    let stats = tracker.snapshot();
    assert_eq!(
        stats.last_stack_depth, 0,
        "stack depth should reset after timeout"
    );
    assert_eq!(
        stats.task_spawns, 1,
        "expected single async task spawn recorded"
    );
    assert_eq!(stats.timeouts, 1, "expected timeout counter to increment");
}
