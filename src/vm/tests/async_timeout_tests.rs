//=============================================
// solvra_script/vm/tests/async_timeout_tests.rs
//=============================================
// Purpose: Validate deterministic async timeout handling.
//=============================================

use std::sync::{Arc, Mutex};

#[path = "../../tests/util.rs"]
mod util;

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use crate::vm::compiler as vm_compiler;
use crate::vm::runtime::{MemoryTracker, RuntimeOptions, run_bytecode};
use crate::vm::{TelemetryEvent, TelemetryEventKind};
use solvra_core::SolvraError;
use solvra_core::vm::bytecode::VmBytecode;

fn compile_timeout_example() -> Arc<VmBytecode> {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/async_timeout.svs"
    ));
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer
        .tokenize()
        .expect("tokenize async timeout example");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse async timeout example");
    let bytecode = vm_compiler::compile_program(&program).expect("compile async timeout example");
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
    let telemetry_events: Arc<Mutex<Vec<TelemetryEventKind>>> = Arc::new(Mutex::new(Vec::new()));
    let telemetry_clone = telemetry_events.clone();
    let hook = Arc::new(move |event: &TelemetryEvent| {
        telemetry_clone
            .lock()
            .expect("telemetry mutex poisoned")
            .push(event.kind.clone());
    });
    let options = RuntimeOptions::with_trace(false)
        .with_async_timeout(10)
        .with_memory_tracker(tracker.clone())
        .with_telemetry_hook(hook);
    let result = run_bytecode(program, options);

    let err = result.expect_err("expected runtime timeout error");
    match err {
        SolvraError::RuntimeException { message, stack } => {
            assert!(
                message.contains("RuntimeException::Timeout"),
                "expected timeout label in message: {message}"
            );
            assert!(
                message.contains("lineage: main -> long_task"),
                "expected lineage string in timeout message: {message}"
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
    assert!(stats.timeouts >= 1, "expected timeout counter to increment");
    assert!(
        stats.scheduler_ticks > 0,
        "expected scheduler tick metrics to be recorded"
    );
    assert!(
        stats
            .last_tick_tasks
            .iter()
            .any(|snapshot| snapshot.label.contains("long_task")),
        "expected last tick snapshot to include async task label"
    );
    assert!(
        !stats.timeout_stack_samples.is_empty(),
        "expected stack depth samples captured for timeouts"
    );

    let recorded = telemetry_events
        .lock()
        .expect("telemetry mutex poisoned")
        .clone();
    assert!(
        recorded
            .iter()
            .any(|kind| matches!(kind, TelemetryEventKind::TaskSpawn)),
        "expected telemetry to include TaskSpawn"
    );
    assert!(
        recorded
            .iter()
            .any(|kind| matches!(kind, TelemetryEventKind::TaskTimeout)),
        "expected telemetry to include TaskTimeout"
    );
}

#[test]
fn deadline_builtin_triggers_timeout_with_lineage() {
    let src = r#"
fn slow() {
    sleep(50);
    return 1;
}

fn main() {
    let job = async slow();
    let ok = core_with_deadline(job, 5);
    println(ok);
    let _ = await job;
}
"#;

    let result = util::run_svs_source_expect_err(src);
    assert!(
        result.stderr.contains("RuntimeException::Timeout"),
        "expected timeout error, got {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("lineage: main -> slow"),
        "expected lineage information, got {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("true"),
        "expected builtin to report success; stdout: {}",
        result.stdout
    );
}

#[test]
fn cancellation_builtin_aborts_task() {
    let src = r#"
fn hang() {
    sleep(100);
    return 0;
}

fn main() {
    let task = async hang();
    let cancelled = core_cancel_task(task);
    println(cancelled);
    let _ = await task;
}
"#;

    let result = util::run_svs_source_expect_err(src);
    assert!(
        result.stderr.contains("RuntimeException::Cancelled"),
        "expected cancellation error, got {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("lineage: main -> hang"),
        "expected lineage for cancellation, got {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("true"),
        "expected builtin to acknowledge cancellation; stdout: {}",
        result.stdout
    );
}
