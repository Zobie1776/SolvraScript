use crate::error::SolvraIdeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Result of executing a task via [`TaskRunner`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskOutcome {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunOptions {
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<String>,
    pub timeout: Option<u64>,
    pub program_override: Option<String>,
}

impl RunOptions {
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            env: HashMap::new(),
            cwd: None,
            timeout: None,
            program_override: None,
        }
    }

    pub fn shell(command: &str) -> Self {
        #[cfg(target_os = "windows")]
        let program = "cmd".to_string();
        #[cfg(not(target_os = "windows"))]
        let program = "sh".to_string();

        #[cfg(target_os = "windows")]
        let args = vec!["/C".to_string(), command.to_string()];
        #[cfg(not(target_os = "windows"))]
        let args = vec!["-c".to_string(), command.to_string()];

        let mut opts = Self::new(args);
        opts.program_override = Some(program);
        opts
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunTaskPayload {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub cwd: Option<String>,
    pub timeout: Option<u64>,
    #[serde(default)]
    pub shell: bool,
}

impl RunTaskPayload {
    pub fn into_parts(self) -> (String, RunOptions) {
        let mut options = RunOptions::new(self.args);
        options.env = self.env;
        options.cwd = self.cwd;
        options.timeout = self.timeout;
        if self.shell {
            let mut args = Vec::new();
            #[cfg(target_os = "windows")]
            let shell_program = "cmd".to_string();
            #[cfg(not(target_os = "windows"))]
            let shell_program = "sh".to_string();

            #[cfg(target_os = "windows")]
            let shell_flag = "/C".to_string();
            #[cfg(not(target_os = "windows"))]
            let shell_flag = "-c".to_string();

            args.push(shell_flag);
            args.push(self.command.clone());
            args.extend(options.args);
            options.args = args;
            return (shell_program, options);
        }
        (self.command, options)
    }
}

pub struct TaskRunner;

impl TaskRunner {
    pub const fn new() -> Self {
        Self
    }

    pub async fn run(
        &self,
        command: &str,
        options: RunOptions,
    ) -> Result<TaskOutcome, SolvraIdeError> {
        let program = options.program_override.as_deref().unwrap_or(command);

        let mut cmd = Command::new(program);
        if !options.args.is_empty() {
            cmd.args(&options.args);
        }
        for (key, value) in options.env {
            cmd.env(key, value);
        }
        if let Some(cwd) = options.cwd {
            cmd.current_dir(cwd);
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output_future = cmd.output();
        let output = if let Some(secs) = options.timeout {
            timeout(Duration::from_secs(secs), output_future)
                .await
                .map_err(|_| SolvraIdeError::task("task timed out"))?
                .map_err(|err| SolvraIdeError::task(err.to_string()))?
        } else {
            output_future
                .await
                .map_err(|err| SolvraIdeError::task(err.to_string()))?
        };

        let stdout_buf = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_buf = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status;

        Ok(TaskOutcome {
            command: command.to_string(),
            stdout: stdout_buf,
            stderr: stderr_buf,
            exit_code: status.code(),
        })
    }
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn run_true_succeeds() {
        let runner = TaskRunner::new();
        let outcome = runner
            .run("sh", RunOptions::shell("echo ready"))
            .await
            .expect("run task");
        assert!(outcome.stdout.contains("ready"));
    }

    #[tokio::test]
    async fn payload_conversion_sets_shell() {
        let payload = RunTaskPayload {
            command: "sh".into(),
            args: vec!["-c".into(), "echo hi".into()],
            env: HashMap::new(),
            cwd: None,
            timeout: None,
            shell: false,
        };
        let (command, options) = payload.into_parts();
        assert_eq!(command, "sh");
        assert_eq!(options.args.len(), 2);
    }
}
