use std::path::PathBuf;

use nova_ide_frontend::{gui::GuiLaunchOptions, nova_ai::NovaAiService, run_gui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchMode {
    Gui,
    Cli,
    Ai,
}

fn parse_launch_mode() -> LaunchMode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--cli") | Some("cli") => LaunchMode::Cli,
        Some("--ai") | Some("ai") => LaunchMode::Ai,
        _ => LaunchMode::Gui,
    }
}

fn main() -> eframe::Result<()> {
    match parse_launch_mode() {
        LaunchMode::Gui => {
            let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let options = GuiLaunchOptions {
                workspace_root: workspace,
            };
            run_gui(options)
        }
        LaunchMode::Cli => {
            println!("NovaIDE CLI mode is not yet feature complete. Use the GUI for the best experience.");
            Ok(())
        }
        LaunchMode::Ai => {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
            runtime.block_on(async {
                let mut service: NovaAiService = NovaAiService::default();
                service.interactive_cli().await
            });
            Ok(())
        }
    }
}
