//=============================================
// solvra_script/vm/tests/async_tests.rs
//=============================================
// Purpose: Validate SolvraScript async scheduler determinism and diagnostics.
//=============================================

#[path = "../../tests/util.rs"]
mod util;

use util::{run_svs_source, run_svs_source_expect_err};

//=============================================
//            Phase 6.2 — Async Integrity & Scheduler Validation
//=============================================
#[test]
fn multiple_async_tasks_complete_deterministically() {
    let src = r#"
fn worker(label, value) {
    println(label);
    return value;
}

fn main() {
    let first = async worker("alpha", 1);
    let second = async worker("beta", 2);
    let a = await first;
    let b = await second;
    println("async done");
    println(a + b);
}
"#;

    let output = run_svs_source(src);
    assert!(
        output.contains("alpha"),
        "expected worker label output; got {output}"
    );
    assert!(
        output.contains("beta"),
        "expected second worker label output; got {output}"
    );
    assert!(
        output.contains("async done"),
        "expected completion marker; got {output}"
    );
    assert!(
        output.contains("3"),
        "expected deterministic sum from awaited tasks; got {output}"
    );
}

#[test]
fn nested_async_dependencies_resolve_in_order() {
    let src = r#"
fn child_task() {
    println("child start");
    return 40;
}

fn parent_task() {
    println("parent start");
    let child = async child_task();
    let result = await child;
    println("parent end");
    return result + 2;
}

fn main() {
    let parent = async parent_task();
    let final_value = await parent;
    println("final value");
    println(final_value);
}
"#;

    let output = run_svs_source(src);
    assert!(
        output.contains("parent start"),
        "expected parent to start; got {output}"
    );
    assert!(
        output.contains("child start"),
        "expected child execution; got {output}"
    );
    assert!(
        output.contains("parent end"),
        "expected parent completion; got {output}"
    );
    assert!(
        output.contains("42"),
        "expected propagated dependency result; got {output}"
    );
}

#[test]
fn async_handles_cleanup_after_completion() {
    let src = r#"
fn immediate(value) {
    println(value);
    return value;
}

fn spawn_and_wait(next) {
    let handle = async immediate(next);
    let value = await handle;
    println(value + 10);
}

fn main() {
    spawn_and_wait(0);
    spawn_and_wait(1);
    spawn_and_wait(2);
    println("loop done");
}
"#;

    let output = run_svs_source(src);
    assert!(
        output.contains("0"),
        "expected first task output; got {output}"
    );
    assert!(
        output.contains("11"),
        "expected second task derived value; got {output}"
    );
    assert!(
        output.contains("12"),
        "expected third task derived value; got {output}"
    );
    assert!(
        output.contains("loop done"),
        "expected cleanup sentinel; got {output}"
    );
}

#[test]
fn async_panic_reports_stack_trace() {
    let src = r#"
fn blowup() {
    let crash = 1 / 0;
    return crash;
}

fn main() {
    let task = async blowup();
    let value = await task;
    println(value);
}
"#;

    let result = run_svs_source_expect_err(src);
    assert!(
        result
            .stderr
            .contains("runtime error: integer division by zero"),
        "expected panic message in stderr; got {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("at blowup"),
        "expected stack trace to include function frame; got {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("at main"),
        "expected caller frame in stack trace; got {}",
        result.stderr
    );
}
