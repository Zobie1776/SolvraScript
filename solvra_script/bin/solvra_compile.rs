use anyhow::{Result, anyhow};
use solvrascript::{parser::Parser, tokenizer::Tokenizer, vm::compiler};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    let src = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("no input file provided"))?;
    let code = fs::read_to_string(&src)?;
    let mut tokenizer = Tokenizer::new(&code);
    let tokens = tokenizer
        .tokenize()
        .map_err(|err| anyhow!("tokenizer error: {err}"))?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|err| anyhow!("parse error: {err}"))?;
    let bytecode = compiler::compile_program(&program)?;
    let out = Path::new(&src).with_extension("svc");
    fs::write(&out, bytecode)?;
    println!("Compiled -> {}", out.display());
    Ok(())
}
