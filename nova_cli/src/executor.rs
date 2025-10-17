//! Execution engine responsible for dispatching parsed statements.

use crate::command::{Argument, Pipeline, Redirection, RedirectionKind, Statement, Word};
use crate::config::CliConfig;
use crate::history::HistoryManager;
use crate::parser::Parser;
use crate::registry::{CommandInvocation, CommandKind, Registry};
use anyhow::{Context, Result};
use novascript::interpreter::Interpreter;
use novascript::parser::Parser as NovaParser;
use novascript::tokenizer::Tokenizer as NovaTokenizer;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Result returned from command execution.
#[derive(Debug, Clone)]
pub struct CommandOutcome {
    exit_code: i32,
    stdout: Option<String>,
}

impl CommandOutcome {
    /// Construct a successful outcome with the provided exit code.
    pub fn success(exit_code: i32) -> Self {
        Self {
            exit_code,
            stdout: None,
        }
    }

    /// Construct an outcome containing stdout output.
    pub fn with_stdout(exit_code: i32, stdout: String) -> Self {
        Self {
            exit_code,
            stdout: Some(stdout),
        }
    }

    /// Exit code associated with the command.
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Borrow stdout data if any.
    pub fn stdout(&self) -> Option<&str> {
        self.stdout.as_deref()
    }

    /// Consume and return stdout.
    pub fn into_stdout(self) -> Option<String> {
        self.stdout
    }
}

/// Execution context provided to command handlers.
pub struct ExecutionContext<'a> {
    registry: &'a Registry,
    config: &'a mut CliConfig,
    config_path: &'a PathBuf,
    history: &'a mut HistoryManager,
    should_exit: &'a mut bool,
    stdin: Option<String>,
}

impl<'a> ExecutionContext<'a> {
    /// Registry accessor.
    pub fn registry(&self) -> &Registry {
        self.registry
    }

    /// Mutable configuration accessor.
    pub fn config_mut(&mut self) -> &mut CliConfig {
        self.config
    }

    /// Current configuration path.
    pub fn config_path(&self) -> &Path {
        self.config_path.as_path()
    }

    /// Persist configuration to disk.
    pub fn persist_config(&self) -> Result<()> {
        self.config.save(self.config_path())
    }

    /// History accessor.
    pub fn history_mut(&mut self) -> &mut HistoryManager {
        self.history
    }

    /// Access pipeline stdin if available.
    pub fn stdin(&self) -> Option<&str> {
        self.stdin.as_deref()
    }

    /// Request shell termination.
    pub fn request_exit(&mut self) {
        *self.should_exit = true;
    }
}

/// Executor driving command dispatch and NovaScript evaluation.
pub struct Executor {
    registry: Arc<Registry>,
    config_path: PathBuf,
    config: CliConfig,
    history: HistoryManager,
    interpreter: Interpreter,
    should_exit: bool,
    suppress_output: bool,
}

impl Executor {
    /// Create a new executor from a registry, config, and history manager.
    pub fn new(
        registry: Arc<Registry>,
        config: CliConfig,
        config_path: PathBuf,
        history: HistoryManager,
    ) -> Self {
        Self {
            registry,
            config_path,
            config,
            history,
            interpreter: Interpreter::new(),
            should_exit: false,
            suppress_output: false,
        }
    }

    /// Access the immutable configuration.
    pub fn config(&self) -> &CliConfig {
        &self.config
    }

    /// Mutable configuration accessor.
    pub fn config_mut(&mut self) -> &mut CliConfig {
        &mut self.config
    }

    /// Borrow the command history manager.
    pub fn history(&self) -> &HistoryManager {
        &self.history
    }

    /// Mutable access to the history manager.
    pub fn history_mut(&mut self) -> &mut HistoryManager {
        &mut self.history
    }

    /// Persist history to disk.
    pub fn save_history(&self) -> Result<()> {
        self.history.save()
    }

    /// Convenience helper returning the prompt string.
    pub fn prompt(&self) -> String {
        self.config.prompt.clone()
    }

