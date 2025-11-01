//=============================================
// solvra_script/vm/tests/parity_tests.rs
//=============================================
// Purpose: Ensure SolvraScript source interpreter and bytecode VM
//          produce identical observable behavior for paired examples.
//=============================================

#[path = "../../tests/util.rs"]
mod util;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

//=============================================
//            Phase 6.1 — Runtime Parity Testing
//=============================================
/// Compile each paired `.svs` example to bytecode and compare execution
/// output with the interpreted run. Emits a parity summary for quick review.
#[test]
fn source_and_bytecode_outputs_match() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let pairs = collect_paired_examples(&examples_dir);
    assert!(
        !pairs.is_empty(),
        "expected at least one .svs/.svc example pair in {}",
        examples_dir.display()
    );

    let mut summary = Vec::new();

    for svs_path in pairs {
        let svs_stem = svs_path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("example");
        let svs_name = format!("{svs_stem}.svs");
        let svc_name = format!("{svs_stem}.svc");

        let work_dir = tempdir().expect("create temporary parity workspace");
        let compiled_path = work_dir.path().join(&svc_name);

        util::compile_to_svc(&svs_path, &compiled_path);
        let source_output = util::run_svs_file(&svs_path);
        let bytecode_output = util::run_svc_file(&compiled_path);

        assert_eq!(
            source_output, bytecode_output,
            "output mismatch for example {}",
            svs_name
        );

        summary.push(format!("[✓] {} vs {} — identical", svs_name, svc_name));
        // TempDir drops here, cleaning compiled artifact for this example.
    }

    println!();
    println!("Runtime parity summary:");
    for line in summary {
        println!("{line}");
    }
}

//=============================================
//            Helpers
//=============================================
/// Collect `.svs` examples that already have a sibling `.svc` file. These
/// samples are expected to execute successfully in both modes.
fn collect_paired_examples(examples_dir: &Path) -> Vec<PathBuf> {
    let mut svs_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("svs")) {
                let svc_path = path.with_extension("svc");
                if svc_path.exists() {
                    svs_files.push(path);
                }
            }
        }
    }
    svs_files.sort();
    svs_files
}
