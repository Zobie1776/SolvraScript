#[path = "../../src/tests/util.rs"]
mod util;

use std::fs;
use tempfile::tempdir;
use util::{compile_to_svc, run_svc_file, run_svs_source};

#[test]
fn test_async_tasks() {
    let src = r#"
fn compute() {
    println("computing");
    return 42;
}

fn main() {
    let task = async compute();
    let value = await task;
    println("async finished");
    println(value);
}
"#;
    let output = run_svs_source(src);
    assert!(output.contains("computing"), "output: {output}");
    assert!(output.contains("async finished"), "output: {output}");
    assert!(output.contains("42"), "output: {output}");
}

#[test]
fn test_async_bytecode_execution() {
    let dir = tempdir().expect("tempdir");
    let src_path = dir.path().join("async_task.svs");
    let svc_path = dir.path().join("async_task.svc");

    let script = r#"
fn compute() {
    println("computing");
    return 42;
}

fn main() {
    let task = async compute();
    let value = await task;
    println("async finished");
    println(value);
}
"#;
    fs::write(&src_path, script).expect("write script");
    compile_to_svc(&src_path, &svc_path);
    let output = run_svc_file(&svc_path);
    assert!(output.contains("computing"), "output: {output}");
    assert!(output.contains("async finished"), "output: {output}");
    assert!(output.contains("42"), "output: {output}");
}
"#;
