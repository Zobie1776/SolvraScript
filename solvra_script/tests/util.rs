use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use tempfile::tempdir;

pub fn run_svs_source(src: &str) -> String {
    let dir = tempdir().expect("create temp dir");
    let script_path = dir.path().join("script.svs");
    fs::write(&script_path, src).expect("write script");
    run_script(&script_path)
}

pub fn run_svs_file<P: AsRef<Path>>(path: P) -> String {
    run_script(path.as_ref())
}

pub fn run_svc_file<P: AsRef<Path>>(path: P) -> String {
    run_script(path.as_ref())
}

pub fn run_svs_source_expect_err(src: &str) -> CommandOutput {
    let dir = tempdir().expect("create temp dir");
    let script_path = dir.path().join("script.svs");
    fs::write(&script_path, src).expect("write script");
    run_script_expect_failure(&script_path)
}

pub fn compile_to_svc<PI: AsRef<Path>, PO: AsRef<Path>>(input: PI, output: PO) {
    let input_path = input.as_ref();
    let output_path = output.as_ref();

    let dir = tempdir().expect("create temp dir");
    let temp_input = dir.path().join(
        input_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("input.svs")),
    );
    fs::copy(input_path, &temp_input).expect("copy source");

    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("solvrascript")
        .arg("--bin")
        .arg("solvra_compile")
        .arg(&temp_input);
    let output = execute(command);
    if !output.status.success() {
        panic!(
            "solvra_compile failed: {}\nstdout:\n{}\nstderr:\n{}",
            output.status, output.stdout, output.stderr
        );
    }

    let compiled = temp_input.with_extension("svc");
    if let Some(parent) = output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::copy(compiled, output_path).expect("copy compiled output");
}

fn run_script(path: &Path) -> String {
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("solvrascript")
        .arg("--bin")
        .arg("solvrascript")
        .arg(path);
    let output = execute(command);
    if !output.status.success() {
        panic!(
            "command failed: {}\nstdout:\n{}\nstderr:\n{}",
            output.status, output.stdout, output.stderr
        );
    }
    output.stdout
}

fn run_script_expect_failure(path: &Path) -> CommandOutput {
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("solvrascript")
        .arg("--bin")
        .arg("solvrascript")
        .arg(path);
    let output = execute(command);
    assert!(
        !output.status.success(),
        "expected command to fail but it succeeded"
    );
    output
}

fn execute(mut command: Command) -> CommandOutput {
    let output = command
        .current_dir(workspace_root())
        .env("SOLVRA_TRACE", "0")
        .output()
        .expect("failed to run command");
    CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status,
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has parent directory")
        .to_path_buf()
}

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: ExitStatus,
}