    /// Determine whether an exit request has been made.
    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// Execute a statement.
    pub fn execute(&mut self, statement: Statement) -> Result<CommandOutcome> {
        match statement {
            Statement::Empty => Ok(CommandOutcome::success(0)),
            Statement::NovaBlock(src) => self.execute_nova_block(&src),
            Statement::NovaExpression(expr) => self.execute_nova_expr(&expr),
            Statement::Pipeline(pipeline) => self.execute_pipeline(pipeline),
        }
    }

    fn context(&mut self, stdin: Option<String>) -> ExecutionContext<'_> {
        ExecutionContext {
            registry: &self.registry,
            config: &mut self.config,
            config_path: &self.config_path,
            history: &mut self.history,
            should_exit: &mut self.should_exit,
            stdin,
        }
    }

    fn execute_nova_block(&mut self, src: &str) -> Result<CommandOutcome> {
        let mut tokenizer = NovaTokenizer::new(src);
        let tokens = tokenizer.tokenize().map_err(|err| anyhow::anyhow!(err))?;
        let mut parser = NovaParser::new(tokens);
        let program = parser.parse().map_err(|err| anyhow::anyhow!(err))?;
        let result = self
            .interpreter
            .eval_program(&program)
            .map_err(|e| anyhow::anyhow!("NovaScript error: {:?}", e))?;
        if let Some(val) = result {
            println!("{}", val);
        }
        Ok(CommandOutcome::success(0))
    }

    fn execute_nova_expr(&mut self, expr: &str) -> Result<CommandOutcome> {
        let mut tokenizer = NovaTokenizer::new(expr);
        let tokens = tokenizer.tokenize().map_err(|err| anyhow::anyhow!(err))?;
        let mut parser = NovaParser::new(tokens);
        let program = parser.parse().map_err(|err| anyhow::anyhow!(err))?;
        let result = self
            .interpreter
            .eval_program(&program)
            .map_err(|e| anyhow::anyhow!("NovaScript error: {:?}", e))?;
        if let Some(val) = result {
            println!("{}", val);
        }
        Ok(CommandOutcome::success(0))
    }

    fn execute_pipeline(&mut self, mut pipeline: Pipeline) -> Result<CommandOutcome> {
        if pipeline.is_empty() {
            return Ok(CommandOutcome::success(0));
        }
        let total = pipeline.commands().len();
        let mut input: Option<String> = None;
        let mut last_outcome = CommandOutcome::success(0);
        for (index, command) in pipeline.commands_mut().iter_mut().enumerate() {
            let argv = self.expand_command(command)?;
            if argv.is_empty() {
                continue;
            }
            let mut stdin_data = input.take();
            for redir in command.redirections() {
                if let RedirectionKind::Input = redir.kind() {
                    let path = self.expand_argument(redir.target())?;
                    let contents = std::fs::read_to_string(&path)
                        .with_context(|| format!("reading input redirection from {}", path))?;
                    stdin_data = Some(contents);
                }
            }
            let expanded = if let Some(expanded) = self.registry.expand_alias(&argv[0], &argv[1..])
            {
                expanded
            } else {
                argv
            };
            let program = expanded.first().cloned().unwrap();
            let args = expanded.into_iter().skip(1).collect::<Vec<_>>();
            let resolution = self.registry.resolve(&program);
            let mut ctx = self.context(stdin_data.clone());
            let outcome = match resolution {
                Some((CommandKind::Builtin, handler)) => handler.handle(
                    &mut ctx,
                    &CommandInvocation::new(program.clone(), args.clone(), stdin_data.clone()),
                )?,
                Some((CommandKind::Nova, handler)) => handler.handle(
                    &mut ctx,
                    &CommandInvocation::new(program.clone(), args.clone(), stdin_data.clone()),
                )?,
                None => self.run_external(&program, &args, stdin_data.clone())?,
            };
            last_outcome =
                self.handle_redirections(command.redirections(), outcome, index + 1 == total)?;
            input = last_outcome.stdout().map(ToOwned::to_owned);
        }
        Ok(last_outcome)
    }

    fn expand_command(&mut self, command: &crate::command::Command) -> Result<Vec<String>> {
        let mut args = Vec::with_capacity(command.args().len() + 1);
        args.push(self.expand_argument(command.program())?);
        for arg in command.args() {
            args.push(self.expand_argument(arg)?);
        }
        Ok(args)
    }

    fn expand_argument(&mut self, arg: &Argument) -> Result<String> {
        let mut result = String::new();
        for part in arg.parts() {
            match part {
                Word::Text(text) => result.push_str(text),
                Word::Env(name) => {
                    let value = std::env::var(name).unwrap_or_default();
                    result.push_str(&value);
                }
                Word::Command(src) => {
                    let output = self.execute_substitution(src)?;
                    result.push_str(output.trim_end());
                }
            }
        }
        Ok(result)
    }

    fn execute_substitution(&mut self, src: &str) -> Result<String> {
        let mut parser = Parser::new();
        let statement = parser.parse(src)?;
        let previous = self.suppress_output;
        self.suppress_output = true;
        let result = self.execute(statement)?;
        self.suppress_output = previous;
        Ok(result.stdout().unwrap_or_default().to_string())
    }

    fn handle_redirections(
        &mut self,
        redirections: &[Redirection],
        mut outcome: CommandOutcome,
        is_last: bool,
    ) -> Result<CommandOutcome> {
        for redir in redirections {
            match redir.kind() {
                RedirectionKind::Output | RedirectionKind::Append => {
                    let path = self.expand_argument(redir.target())?;
                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(matches!(redir.kind(), RedirectionKind::Output))
                        .append(matches!(redir.kind(), RedirectionKind::Append))
                        .open(&path)
                        .with_context(|| format!("opening redirection target {}", path))?;
                    if let Some(stdout) = outcome.stdout() {
                        file.write_all(stdout.as_bytes())?;
                    }
                    if is_last {
                        outcome = CommandOutcome::success(outcome.exit_code());
                    }
                }
                RedirectionKind::Input => {}
            }
        }
        if is_last && !self.suppress_output {
            if let Some(stdout) = outcome.stdout() {
                print!("{}", stdout);
                std::io::stdout().flush().ok();
            }
        }
        Ok(outcome)
    }

    fn run_external(
        &self,
        program: &str,
        args: &[String],
        stdin_data: Option<String>,
    ) -> Result<CommandOutcome> {
        crate::cmd::external::run(program, args, stdin_data.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::builtin;
    use crate::config::CliConfig;
    use crate::parser::Parser;
    use crate::registry::Registry;
    use tempfile::tempdir;

    #[test]
    fn test_expand_env() {
        std::env::set_var("NOVA_TEST_ENV", "ok");
        let registry = Arc::new(Registry::new());
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let history_path = temp.path().join("history");
        let history = crate::history::HistoryManager::with_path(history_path);
        let mut executor = Executor::new(registry, CliConfig::default(), config_path, history);
        let mut parser = Parser::new();
        let statement = parser.parse("echo $NOVA_TEST_ENV").unwrap();
        let outcome = executor.execute(statement).unwrap();
        assert_eq!(outcome.stdout(), Some("ok\n"));
    }

    #[test]
    fn alias_invocation_dispatches() {
        let registry = Arc::new(Registry::new());
        builtin::register(&registry);
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let history_path = temp.path().join("history");
        let history = crate::history::HistoryManager::with_path(history_path);
        let mut executor = Executor::new(
            Arc::clone(&registry),
            CliConfig::default(),
            config_path,
            history,
        );
        registry.set_alias("hi".to_string(), "echo hello".to_string());
        let mut parser = Parser::new();
        let statement = parser.parse("hi").unwrap();
        let outcome = executor.execute(statement).unwrap();
        assert_eq!(outcome.stdout(), Some("hello\n"));
    }
}
