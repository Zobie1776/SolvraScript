use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Opcode {
    Nop = 0,
    LoadConst,
    LoadVar,
    StoreVar,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Not,
    Pop,
    Jump,
    JumpIfFalse,
    MakeList,
    LoadLambda,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    CallBuiltin,
    Call,
    CallAsync,
    Await,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand_a: u32,
    pub operand_b: u32,
    pub debug: Option<u32>,
}

impl Instruction {
    pub fn new(opcode: Opcode, operand_a: u32, operand_b: u32, debug: Option<u32>) -> Self {
        Self {
            opcode,
            operand_a,
            operand_b,
            debug,
        }
    }
}
