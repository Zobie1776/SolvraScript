pub mod debugger;
pub mod file_explorer;
pub mod git_panel;
pub mod gui;
pub mod lsp_client;
pub mod settings;
pub mod solvra_ai;
pub mod theme;

pub use gui::{run_gui, SolvraIdeApp, SolvraIdeContext};
