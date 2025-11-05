//==================================================
// File: tests/stdlib.rs
//==================================================
// Author: ZobieLabs
// License: Apache License 2.0
// Goal: Validate Phase 1 stdlib modules through interpreter execution
// Objective: Ensure <io>, <string>, and <math> modules evaluate successfully
//==================================================

use std::fs;
use std::path::PathBuf;

use solvrascript::{
    interpreter::{Interpreter, Value},
    parser::Parser,
    tokenizer::Tokenizer,
};

//==================================================
// Section 1.0 - Helpers
//==================================================
// @TODO[StdlibPhase3]: Extend coverage once net/toml modules land.
// @ZNOTE[StdlibTests]: Uses interpreter to mirror developer workflows during migration.

fn eval_file(relative_path: &str) -> Value {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join(relative_path);
    let source = fs::read_to_string(&path).expect("read stdlib test script");
    eval_source(&source)
}

fn eval_source(source: &str) -> Value {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize source");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse program");
    let mut interpreter = Interpreter::new();
    interpreter
        .eval_program(&program)
        .expect("execute program")
        .unwrap_or(Value::Null)
}

//==================================================
// Section 2.0 - Tests
//==================================================

#[test]
fn stdlib_io_forwarders_execute() {
    let result = eval_file("tests/stdlib/use_std_io.svs");
    assert_eq!(result, Value::Int(42));
}

#[test]
fn stdlib_string_helpers_behave() {
    let result = eval_file("tests/stdlib/use_std_string.svs");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn stdlib_math_helpers_behave() {
    let result = eval_file("tests/stdlib/use_std_math.svs");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn stdlib_time_helpers_behave() {
    let result = eval_file("tests/stdlib/use_std_time.svs");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn stdlib_fs_helpers_behave() {
    let result = eval_file("tests/stdlib/use_std_fs.svs");
    let map = match result {
        Value::Object(entries) => entries,
        other => panic!("expected object from fs script, got {other:?}"),
    };
    assert_eq!(map.get("ok"), Some(&Value::Bool(true)));
    match map.get("length") {
        Some(Value::Int(len)) => assert!(*len > 0),
        other => panic!("expected length integer, got {other:?}"),
    }
    assert_eq!(map.get("home_is_string"), Some(&Value::Bool(true)));
}

#[test]
fn stdlib_json_helpers_behave() {
    let result = eval_file("tests/stdlib/use_std_json.svs");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn stdlib_sys_helpers_behave() {
    let result = eval_file("tests/stdlib/use_std_sys.svs");
    assert_eq!(result, Value::Bool(true));
}
