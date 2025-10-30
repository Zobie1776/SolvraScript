//! SolvraCLI entry point launching the interactive shell.

use solvra_cli::SolvraTerminal;

fn main() {
    if let Err(error) = SolvraTerminal::new().and_then(|mut terminal| terminal.run()) {
        eprintln!("solvra-cli error: {error:?}");
        std::process::exit(1);
    }
}
