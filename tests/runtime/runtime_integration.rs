#[path = "../../src/tests/util.rs"]
mod util;

use std::fs;
use tempfile::tempdir;
use util::{compile_to_svc, run_svc_file, run_svs_file};

#[test]
fn test_end_to_end_consistency() {
    let dir = tempdir().expect("tempdir");
    let src_path = dir.path().join("program.svs");
    let svc_path = dir.path().join("program.svc");

    let script = r#"
fn greet(name) {
    println(name);
}

fn main() {
    let label = if 2 < 5 { "Solvra" } else { "Fail" };
    greet(label);
}
"#;
    fs::write(&src_path, script).expect("write script");

    let src_output = run_svs_file(&src_path);
    compile_to_svc(&src_path, &svc_path);
    let bin_output = run_svc_file(&svc_path);
    assert_eq!(src_output, bin_output);
}
"#;
