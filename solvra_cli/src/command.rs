//! Data structures describing parsed SolvraCLI command pipelines.

use std::fmt;

/// A top-level statement parsed from the interactive shell.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// A pipeline consisting of one or more commands separated by `|`.
    Pipeline(Pipeline),
    /// A multi-line SolvraScript block executed with the embedded interpreter.
    SolvraBlock(String),
    /// A single-line SolvraScript expression executed via the interpreter.
    SolvraExpression(String),
    /// Represents an empty or whitespace-only input.
    Empty,
}

/// Sequence of commands connected through pipes.
#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    commands: Vec<Command>,
    background: bool,
}

impl Pipeline {
    /// Creates a new pipeline.
    pub fn new(commands: Vec<Command>, background: bool) -> Self {
        Self {
            commands,
            background,
        }
    }

    /// Borrow commands in the pipeline.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Borrow commands mutably.
    pub fn commands_mut(&mut self) -> &mut [Command] {
        &mut self.commands
    }

    /// Returns `true` when there are no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Returns true when the pipeline should run in the background.
    pub fn background(&self) -> bool {
        self.background
    }
}

/// Represents a single command invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    program: Argument,
    args: Vec<Argument>,
    redirections: Vec<Redirection>,
}

impl Command {
    /// Construct a new command.
    pub fn new(program: Argument, args: Vec<Argument>, redirections: Vec<Redirection>) -> Self {
        Self {
            program,
            args,
            redirections,
        }
    }

    /// Name or path of the command to execute.
    pub fn program(&self) -> &Argument {
        &self.program
    }

    /// Arguments passed to the command (not including the program itself).
    pub fn args(&self) -> &[Argument] {
        &self.args
    }

    /// Redirection definitions attached to the command.
    pub fn redirections(&self) -> &[Redirection] {
        &self.redirections
    }

    /// Mutably access the arguments.
    pub fn args_mut(&mut self) -> &mut [Argument] {
        &mut self.args
    }
}

/// Shell words composing an argument.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    parts: Vec<Word>,
}

impl Argument {
    /// Construct an argument from words.
    pub fn new(parts: Vec<Word>) -> Self {
        Self { parts }
    }

    /// Returns the parts composing the argument.
    pub fn parts(&self) -> &[Word] {
        &self.parts
    }

    /// Returns true when the argument contains no parts.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for part in &self.parts {
            if !first {
                write!(f, " ")?;
            }
            match part {
                Word::Text(text) => write!(f, "{}", text)?,
                Word::Env(var) => write!(f, "${}", var)?,
                Word::Command(sub) => write!(f, "$({})", sub)?,
            }
            first = false;
        }
        Ok(())
    }
}

/// Individual shell word components.
#[derive(Debug, Clone, PartialEq)]
pub enum Word {
    /// Literal text extracted from the input.
    Text(String),
    /// Environment variable expansion placeholder.
    Env(String),
    /// Command substitution placeholder.
    Command(String),
}

/// Redirection variants supported by the shell.
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectionKind {
    /// Redirect standard input from the provided file.
    Input,
    /// Redirect standard output to the provided file (truncating first).
    Output,
    /// Redirect standard output appending to the provided file.
    Append,
    /// Merge stderr into stdout (`2>&1`).
    StderrToStdout,
}

/// A redirection definition for a command.
#[derive(Debug, Clone, PartialEq)]
pub struct Redirection {
    kind: RedirectionKind,
    target: Option<Argument>,
}

impl Redirection {
    /// Create a redirection instance.
    pub fn new(kind: RedirectionKind, target: Option<Argument>) -> Self {
        Self { kind, target }
    }

    /// Borrow the redirection kind.
    pub fn kind(&self) -> &RedirectionKind {
        &self.kind
    }

    /// Borrow the redirection target argument.
    pub fn target(&self) -> Option<&Argument> {
        self.target.as_ref()
    }
}
