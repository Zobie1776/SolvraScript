//! Nova specific sub-commands dispatched through the `nova` builtin.

use crate::executor::{CommandOutcome, ExecutionContext};
use crate::registry::{CommandHandler, CommandInvocation};
use anyhow::{bail, Result};
use std::sync::Arc;

/// Register nova verbs under the shared `nova` entry.
pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.insert_nova("nova", Arc::new(NovaCommand));
}

struct NovaCommand;

impl CommandHandler for NovaCommand {
    fn name(&self) -> &str {
        "nova"
    }

    fn summary(&self) -> &str {
        "Nova project helper commands"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        let mut args = invocation.args.clone();
        if args.is_empty() {
            return Ok(CommandOutcome::with_stdout(0, nova_help()));
        }
        let sub = args.remove(0);
        match sub.as_str() {
            "version" => Ok(CommandOutcome::with_stdout(0, "NovaCLI v0.1".to_string())),
            "help" => Ok(CommandOutcome::with_stdout(0, nova_help())),
            "about" => Ok(CommandOutcome::with_stdout(0, about_message())),
            other => bail!("unknown nova command: {}", other),
        }
    }
}

fn nova_help() -> String {
    "nova <command>\n  version   Print NovaCLI version\n  help      Show this help message\n  about     Learn more about NovaCLI".to_string()
}

fn about_message() -> String {
    "NovaCLI integrates NovaScript with a friendly command line.".to_string()
}
