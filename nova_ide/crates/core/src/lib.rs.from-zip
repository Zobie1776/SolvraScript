#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

//! NovaIDE core services.
//!
//! This crate bundles the cross-platform functionality required by the desktop
//! shell and language tooling: filesystem enumeration, workspace management,
//! build task orchestration, NovaScript language intelligence, a lightweight
//! debugger stub, and integration with the NovaFailSafe passphrase gate.

pub mod debugger;
pub mod error;
pub mod lsp;
pub mod tasks;
pub mod tree;
pub mod workspace;

pub use debugger::{DebugSession, DebuggerEvent, DebuggerState};
pub use error::NovaIdeError;
pub use lsp::{CompletionItem, Diagnostic, HoverResult, NovaLanguageServer, TextPosition};
pub use tasks::{RunOptions, TaskOutcome, TaskRunner};
pub use tree::{ProjectNode, ProjectTreeBuilder};
pub use workspace::{WorkspaceConfig, WorkspaceLoader};

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn run_task_echo() {
        let runner = TaskRunner::new();
        let result = runner
            .run("sh", RunOptions::shell("echo hello"))
            .await
            .expect("task runs");
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("echo hello") || result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn tree_builder_lists_files() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("main.nova"), "let x = 1").unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/lib.nova"), "fn add() {}").unwrap();

        let builder = ProjectTreeBuilder::default();
        let tree = builder.build(root).unwrap();
        assert_eq!(tree.name, root.file_name().unwrap().to_string_lossy());
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn workspace_loader_reads_defaults() {
        let tmp = tempdir().unwrap();
        let config_path = tmp.path().join("workspace.toml");
        fs::write(&config_path, "[task.default]\nbuild='nova build'\n").unwrap();

        let loader = WorkspaceLoader::new(config_path);
        let config = loader.load().unwrap();
        assert_eq!(config.task.default.build, "nova build");
    }

    #[test]
    fn lsp_provides_completions() {
        let source = "let foo = 1;\nfn bar() { foo; }";
        let server = NovaLanguageServer::new();
        let completions = server
            .complete(source, (1, 12).into())
            .expect("completions");
        assert!(completions.iter().any(|item| item.label == "foo"));
    }

    #[test]
    fn debugger_breakpoint_roundtrip() {
        let mut session = DebugSession::new();
        session.toggle_breakpoint(PathBuf::from("main.nova"), 3);
        assert!(session.has_breakpoint("main.nova", 3));
        session.toggle_breakpoint(PathBuf::from("main.nova"), 3);
        assert!(!session.has_breakpoint("main.nova", 3));
    }
}
