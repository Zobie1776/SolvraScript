#[path = "../src/tests/util.rs"]
mod util;

use util::{run_svs_source, run_svs_source_expect_err};

#[test]
fn slice_operations_produce_expected_results() {
    let script = r#"
fn main() {
    let a = [1, 2, 3, 4, 5];
    let s1 = a[1:4];
    let s2 = a[:3];
    let s3 = a[2:];
    let s4 = a[::2];
    let s5 = a[1:4:2];
    let rev = a[::-1];
    let tail = a[:-1];
    let mid = a[-4:-1];
    let step_neg = a[4:1:-2];
    let nested = a[1:][::2];
    let t = "hello world";
    let u = t[0:5];
    let unicode = "héllo";
    let u1 = unicode[:2];
    let u2 = unicode[::-1];

    println(len(s1));
    println(len(s2));
    println(len(s3));
    println(len(s4));
    println(len(s5));
    println(len(tail));
    println(len(mid));
    println(len(step_neg));
    println(len(nested));

    println(s1[0]);
    println(s2[2]);
    println(s3[0]);
    println(s4[1]);
    println(s5[1]);
    println(u);
    println(rev[0]);
    println(mid[0]);
    println(step_neg[0]);
    println(step_neg[1]);
    println(nested[0]);
    println(nested[1]);
    println(u1);
    println(u2);
}

main();
"#;

    let output = run_svs_source(script);
    let expected = [
        "3", "3", "3", "3", "2", "4", "3", "2", "2", "2", "3", "3", "3", "4", "hello", "5", "2",
        "5", "3", "2", "4", "hé", "olléh",
    ];

    for line in expected {
        assert!(
            output.contains(line),
            "expected output to contain {line}, got {output}"
        );
    }
}

#[test]
fn slice_errors_surface_as_runtime_errors() {
    let invalid_target = r#"
fn main() {
    let a = 42[1:3];
    println(a);
}

main();
"#;
    let err = run_svs_source_expect_err(invalid_target);
    assert!(
        err.stderr.contains("slice target"),
        "expected slice target error, got {}",
        err.stderr
    );

    let invalid_index = r#"
fn main() {
    let t = "hello";
    let u = t[1:"x"];
    println(u);
}

main();
"#;
    let err = run_svs_source_expect_err(invalid_index);
    assert!(
        err.stderr.contains("slice indices") || err.stderr.contains("slice"),
        "expected slice index error, got {}",
        err.stderr
    );

    let zero_step = r#"
fn main() {
    let a = [1,2,3];
    let b = a[::0];
    println(b);
}

main();
"#;
    let err = run_svs_source_expect_err(zero_step);
    assert!(
        err.stderr.contains("step cannot be zero") || err.stderr.contains("slice step"),
        "expected zero-step error, got {}",
        err.stderr
    );

    let object_slice = r#"
fn main() {
    let m = { key: "value" };
    let b = m[0:1];
    println(b);
}

main();
"#;
    let err = run_svs_source_expect_err(object_slice);
    assert!(
        err.stderr.contains("slice target"),
        "expected slice target error, got {}",
        err.stderr
    );
}

#[test]
fn slices_work_in_functions_and_expressions() {
    let script = r#"
fn take(a) {
    return a[1:3];
}

fn slice_expr(a, i, j, n) {
    return a[(i + 1):(j - 1):n];
}

fn main() {
    let base = [0, 1, 2, 3, 4];
    let r = take(base);
    println(len(r));
    println(r[0]);

    let s = slice_expr(base, 0, 5, 2);
    println(len(s));
    println(s[0]);
    println(s[1]);
}

main();
"#;

    let output = run_svs_source(script);
    let expected = ["2", "1", "2", "1", "3"];
    for line in expected {
        assert!(
            output.contains(line),
            "expected output to contain {line}, got {output}"
        );
    }
}
