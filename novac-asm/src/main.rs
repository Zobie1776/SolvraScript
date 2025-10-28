//================================================
// [Novac Main ASM]
//================================================
// Author: NovaOS Contributors
// License: Apache 2.0
// Goal:
// Objective:
//================================================

//================================================
// Imports/Modules
//================================================
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nova_core::novac::{self, Bytecode};


#[derive(Parser)]
#[command(author, version, about = "NovaCore assembler and disassembler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Assemble textual NovaCore bytecode into binary .nvc files
    Assemble {
        /// Input assembly file
        input: PathBuf,
        /// Output path (defaults to input with .nvc extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Disassemble binary .nvc files back into textual assembly
    Disassemble {
        /// Input binary file
        input: PathBuf,
        /// Output path (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Assemble { input, output } => {
            let source = fs::read_to_string(&input)
                .with_context(|| format!("failed to read {}", input.display()))?;
            let bytecode = novac::assemble(&source)?;
            let bytes = bytecode.encode()?;
            let output_path = output.unwrap_or_else(|| input.with_extension("nvc"));
            fs::write(&output_path, &bytes)
                .with_context(|| format!("failed to write {}", output_path.display()))?;
            println!(
                "Assembled {} -> {} ({} bytes)",
                input.display(),
                output_path.display(),
                bytes.len()
            );
        }
        Command::Disassemble { input, output } => {
            let bytes =
                fs::read(&input).with_context(|| format!("failed to read {}", input.display()))?;
            let bytecode = Bytecode::decode(&bytes)?;
            let assembly = novac::disassemble(&bytecode)?;
            if let Some(path) = output {
                let mut file = fs::File::create(&path)
                    .with_context(|| format!("failed to create {}", path.display()))?;
                file.write_all(assembly.as_bytes())
                    .with_context(|| format!("failed to write {}", path.display()))?;
                println!(
                    "Disassembled {} -> {} ({} instructions)",
                    input.display(),
                    path.display(),
                    bytecode
                        .functions
                        .iter()
                        .map(|f| f.instructions.len())
                        .sum::<usize>()
                );
            } else {
                print!("{}", assembly);
            }
        }
    }

    Ok(())
}
