//=============================================
// solvra_script/platform/sys_macos.rs
//=============================================
// Author: SolvraOS Contributors
// License: MIT (see LICENSE)
// Goal: macOS-specific platform implementation
// Objective: Provide Darwin-friendly operations for SolvraScript
// Formatting: Zobie.format (.solvraformat)
//=============================================

use super::{
    CommandResult, CommandSpec, PlatformError, PlatformOps, PlatformResult, StdioMode,
    SystemTimestamp,
};
use chrono::{Datelike, Timelike, Utc};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct MacOSPlatform;

impl PlatformOps for MacOSPlatform {
    fn system_time() -> PlatformResult<f64> {
        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .map_err(|err| PlatformError::IoError(err.to_string()))?;
        Ok(duration.as_secs_f64())
    }

    fn system_timestamp() -> PlatformResult<SystemTimestamp> {
        let now = Utc::now();
        Ok(SystemTimestamp {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
            second: now.second(),
            nanosecond: now.nanosecond(),
        })
    }

    fn read_file(path: &str) -> PlatformResult<String> {
        fs::read_to_string(path).map_err(PlatformError::from)
    }

    fn write_file(path: &str, data: &str) -> PlatformResult<()> {
        fs::write(path, data).map_err(PlatformError::from)
    }

    fn append_file(path: &str, data: &str) -> PlatformResult<()> {
        let mut file = OpenOptions::new().append(true).create(true).open(path)?;
        file.write_all(data.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    fn path_exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    fn list_directory(path: &str) -> PlatformResult<Vec<String>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                entries.push(name.to_string());
            }
        }
        Ok(entries)
    }

    fn env_get(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    unsafe fn env_set(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn home_dir() -> Option<String> {
        dirs::home_dir().map(|p| p.to_string_lossy().to_string())
    }

    fn path_join(left: &str, right: &str) -> String {
        PathBuf::from(left)
            .join(right)
            .to_string_lossy()
            .to_string()
    }

    fn canonicalize_path(path: &str) -> PlatformResult<String> {
        fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(PlatformError::from)
    }

    fn sleep(duration: Duration) {
        std::thread::sleep(duration);
    }

    fn print(text: &str) -> PlatformResult<()> {
        print!("{text}");
        io::stdout().flush().map_err(PlatformError::from)
    }

    fn println(text: &str) -> PlatformResult<()> {
        println!("{text}");
        Ok(())
    }

    fn eprintln(text: &str) -> PlatformResult<()> {
        eprintln!("{text}");
        Ok(())
    }

    fn flush_stdout() -> PlatformResult<()> {
        io::stdout().flush().map_err(PlatformError::from)
    }

    fn read_line(prompt: Option<&str>) -> PlatformResult<String> {
        if let Some(message) = prompt {
            print!("{message}");
            io::stdout().flush().map_err(PlatformError::from)?;
        }
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        while line.ends_with(['\n', '\r']) {
            line.pop();
        }
        Ok(line)
    }

    fn execute_command(spec: &CommandSpec) -> PlatformResult<CommandResult> {
        let mut cmd = build_command(spec);
        let output = cmd.output().map_err(PlatformError::from)?;
        Ok(CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn spawn_command(spec: &CommandSpec) -> PlatformResult<u32> {
        let mut cmd = build_command(spec);
        let child = cmd.spawn().map_err(PlatformError::from)?;
        Ok(child.id())
    }
}

fn build_command(spec: &CommandSpec) -> Command {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args);

    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }

    if spec.clear_env {
        cmd.env_clear();
    }

    if let Some(vars) = &spec.env {
        for (key, value) in vars {
            cmd.env(key, value);
        }
    }

    cmd.stdin(map_stdio(spec.stdin));
    cmd.stdout(map_stdio(spec.stdout));
    cmd.stderr(map_stdio(spec.stderr));
    cmd
}

fn map_stdio(mode: StdioMode) -> Stdio {
    match mode {
        StdioMode::Inherit => Stdio::inherit(),
        StdioMode::Null => Stdio::null(),
        StdioMode::Piped => Stdio::piped(),
    }
}
