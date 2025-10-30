//! Solvra-specific sub-commands dispatched through the `solvra` builtin.

use crate::executor::{CommandOutcome, ExecutionContext};
use crate::registry::{CommandHandler, CommandInvocation};
use anyhow::{bail, Result};
use std::sync::Arc;

/// Register Solvra verbs under the shared `solvra` entry.
pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.insert_solvra("solvra", Arc::new(SolvraCommand));
}

struct SolvraCommand;

impl CommandHandler for SolvraCommand {
    fn name(&self) -> &str {
        "solvra"
    }

    fn summary(&self) -> &str {
        "Solvra project helper commands"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let mut args = invocation.args.clone();
        if args.is_empty() {
            return Ok(CommandOutcome::with_stdout(0, solvra_help()));
        }
        let sub = args.remove(0);
        match sub.as_str() {
            "version" => Ok(CommandOutcome::with_stdout(0, "SolvraCLI v0.1".to_string())),
            "help" => Ok(CommandOutcome::with_stdout(0, solvra_help())),
            "about" => Ok(CommandOutcome::with_stdout(0, about_message())),
            other => bail!("unknown solvra command: {}", other),
        }
    }
}

fn solvra_help() -> String {
    "solvra <command>\n  version   Print SolvraCLI version\n  help      Show this help message\n  about     Learn more about SolvraCLI".to_string()
}

fn about_message() -> String {
    "SolvraCLI integrates SolvraScript with a friendly command line.".to_string()
}
