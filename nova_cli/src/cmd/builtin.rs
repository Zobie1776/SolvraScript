//! Built-in command implementations for NovaCLI.

use crate::cmd::app_store::AppStoreCommand;
use crate::cmd::external;
use crate::executor::{CommandOutcome, ExecutionContext};
use crate::registry::{CommandHandler, CommandInvocation, Registry};
use anyhow::{anyhow, bail, Context, Result};
use directories::UserDirs;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

/// Register all built-in commands.
pub fn register(registry: &Arc<Registry>) {
    registry.insert_builtin("cd", Arc::new(Cd));
    registry.insert_builtin("pwd", Arc::new(Pwd));
    registry.insert_builtin("echo", Arc::new(Echo));
    registry.insert_builtin("exit", Arc::new(Exit));
    registry.insert_builtin("history", Arc::new(History));
    registry.insert_builtin("alias", Arc::new(Alias));
    registry.insert_builtin("unalias", Arc::new(Unalias));
    registry.insert_builtin("export", Arc::new(Export));
    registry.insert_builtin("unset", Arc::new(Unset));
    registry.insert_builtin("help", Arc::new(Help));
    registry.insert_builtin("app", Arc::new(AppStoreCommand));
    registry.insert_builtin("ls", Arc::new(ExternalProxy("ls".into())));
    registry.insert_builtin("cat", Arc::new(ExternalProxy("cat".into())));
    registry.insert_builtin("cp", Arc::new(ExternalProxy("cp".into())));
    registry.insert_builtin("mv", Arc::new(ExternalProxy("mv".into())));
    registry.insert_builtin("rm", Arc::new(ExternalProxy("rm".into())));
}

struct Cd;

impl CommandHandler for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn summary(&self) -> &str {
        "Change the current working directory"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let target = invocation
            .args
            .first()
            .cloned()
            .map(PathBuf::from)
            .or_else(|| UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf()));
        let path = target.ok_or_else(|| anyhow!("unable to determine home directory"))?;
        env::set_current_dir(&path)
            .with_context(|| format!("changing directory to {}", path.display()))?;
        Ok(CommandOutcome::success(0))
    }
}

struct Pwd;

impl CommandHandler for Pwd {
    fn name(&self) -> &str {
        "pwd"
    }

    fn summary(&self) -> &str {
        "Print the current working directory"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        _invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let cwd = env::current_dir()?;
        Ok(CommandOutcome::with_stdout(
            0,
            format!("{}\n", cwd.display()),
        ))
    }
}

struct Echo;

impl CommandHandler for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn summary(&self) -> &str {
        "Echo arguments back to the terminal"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        if invocation.args.is_empty() {
            if let Some(stdin) = invocation.stdin.clone() {
                return Ok(CommandOutcome::with_stdout(0, stdin));
            }
        }
        let output = invocation.args.join(" ") + "\n";
        Ok(CommandOutcome::with_stdout(0, output))
    }
}

struct Exit;

impl CommandHandler for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn summary(&self) -> &str {
        "Exit the NovaCLI session"
    }

    fn handle(
        &self,
        ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        ctx.request_exit();
        let code = invocation
            .args
            .first()
            .and_then(|arg| arg.parse::<i32>().ok())
            .unwrap_or(0);
        Ok(CommandOutcome::success(code))
    }
}

struct History;

impl CommandHandler for History {
    fn name(&self) -> &str {
        "history"
    }

    fn summary(&self) -> &str {
        "Show the command history"
    }

    fn handle(
        &self,
        ctx: &mut ExecutionContext<'_>,
        _invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let mut output = String::new();
        for (idx, entry) in ctx.history_mut().entries().enumerate() {
            output.push_str(&format!("{:>5}  {}\n", idx + 1, entry));
        }
        Ok(CommandOutcome::with_stdout(0, output))
    }
}

struct Alias;

impl CommandHandler for Alias {
    fn name(&self) -> &str {
        "alias"
    }

