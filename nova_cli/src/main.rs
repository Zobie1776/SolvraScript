//! NovaCLI entry point launching the interactive shell.

use nova_cli::NovaTerminal;

fn main() {
    if let Err(error) = NovaTerminal::new().and_then(|mut terminal| terminal.run()) {
        eprintln!("nova-cli error: {error:?}");
        std::process::exit(1);
    }
}
