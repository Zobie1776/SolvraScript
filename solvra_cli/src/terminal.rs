//! Terminal and REPL implementation backed by rustyline.

use crate::cmd::{builtin, solvra as solvra_cmd};
use crate::completer::SolvraHelper;
use crate::config::CliConfig;
use crate::executor::{CommandOutcome, Executor};
use crate::history::HistoryManager;
use crate::parser::Parser;
use crate::registry::Registry;
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::sync::Arc;

/// SolvraCLI interactive terminal driving user interaction.
pub struct SolvraTerminal {
    executor: Executor,
    parser: Parser,
    editor: Editor<SolvraHelper, DefaultHistory>,
    prompt: String,
}

impl SolvraTerminal {
    /// Create a new terminal instance with all components wired together.
    pub fn new() -> Result<Self> {
        let registry = Arc::new(Registry::new());
        builtin::register(&registry);
        solvra_cmd::register(&registry);

        let (config, config_path) = CliConfig::load()?;
        for (key, value) in config.env.clone() {
            std::env::set_var(key, value);
        }

        for (alias, value) in config.aliases.clone() {
            registry.set_alias(alias, value);
        }

        let history = HistoryManager::load()?;
        let executor = Executor::new(Arc::clone(&registry), config, config_path, history);
        let helper = SolvraHelper::new(Arc::clone(&registry));
        let mut editor = Editor::<SolvraHelper, DefaultHistory>::new()?;
        editor.set_helper(Some(helper));

        let prompt = executor.prompt();
        Ok(Self {
            executor,
            parser: Parser::new(),
            editor,
            prompt,
        })
    }

    /// Run the interactive loop until exit is requested.
    pub fn run(&mut self) -> Result<()> {
        let entries: Vec<String> = self.executor.history().entries().cloned().collect();
        for entry in &entries {
            let _ = self.editor.add_history_entry(entry.to_string());
        }
        loop {
            match self.editor.readline(&self.prompt) {
                Ok(line) => {
                    self.handle_line(line)?;
                    if self.executor.should_exit() {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    eprintln!("readline error: {err}");
                    break;
                }
            }
        }
        self.executor.save_history()?;
        Ok(())
    }

    /// Execute a single line of input (used by integration tests).
    pub fn handle_line(&mut self, line: String) -> Result<CommandOutcome> {
        if line.trim().is_empty() {
            return Ok(CommandOutcome::success(0));
        }
        let mut effective = line;
        if effective.trim() == "!!" {
            if let Some(prev) = self.executor.history().last() {
                println!("{}", prev);
                effective = prev.clone();
            } else {
                return Ok(CommandOutcome::success(0));
            }
        }
        let _ = self.editor.add_history_entry(effective.clone());
        self.executor.history_mut().add(&effective);
        let statement = self.parser.parse(&effective)?;
        let outcome = self.executor.execute(statement)?;
        Ok(outcome)
    }
}
