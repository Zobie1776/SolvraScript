pub mod assembler;
pub mod ast;
pub mod ir;
pub mod spec;
pub mod vm;

#[cfg(feature = "jit")]
pub mod jit;

pub use assembler::{assemble, AssemblyConfig};
pub use ast::{Ast, BinaryOp, Expr, Function, Span, Stmt, UnaryOp};
pub use ir::{IrFunction, IrInstruction, IrProgram, IrSpan};
pub use spec::{DebugSymbol, FunctionDescriptor, Opcode, SolvraBytecode};
