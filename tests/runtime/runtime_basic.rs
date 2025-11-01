#[path = "../util.rs"]
mod util;

use util::run_svs_source;

#[test]
fn test_basic_arithmetic() {
    let src = r#"
fn main() {
    let a = 10;
    let b = 5;
    println(a + b);
}
"#;
    let output = run_svs_source(src);
    assert!(output.contains("15"), "output: {output}");
}

#[test]
fn test_if_else_and_assignment() {
    let src = r#"
fn main() {
    let value = if 3 > 1 { 7 } else { 0 };
    if value == 7 {
        println("ok");
    } else {
        println("fail");
    }
}
"#;
    let output = run_svs_source(src);
    assert!(output.contains("ok"), "output: {output}");
    assert!(!output.contains("fail"), "unexpected branch executed: {output}");
}

#[test]
fn test_function_call_and_return() {
    let src = r#"
fn add(a, b) {
    return a + b;
}

fn main() {
    let result = add(2, 8);
    println(result);
}
"#;
    let output = run_svs_source(src);
    assert!(output.contains("10"), "output: {output}");
}
"#;
