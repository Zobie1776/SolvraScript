//! Diagnostic tests for debugging stdx/datetime module behavior.
//! These tests *do not* assume the module is correct — they probe every step.

use std::time::{Duration, Instant};
use solvrascript::{
    modules::{ImportSource, ModuleLoader},
    tokenizer::Tokenizer,
    parser::Parser,
    interpreter::Interpreter,
};

fn with_timeout<F: FnOnce() -> R, R>(dur: Duration, f: F) -> Option<R> {
    let start = Instant::now();
    std::thread::spawn(move || f())
        .join()
        .ok()
        .filter(|_| start.elapsed() < dur)
}

/// Create a loader with all standard search paths.
fn make_loader() -> ModuleLoader {
    let mut loader = ModuleLoader::new();
    loader.add_script_path(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/stdx"),
    );
    loader
}

//
// ─────────────────────────────────────────────────────────────
//   1. Basic resolution tests
// ─────────────────────────────────────────────────────────────
//

#[test]
fn debug_datetime_resolves() {
    let mut loader = make_loader();

    let res = loader.prepare_module(
        &ImportSource::BareModule("stdx.datetime".into()),
        None,
    );

    assert!(
        res.is_ok(),
        "Failed to resolve stdx.datetime: {:?}",
        res.err()
    );
}

#[test]
fn debug_datetime_no_cycles() {
    let mut loader = make_loader();

    let result = with_timeout(Duration::from_secs(10), || {
        loader.prepare_module(&ImportSource::BareModule("stdx.datetime".into()), None)
    });

    assert!(
        result.is_some(),
        "stdx.datetime import hung more than 10 seconds — likely cyclic import"
    );

    let res = result.unwrap();
    assert!(
        res.is_ok(),
        "stdx.datetime failed due to cycle or invalid import: {:?}",
        res.err()
    );
}

//
// ─────────────────────────────────────────────────────────────
//   2. Test internal file resolution (datetime.svs, mod.svs)
// ─────────────────────────────────────────────────────────────
//

#[test]
fn debug_datetime_files_exist() {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/stdx/datetime");

    let datetime = base.join("datetime.svs");
    let module = base.join("mod.svs");

    assert!(datetime.exists(), "datetime.svs missing at: {:?}", datetime);
    assert!(module.exists(), "mod.svs missing at: {:?}", module);
}

#[test]
fn debug_datetime_loads_internal_files() {
    let path = format!("{}/src/stdx/datetime/datetime.svs", env!("CARGO_MANIFEST_DIR"));
    let content = std::fs::read_to_string(&path);

    assert!(
        content.is_ok(),
        "Failed to read datetime.svs: {:?}",
        content.err()
    );
}

//
// ─────────────────────────────────────────────────────────────
//   3. Tokenization & Parsing Tests
// ─────────────────────────────────────────────────────────────
//

#[test]
fn debug_datetime_tokenizes() {
    let path = format!("{}/src/stdx/datetime/datetime.svs", env!("CARGO_MANIFEST_DIR"));
    let code = std::fs::read_to_string(&path).expect("read datetime.svs");

    let mut tokenizer = Tokenizer::new(&code);
    let tokens = tokenizer.tokenize();

    assert!(
        tokens.is_ok(),
        "Tokenizer failed: {:?}",
        tokens.err()
    );
}

#[test]
fn debug_datetime_parses() {
    let path = format!("{}/src/stdx/datetime/datetime.svs", env!("CARGO_MANIFEST_DIR"));
    let code = std::fs::read_to_string(&path).expect("read datetime.svs");

    let mut tokenizer = Tokenizer::new(&code);
    let tokens = tokenizer.tokenize().expect("tokenize datetime.svs");

    let mut parser = Parser::new(tokens);
    let parsed = parser.parse();

    assert!(parsed.is_ok(), "Parser failed: {:?}", parsed.err());
}

//
// ─────────────────────────────────────────────────────────────
//   4. Execution Tests
// ─────────────────────────────────────────────────────────────
//

#[test]
fn debug_datetime_exec_smoke_test() {
    let path = format!("{}/src/stdx/datetime/mod.svs", env!("CARGO_MANIFEST_DIR"));
    let code = std::fs::read_to_string(&path).expect("read mod.svs");

    let mut tokenizer = Tokenizer::new(&code);
    let tokens = tokenizer.tokenize().expect("tokenize mod.svs");

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse mod.svs");

    let mut interp = Interpreter::new();

    let res = with_timeout(Duration::from_secs(10), || {
        interp.eval_program(&program)
    });

    assert!(
        res.is_some(),
        "Execution hung more than 10 seconds — likely cyclic runtime behavior"
    );
}

//
// ─────────────────────────────────────────────────────────────
//   5. Recursively try re-importing datetime in isolation
// ─────────────────────────────────────────────────────────────
//

#[test]
fn debug_datetime_import_stress() {
    for i in 0..30 {
        let mut loader = make_loader();
        let res = with_timeout(Duration::from_secs(2), || {
            loader.prepare_module(&ImportSource::BareModule("stdx.datetime".into()), None)
        });

        assert!(
            res.is_some(),
            "Iteration {i}: import hung — cycle still present"
        );

        let result = res.unwrap();
        assert!(
            result.is_ok(),
            "Iteration {i}: import error {:?}",
            result.err()
        );
    }
}

