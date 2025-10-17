//! Helpers for executing external processes in a cross-platform manner.

use crate::executor::CommandOutcome;
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Execute an external program returning a [`CommandOutcome`].
pub fn run(program: &str, args: &[String], stdin: Option<&str>) -> Result<CommandOutcome> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::inherit());
    let mut child = command
        .spawn()
        .with_context(|| format!("spawning {}", program))?;
    if let Some(input) = stdin {
        if let Some(mut child_stdin) = child.stdin.take() {
            child_stdin.write_all(input.as_bytes())?;
        }
    }
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(CommandOutcome::with_stdout(
        output.status.code().unwrap_or_default(),
        stdout,
    ))
}
