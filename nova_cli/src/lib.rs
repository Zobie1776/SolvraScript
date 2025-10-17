//! Core library for NovaCLI providing parsing, execution, and terminal utilities.

pub mod command;
pub mod completer;
pub mod config;
pub mod executor;
pub mod history;
pub mod parser;
pub mod registry;
pub mod terminal;
pub mod cmd {
    pub mod builtin;
    pub mod external;
    pub mod nova;
}

pub use terminal::NovaTerminal;
