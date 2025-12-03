pub mod ast;
pub mod bytecode;
pub mod interpreter;
pub mod parser;
pub mod resolver;
pub mod runtime;
pub mod symbol;
pub mod tokenizer;
pub mod vm;
pub mod compiler {
    pub mod tier1;
    pub mod tier2;
}
pub mod compat;
pub mod core_bridge;
pub mod ir;
pub mod modules;
pub mod platform;
pub mod stdlib_registry;
pub mod stdx;

#[cfg(test)]
pub mod tests;
