//! Command registry containing builtins, nova verbs, and aliases.

use crate::executor::{CommandOutcome, ExecutionContext};
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Invocation information passed to command handlers.
#[derive(Debug, Clone)]
pub struct CommandInvocation {
    /// Command name after alias resolution.
    pub name: String,
    /// Arguments provided to the handler.
    pub args: Vec<String>,
    /// Optional stdin data produced by a previous pipeline stage.
    pub stdin: Option<String>,
}

impl CommandInvocation {
    /// Construct a new invocation.
    pub fn new(name: String, args: Vec<String>, stdin: Option<String>) -> Self {
        Self { name, args, stdin }
    }
}

/// Trait implemented by handlers registered in the [`Registry`].
pub trait CommandHandler: Send + Sync {
    /// Short descriptive name.
    fn name(&self) -> &str;
    /// One line summary suitable for `help` output.
    fn summary(&self) -> &str;
    /// Execute the handler.
    fn handle(
        &self,
        ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome>;
}

impl std::fmt::Debug for dyn CommandHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandHandler")
            .field("name", &self.name())
            .field("summary", &self.summary())
            .finish()
    }
}

/// Registry categories reflecting dispatch precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// Built-in command executed within the process.
    Builtin,
    /// Nova verb provided by the project.
    Nova,
}

/// Stored handler entry.
#[derive(Clone, Debug)]
struct Entry {
    handler: Arc<dyn CommandHandler>,
    kind: CommandKind,
}

/// Thread-safe registry for commands and aliases.
#[derive(Debug, Default)]
pub struct Registry {
    handlers: RwLock<HashMap<String, Entry>>,
    aliases: RwLock<HashMap<String, String>>,
}

impl Registry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a built-in command.
    pub fn insert_builtin(&self, name: &str, handler: Arc<dyn CommandHandler>) {
        self.handlers.write().insert(
            name.to_string(),
            Entry {
                handler,
                kind: CommandKind::Builtin,
            },
        );
    }

    /// Register a nova verb.
    pub fn insert_nova(&self, name: &str, handler: Arc<dyn CommandHandler>) {
        self.handlers.write().insert(
            name.to_string(),
            Entry {
                handler,
                kind: CommandKind::Nova,
            },
        );
    }

    /// Set an alias mapping.
    pub fn set_alias(&self, name: String, value: String) {
        self.aliases.write().insert(name, value);
    }

    /// Remove an alias.
    pub fn remove_alias(&self, name: &str) -> bool {
        self.aliases.write().remove(name).is_some()
    }

    /// Access current aliases.
    pub fn aliases(&self) -> HashMap<String, String> {
        self.aliases.read().clone()
    }

    /// Resolve a handler by command name returning its entry and metadata.
    pub fn resolve(&self, name: &str) -> Option<(CommandKind, Arc<dyn CommandHandler>)> {
        self.handlers
            .read()
            .get(name)
            .map(|entry| (entry.kind, Arc::clone(&entry.handler)))
    }

    /// Apply alias expansion by splitting on whitespace.
    pub fn expand_alias(&self, name: &str, args: &[String]) -> Option<Vec<String>> {
        let alias = self.aliases.read().get(name)?.clone();
        let mut expanded: Vec<String> = shell_words::split(&alias)
            .unwrap_or_else(|_| alias.split_whitespace().map(|s| s.to_string()).collect());
        expanded.extend(args.iter().cloned());
        Some(expanded)
    }

    /// List registered commands filtered by kind.
    pub fn commands_by_kind(&self, kind: CommandKind) -> Vec<String> {
        self.handlers
            .read()
            .iter()
            .filter(|(_, entry)| entry.kind == kind)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List of all command names.
    pub fn all_commands(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }
}

/// Helper type alias for shared handlers.
pub type SharedHandler = Arc<dyn CommandHandler>;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    struct TestHandler;

    impl CommandHandler for TestHandler {
        fn name(&self) -> &str {
            "test"
        }

        fn summary(&self) -> &str {
            "test handler"
        }

        fn handle(
            &self,
            _ctx: &mut ExecutionContext<'_>,
            _invocation: &CommandInvocation,
        ) -> Result<CommandOutcome> {
            Ok(CommandOutcome::success(0))
        }
    }

    #[test]
    fn test_alias_roundtrip() {
        let registry = Registry::new();
        registry.insert_builtin("test", Arc::new(TestHandler));
        registry.set_alias("t".to_string(), "test --flag".to_string());
        let expanded = registry.expand_alias("t", &["extra".into()]).unwrap();
        assert_eq!(expanded, vec!["test", "--flag", "extra"]);
    }
}