    fn summary(&self) -> &str {
        "Create or list aliases"
    }

    fn handle(
        &self,
        ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        if invocation.args.is_empty() {
            let mut output = String::new();
            let mut aliases: Vec<_> = ctx.registry().aliases().into_iter().collect();
            aliases.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, value) in aliases {
                output.push_str(&format!("alias {}='{}'\n", name, value));
            }
            return Ok(CommandOutcome::with_stdout(0, output));
        }
        let mut updated = false;
        if invocation.args.len() >= 2 && !invocation.args[0].contains('=') {
            let name = invocation.args[0].clone();
            let value = invocation.args[1..].join(" ");
            ctx.registry().set_alias(name.clone(), value.clone());
            ctx.config_mut().aliases.insert(name, value);
            updated = true;
        } else {
            for arg in &invocation.args {
                if let Some((name, value)) = arg.split_once('=') {
                    let name = name.trim().to_string();
                    let value = value.trim().to_string();
                    ctx.registry().set_alias(name.clone(), value.clone());
                    ctx.config_mut().aliases.insert(name, value);
                    updated = true;
                } else {
                    bail!("alias requires NAME=VALUE assignments");
                }
            }
        }
        if updated {
            ctx.persist_config()?;
        }
        Ok(CommandOutcome::success(0))
    }
}

struct Unalias;

impl CommandHandler for Unalias {
    fn name(&self) -> &str {
        "unalias"
    }

    fn summary(&self) -> &str {
        "Remove an alias"
    }

    fn handle(
        &self,
        ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        if invocation.args.is_empty() {
            bail!("unalias requires at least one alias name");
        }
        for name in &invocation.args {
            if ctx.registry().remove_alias(name) {
                ctx.config_mut().aliases.remove(name);
            }
        }
        ctx.persist_config()?;
        Ok(CommandOutcome::success(0))
    }
}

struct Export;

impl CommandHandler for Export {
    fn name(&self) -> &str {
        "export"
    }

    fn summary(&self) -> &str {
        "Set an environment variable"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        if invocation.args.is_empty() {
            bail!("export requires NAME=VALUE");
        }
        for arg in &invocation.args {
            let (name, value) = if let Some((name, value)) = arg.split_once('=') {
                (name.trim(), value.trim())
            } else {
                bail!("export expects assignments like NAME=VALUE");
            };
            env::set_var(name, value);
        }
        Ok(CommandOutcome::success(0))
    }
}

struct Unset;

impl CommandHandler for Unset {
    fn name(&self) -> &str {
        "unset"
    }

    fn summary(&self) -> &str {
        "Remove an environment variable"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        if invocation.args.is_empty() {
            bail!("unset requires a variable name");
        }
        for name in &invocation.args {
            env::remove_var(name);
        }
        Ok(CommandOutcome::success(0))
    }
}

struct Help;

impl CommandHandler for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn summary(&self) -> &str {
        "Show available commands"
    }

    fn handle(
        &self,
        ctx: &mut ExecutionContext<'_>,
        _invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let mut entries = Vec::new();
        for name in ctx
            .registry()
            .commands_by_kind(crate::registry::CommandKind::Builtin)
        {
            entries.push(format!("{} (builtin)", name));
        }
        for name in ctx
            .registry()
            .commands_by_kind(crate::registry::CommandKind::Nova)
        {
            entries.push(format!("{} (nova)", name));
        }
        entries.sort();
        let mut output = String::from("Available commands:\n");
        for entry in entries {
            output.push_str(&format!("  {}\n", entry));
        }
        Ok(CommandOutcome::with_stdout(0, output))
    }
}

struct ExternalProxy(String);

impl CommandHandler for ExternalProxy {
    fn name(&self) -> &str {
        &self.0
    }

    fn summary(&self) -> &str {
        "Proxy to a system command"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let args = invocation.args.clone();
        external::run(&self.0, &args, invocation.stdin.as_deref())
    }
}
