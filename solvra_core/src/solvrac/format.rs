use std::convert::TryFrom;
use std::io::{Cursor, Read};

use thiserror::Error;

pub const MAGIC: &[u8; 4] = b"SVC1";
pub const VERSION: u8 = 1;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SolvracError {
    #[error("invalid file header")]
    InvalidHeader,
    #[error("unsupported version {0}")]
    UnsupportedVersion(u8),
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("invalid utf-8 sequence")]
    InvalidUtf8,
    #[error("unknown opcode {0}")]
    UnknownOpcode(u8),
    #[error("instruction {0} expects {1} operands but received {2}")]
    OperandMismatch(&'static str, usize, usize),
    #[error("label {0} is undefined")]
    UndefinedLabel(String),
    #[error("function {0} is undefined")]
    UndefinedFunction(String),
    #[error("duplicate label {0}")]
    DuplicateLabel(String),
    #[error("duplicate function {0}")]
    DuplicateFunction(String),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

impl Constant {
    fn encode(&self, bytes: &mut Vec<u8>) {
        match self {
            Constant::String(value) => {
                bytes.push(0);
                let value_bytes = value.as_bytes();
                bytes.extend_from_slice(&(value_bytes.len() as u32).to_le_bytes());
                bytes.extend_from_slice(value_bytes);
            }
            Constant::Integer(value) => {
                bytes.push(1);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            Constant::Float(value) => {
                bytes.push(2);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            Constant::Boolean(value) => {
                bytes.push(3);
                bytes.push(u8::from(*value));
            }
            Constant::Null => {
                bytes.push(4);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    LoadConst = 0,
    LoadVar = 1,
    StoreVar = 2,
    Add = 3,
    Sub = 4,
    Mul = 5,
    Div = 6,
    Call = 7,
    Return = 8,
    Jump = 9,
    JumpIfFalse = 10,
    CmpLt = 11,
    CmpEq = 12,
    Mod = 13,
    Neg = 14,
    Not = 15,
    CmpGt = 16,
    CmpLe = 17,
    CmpGe = 18,
    And = 19,
    Or = 20,
    Pop = 21,
    MakeList = 22,
    LoadLambda = 23,
    CallBuiltin = 24,
    CallAsync = 25,
    Await = 26,
    Nop = 27,
}

impl Opcode {
    pub fn name(self) -> &'static str {
        match self {
            Opcode::LoadConst => "LOAD_CONST",
            Opcode::LoadVar => "LOAD_VAR",
            Opcode::StoreVar => "STORE_VAR",
            Opcode::Add => "ADD",
            Opcode::Sub => "SUB",
            Opcode::Mul => "MUL",
            Opcode::Div => "DIV",
            Opcode::Mod => "MOD",
            Opcode::Neg => "NEG",
            Opcode::Not => "NOT",
            Opcode::Call => "CALL",
            Opcode::Return => "RETURN",
            Opcode::Jump => "JUMP",
            Opcode::JumpIfFalse => "JUMP_IF_FALSE",
            Opcode::CmpLt => "CMP_LT",
            Opcode::CmpEq => "CMP_EQ",
            Opcode::CmpGt => "CMP_GT",
            Opcode::CmpLe => "CMP_LE",
            Opcode::CmpGe => "CMP_GE",
            Opcode::And => "AND",
            Opcode::Or => "OR",
            Opcode::Pop => "POP",
            Opcode::MakeList => "MAKE_LIST",
            Opcode::LoadLambda => "LOAD_LAMBDA",
            Opcode::CallBuiltin => "CALL_BUILTIN",
            Opcode::CallAsync => "CALL_ASYNC",
            Opcode::Await => "AWAIT",
            Opcode::Nop => "NOP",
        }
    }

    pub fn operand_count(self) -> usize {
        match self {
            Opcode::LoadConst | Opcode::LoadVar | Opcode::StoreVar => 1,
            Opcode::Call => 2,
            Opcode::CallBuiltin => 2,
            Opcode::CallAsync => 2,
            Opcode::Jump | Opcode::JumpIfFalse => 1,
            Opcode::MakeList | Opcode::LoadLambda => 1,
            Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Neg
            | Opcode::Not
            | Opcode::Return
            | Opcode::CmpLt
            | Opcode::CmpEq
            | Opcode::CmpGt
            | Opcode::CmpLe
            | Opcode::CmpGe
            | Opcode::And
            | Opcode::Or
            | Opcode::Pop
            | Opcode::Await
            | Opcode::Nop => 0,
        }
    }
}

impl TryFrom<u8> for Opcode {
    type Error = SolvracError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let opcode = match value {
            0 => Opcode::LoadConst,
            1 => Opcode::LoadVar,
            2 => Opcode::StoreVar,
            3 => Opcode::Add,
            4 => Opcode::Sub,
            5 => Opcode::Mul,
            6 => Opcode::Div,
            7 => Opcode::Call,
            8 => Opcode::Return,
            9 => Opcode::Jump,
            10 => Opcode::JumpIfFalse,
            11 => Opcode::CmpLt,
            12 => Opcode::CmpEq,
            13 => Opcode::Mod,
            14 => Opcode::Neg,
            15 => Opcode::Not,
            16 => Opcode::CmpGt,
            17 => Opcode::CmpLe,
            18 => Opcode::CmpGe,
            19 => Opcode::And,
            20 => Opcode::Or,
            21 => Opcode::Pop,
            22 => Opcode::MakeList,
            23 => Opcode::LoadLambda,
            24 => Opcode::CallBuiltin,
            25 => Opcode::CallAsync,
            26 => Opcode::Await,
            27 => Opcode::Nop,
            other => return Err(SolvracError::UnknownOpcode(other)),
        };
        Ok(opcode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<u32>,
}

impl Instruction {
    pub fn new(opcode: Opcode, operands: Vec<u32>) -> Self {
        Self { opcode, operands }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: u16,
    pub instructions: Vec<Instruction>,
}

impl Function {
    pub fn new(name: impl Into<String>, parameters: u16, instructions: Vec<Instruction>) -> Self {
        Self {
            name: name.into(),
            parameters,
            instructions,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bytecode {
    pub version: u8,
    pub constants: Vec<Constant>,
    pub functions: Vec<Function>,
}

impl Bytecode {
    pub fn new(constants: Vec<Constant>, functions: Vec<Function>) -> Self {
        Self {
            version: VERSION,
            constants,
            functions,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, SolvracError> {
        if self.version != VERSION {
            return Err(SolvracError::UnsupportedVersion(self.version));
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(self.version);

        bytes.extend_from_slice(&(self.constants.len() as u32).to_le_bytes());
        for constant in &self.constants {
            constant.encode(&mut bytes);
        }

        bytes.extend_from_slice(&(self.functions.len() as u32).to_le_bytes());
        for function in &self.functions {
            let name_bytes = function.name.as_bytes();
            bytes.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(name_bytes);
            bytes.extend_from_slice(&function.parameters.to_le_bytes());
            bytes.extend_from_slice(&(function.instructions.len() as u32).to_le_bytes());
            for instruction in &function.instructions {
                bytes.push(instruction.opcode as u8);
                bytes.push(instruction.operands.len() as u8);
                for operand in &instruction.operands {
                    bytes.extend_from_slice(&operand.to_le_bytes());
                }
            }
        }

        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SolvracError> {
        let mut cursor = Cursor::new(bytes);
        let mut magic = [0u8; 4];
        cursor
            .read_exact(&mut magic)
            .map_err(|_| SolvracError::UnexpectedEof)?;
        if &magic != MAGIC {
            return Err(SolvracError::InvalidHeader);
        }

        let mut version = [0u8; 1];
        cursor
            .read_exact(&mut version)
            .map_err(|_| SolvracError::UnexpectedEof)?;
        let version = version[0];
        if version != VERSION {
            return Err(SolvracError::UnsupportedVersion(version));
        }

        let constants = read_constants(&mut cursor)?;
        let functions = read_functions(&mut cursor)?;

        Ok(Bytecode {
            version,
            constants,
            functions,
        })
    }
}

fn read_constants(cursor: &mut Cursor<&[u8]>) -> Result<Vec<Constant>, SolvracError> {
    let count = read_u32(cursor)?;
    let mut constants = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut tag = [0u8; 1];
        cursor
            .read_exact(&mut tag)
            .map_err(|_| SolvracError::UnexpectedEof)?;
        let constant = match tag[0] {
            0 => {
                let len = read_u32(cursor)? as usize;
                let mut data = vec![0u8; len];
                cursor
                    .read_exact(&mut data)
                    .map_err(|_| SolvracError::UnexpectedEof)?;
                let string = String::from_utf8(data).map_err(|_| SolvracError::InvalidUtf8)?;
                Constant::String(string)
            }
            1 => {
                let mut buf = [0u8; 8];
                cursor
                    .read_exact(&mut buf)
                    .map_err(|_| SolvracError::UnexpectedEof)?;
                Constant::Integer(i64::from_le_bytes(buf))
            }
            2 => {
                let mut buf = [0u8; 8];
                cursor
                    .read_exact(&mut buf)
                    .map_err(|_| SolvracError::UnexpectedEof)?;
                Constant::Float(f64::from_le_bytes(buf))
            }
            3 => {
                let mut buf = [0u8; 1];
                cursor
                    .read_exact(&mut buf)
                    .map_err(|_| SolvracError::UnexpectedEof)?;
                Constant::Boolean(buf[0] != 0)
            }
            4 => Constant::Null,
            other => {
                return Err(SolvracError::Message(format!(
                    "unknown constant tag {other}"
                )))
            }
        };
        constants.push(constant);
    }
    Ok(constants)
}

fn read_functions(cursor: &mut Cursor<&[u8]>) -> Result<Vec<Function>, SolvracError> {
    let count = read_u32(cursor)?;
    let mut functions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let name_len = read_u32(cursor)? as usize;
        let mut name_bytes = vec![0u8; name_len];
        cursor
            .read_exact(&mut name_bytes)
            .map_err(|_| SolvracError::UnexpectedEof)?;
        let name = String::from_utf8(name_bytes).map_err(|_| SolvracError::InvalidUtf8)?;

        let mut parameters_buf = [0u8; 2];
        cursor
            .read_exact(&mut parameters_buf)
            .map_err(|_| SolvracError::UnexpectedEof)?;
        let parameters = u16::from_le_bytes(parameters_buf);

        let instruction_count = read_u32(cursor)? as usize;
        let mut instructions = Vec::with_capacity(instruction_count);
        for _ in 0..instruction_count {
            let mut opcode = [0u8; 1];
            cursor
                .read_exact(&mut opcode)
                .map_err(|_| SolvracError::UnexpectedEof)?;
            let opcode = Opcode::try_from(opcode[0])?;
            let mut operand_count = [0u8; 1];
            cursor
                .read_exact(&mut operand_count)
                .map_err(|_| SolvracError::UnexpectedEof)?;
            let operand_count = operand_count[0] as usize;
            let expected = opcode.operand_count();
            if expected != operand_count {
                return Err(SolvracError::OperandMismatch(
                    opcode.name(),
                    expected,
                    operand_count,
                ));
            }
            let mut operands = Vec::with_capacity(operand_count);
            for _ in 0..operand_count {
                operands.push(read_u32(cursor)?);
            }
            instructions.push(Instruction::new(opcode, operands));
        }

        functions.push(Function::new(name, parameters, instructions));
    }
    Ok(functions)
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, SolvracError> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| SolvracError::UnexpectedEof)?;
    Ok(u32::from_le_bytes(buf))
}
