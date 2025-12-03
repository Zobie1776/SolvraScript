//==============================================
// File: runtime.rs
// Author: Codex
// License: Duality Public License (DPL v1.0)
// Goal: Shared runtime helpers for SolvraScript tests
// Objective: Execute .svs fixtures with consistent module resolution and assertions
//==============================================

//==============================================
// Import & Modules
//==============================================

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::interpreter::{Interpreter, Value};
use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

//==============================================
// Section 1.0 - SVS Test Harness
//==============================================
// Run an .svs script relative to the crate root and assert it returns `true`.
pub fn run_svs_test(relative_path: &str) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir.join(relative_path);
    let source = fs::read_to_string(&script_path)
        .unwrap_or_else(|err| panic!("read {} failed: {}", script_path.display(), err));

    let mut tokenizer = Tokenizer::new(&source);
    let tokens = tokenizer
        .tokenize()
        .unwrap_or_else(|err| panic!("tokenize {}: {}", script_path.display(), err));
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|err| panic!("parse {}: {:?}", script_path.display(), err));

    let mut interpreter = Interpreter::with_std();
    interpreter.reset_execution_timer();
    let search_paths = interpreter.script_search_paths();
    println!(
        "[debug] Resolving SVS fixture {} with search roots {:?}",
        script_path.display(),
        search_paths
    );
    let start = Instant::now();
    let result = interpreter
        .eval_program_with_origin(&program, Some(&script_path))
        .unwrap_or_else(|err| panic!("execute {}: {:?}", script_path.display(), err))
        .unwrap_or(Value::Null);
    let elapsed = start.elapsed();
    if elapsed > Duration::from_secs(5) {
        panic!(
            "SVS test {} timed out after {:?}",
            script_path.display(),
            elapsed
        );
    }

    match result {
        Value::Bool(true) => {}
        other => panic!(
            "SVS test {} failed: expected boolean true, got {other:?}",
            script_path.display()
        ),
    }
}

//==============================================
// End of file
//==============================================
