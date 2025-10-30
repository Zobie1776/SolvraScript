//================================================
// [Solvrac Main ASM]
//================================================
// Author: SolvraOS Contributors
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
use solvra_core::solvrac::{self, Bytecode};


#[derive(Parser)]
#[command(author, version, about = "SolvraCore assembler and disassembler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Assemble textual SolvraCore bytecode into binary .svc files
    Assemble {
        /// Input assembly file
        input: PathBuf,
        /// Output path (defaults to input with .svc extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Disassemble binary .svc files back into textual assembly
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
            let bytecode = solvrac::assemble(&source)?;
            let bytes = bytecode.encode()?;
            let output_path = output.unwrap_or_else(|| input.with_extension("svc"));
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
            let assembly = solvrac::disassemble(&bytecode)?;
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
