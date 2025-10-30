//! Intermediate representation used as a staging ground before emitting bytecode.

use std::sync::Arc;

use super::{ast::Span, spec::Constant};

/// Identifier referencing a label within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(pub usize);

/// Span information preserved across lowering stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrSpan {
    file: Arc<str>,
    line: u32,
    column: u32,
}

impl IrSpan {
    pub fn new(file: Arc<str>, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }

    pub fn from_span(span: &Span) -> Self {
        Self {
            file: span.file().clone(),
            line: span.line(),
            column: span.column(),
        }
    }

    pub fn file(&self) -> &Arc<str> {
        &self.file
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn column(&self) -> u32 {
        self.column
    }
}

/// Intermediate representation for the entire program.
#[derive(Debug, Clone)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    pub entry: usize,
}

impl IrProgram {
    pub fn new(functions: Vec<IrFunction>, entry: usize) -> Self {
        Self { functions, entry }
    }
}

/// A function inside the intermediate representation.
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub arity: u16,
    pub locals: u16,
    pub instructions: Vec<IrInstruction>,
}

impl IrFunction {
    pub fn new(
        name: impl Into<String>,
        arity: u16,
        locals: u16,
        instructions: Vec<IrInstruction>,
    ) -> Self {
        Self {
            name: name.into(),
            arity,
            locals,
            instructions,
        }
    }
}

/// Instruction inside the intermediate representation.
#[derive(Debug, Clone)]
pub struct IrInstruction {
    pub opcode: IrOpcode,
    pub span: IrSpan,
}

impl IrInstruction {
    pub fn new(opcode: IrOpcode, span: IrSpan) -> Self {
        Self { opcode, span }
    }
}

/// Call target used by [`IrOpcode::Call`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallTarget {
    Function(usize),
    Sative(usize),
}

/// Stack based intermediate opcodes. They closely mirror the final bytecode but keep labels
/// explicit to make control flow optimisations easier.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq)]
pub enum IrOpcode {
    Label(LabelId),
    PushConst(Constant),
    LoadLocal(u16),
    StoreLocal(u16),
    LoadGlobal(u32),
    StoreGlobal(u32),
    Jump(LabelId),
    JumpIfFalse(LabelId),
    JumpIfTrue(LabelId),
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Negate,
    Equals,
    NotEquals,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LogicalAnd,
    LogicalOr,
    LogicalNot,
    Call { target: CallTarget, args: u16 },
    Return,
    Pop,
    BuildList(u16),
    Index,
    StoreIndex,
    PushCatch { handler: LabelId },
    PopCatch,
    Throw,
}

/// Result of optimisation passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Optimisation {
    /// Optimisation changed the IR.
    Changed,
    /// No changes were made.
    Unchanged,
}
