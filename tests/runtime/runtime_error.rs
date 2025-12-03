#[path = "../../src/tests/util.rs"]
mod util;

use util::{run_svs_source_expect_err, CommandOutput};

fn assert_failed(output: &CommandOutput, needle: &str) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded with stdout: {}",
        output.stdout
    );
    let combined = format!("{}{}", output.stdout, output.stderr);
    assert!(
        combined.contains(needle),
        "expected error containing '{needle}', got:\n{combined}"
    );
}

#[test]
fn test_missing_builtin_error() {
    let src = r#"
fn main() {
    magic_print("hello");
}
"#;
    let output = run_svs_source_expect_err(src);
    assert_failed(&output, "unknown builtin function 'magic_print'");
}

#[test]
fn test_await_invalid_handle() {
    let src = r#"
fn main() {
    let bogus = 12;
    let value = await bogus;
    println(value);
}
"#;
    let output = run_svs_source_expect_err(src);
    assert_failed(&output, "await expects task identifier");
}
"#;
