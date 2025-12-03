#[path = "../src/tests/util.rs"]
mod util;

use util::run_svs_source;

#[test]
fn main_function_invocation_executes() {
    let src = r#"
fn main() {
    println("hello");
}

main();
"#;

    let output = run_svs_source(src);
    assert!(
        output.contains("hello"),
        "expected hello in output: {output}"
    );
}

#[test]
fn nested_user_function_calls_return_values() {
    let src = r#"
fn add(a, b) {
    return a + b;
}

fn compute_triple(base) {
    let doubled = add(base, base);
    return add(doubled, base);
}

fn main() {
    let total = compute_triple(5);
    println(total);
}

main();
"#;

    let output = run_svs_source(src);
    assert!(
        output.contains("15"),
        "expected 15 from nested user function calls, got {output}"
    );
}
