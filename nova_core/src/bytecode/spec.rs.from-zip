//! Definition of the NovaBytecode binary format.

use std::convert::TryFrom;

use thiserror::Error;

pub const MAGIC: &[u8; 4] = b"NVBC";
pub const VERSION: u16 = 2;

/// Opcodes understood by the NovaCore virtual machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    Halt = 0,
    LoadConst = 1,
    LoadLocal = 2,
    StoreLocal = 3,
    LoadGlobal = 4,
    StoreGlobal = 5,
    Jump = 6,
    JumpIfFalse = 7,
    JumpIfTrue = 8,
    Add = 9,
    Subtract = 10,
    Multiply = 11,
    Divide = 12,
    Modulo = 13,
    Negate = 14,
    Equals = 15,
    NotEquals = 16,
    Less = 17,
    LessEqual = 18,
    Greater = 19,
    GreaterEqual = 20,
    LogicalAnd = 21,
    LogicalOr = 22,
    LogicalNot = 23,
    Call = 24,
    CallNative = 25,
    Return = 26,
    Pop = 27,
    BuildList = 28,
    Index = 29,
    StoreIndex = 30,
    PushCatch = 31,
    PopCatch = 32,
    Throw = 33,
    DebugTrap = 34,
}

impl TryFrom<u8> for Opcode {
    type Error = NovaBytecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use Opcode::*;
        let opcode = match value {
            0 => Halt,
            1 => LoadConst,
            2 => LoadLocal,
            3 => StoreLocal,
            4 => LoadGlobal,
            5 => StoreGlobal,
            6 => Jump,
            7 => JumpIfFalse,
            8 => JumpIfTrue,
            9 => Add,
            10 => Subtract,
            11 => Multiply,
            12 => Divide,
            13 => Modulo,
            14 => Negate,
            15 => Equals,
            16 => NotEquals,
            17 => Less,
            18 => LessEqual,
            19 => Greater,
            20 => GreaterEqual,
            21 => LogicalAnd,
            22 => LogicalOr,
            23 => LogicalNot,
            24 => Call,
            25 => CallNative,
            26 => Return,
            27 => Pop,
            28 => BuildList,
            29 => Index,
            30 => StoreIndex,
            31 => PushCatch,
            32 => PopCatch,
            33 => Throw,
            34 => DebugTrap,
            other => return Err(NovaBytecodeError::UnknownOpcode(other)),
        };
        Ok(opcode)
    }
}

/// Single instruction stored in NovaBytecode.
#[derive(Debug, Clone, PartialEq)]
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

    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.opcode as u8);
        bytes.extend_from_slice(&self.operand_a.to_le_bytes());
        bytes.extend_from_slice(&self.operand_b.to_le_bytes());
        bytes.extend_from_slice(&self.debug.unwrap_or(u32::MAX).to_le_bytes());
    }
}

/// Constants embedded in the bytecode.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl Constant {
    fn encode(&self, bytes: &mut Vec<u8>) {
        match self {
            Constant::Null => bytes.push(0),
            Constant::Boolean(value) => {
                bytes.push(1);
                bytes.push(u8::from(*value));
            }
            Constant::Integer(value) => {
                bytes.push(2);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            Constant::Float(value) => {
                bytes.push(3);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            Constant::String(value) => {
                bytes.push(4);
                let bytes_value = value.as_bytes();
                let len = u32::try_from(bytes_value.len())
                    .expect("string constants longer than u32::MAX are unsupported");
                bytes.extend_from_slice(&len.to_le_bytes());
                bytes.extend_from_slice(bytes_value);
            }
        }
    }
}

/// Debug symbol associated with instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugSymbol {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Description of a function stored inside the bytecode file.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDescriptor {
    pub name: String,
    pub arity: u16,
    pub locals: u16,
    pub instructions: Vec<Instruction>,
}

impl FunctionDescriptor {
    pub fn new(
        name: impl Into<String>,
        arity: u16,
        locals: u16,
        instructions: Vec<Instruction>,
    ) -> Self {
        Self {
            name: name.into(),
            arity,
            locals,
            instructions,
        }
    }
}

/// In memory representation of NovaBytecode.
#[derive(Debug, Clone)]
pub struct NovaBytecode {
    constants: Vec<Constant>,
    functions: Vec<FunctionDescriptor>,
    debug_symbols: Vec<DebugSymbol>,
    entry: usize,
}

impl NovaBytecode {
    pub fn new(
        constants: Vec<Constant>,
        functions: Vec<FunctionDescriptor>,
        debug_symbols: Vec<DebugSymbol>,
        entry: usize,
    ) -> Self {
        Self {
            constants,
            functions,
            debug_symbols,
            entry,
        }
    }

    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    pub fn functions(&self) -> &[FunctionDescriptor] {
        &self.functions
    }

    pub fn debug_symbols(&self) -> &[DebugSymbol] {
        &self.debug_symbols
    }

    pub fn entry(&self) -> usize {
        self.entry
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&(self.entry as u32).to_le_bytes());

        bytes.extend_from_slice(&(self.constants.len() as u32).to_le_bytes());
        for constant in &self.constants {
            constant.encode(&mut bytes);
        }

        bytes.extend_from_slice(&(self.debug_symbols.len() as u32).to_le_bytes());
        for symbol in &self.debug_symbols {
            let name_bytes = symbol.file.as_bytes();
            bytes.extend_from_slice(&u32::try_from(name_bytes.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(name_bytes);
            bytes.extend_from_slice(&symbol.line.to_le_bytes());
            bytes.extend_from_slice(&symbol.column.to_le_bytes());
        }

        bytes.extend_from_slice(&(self.functions.len() as u32).to_le_bytes());
        for function in &self.functions {
            let name_bytes = function.name.as_bytes();
            bytes.extend_from_slice(&u32::try_from(name_bytes.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(name_bytes);
            bytes.extend_from_slice(&function.arity.to_le_bytes());
            bytes.extend_from_slice(&function.locals.to_le_bytes());
            bytes.extend_from_slice(&(function.instructions.len() as u32).to_le_bytes());
            for inst in &function.instructions {
                inst.encode(&mut bytes);
            }
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NovaBytecodeError> {
        if bytes.len() < 10 {
            return Err(NovaBytecodeError::UnexpectedEof);
        }
        if &bytes[0..4] != MAGIC {
            return Err(NovaBytecodeError::InvalidMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != VERSION {
            return Err(NovaBytecodeError::UnsupportedVersion(version));
        }
        let mut cursor = 6;
        let entry = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;

        let constants_len =
            u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        let mut constants = Vec::with_capacity(constants_len);
        for _ in 0..constants_len {
            if cursor >= bytes.len() {
                return Err(NovaBytecodeError::UnexpectedEof);
            }
            let tag = bytes[cursor];
            cursor += 1;
            let constant = match tag {
                0 => Constant::Null,
                1 => {
                    if cursor >= bytes.len() {
                        return Err(NovaBytecodeError::UnexpectedEof);
                    }
                    let flag = bytes[cursor];
                    cursor += 1;
                    Constant::Boolean(flag != 0)
                }
                2 => {
                    if cursor + 8 > bytes.len() {
                        return Err(NovaBytecodeError::UnexpectedEof);
                    }
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&bytes[cursor..cursor + 8]);
                    cursor += 8;
                    Constant::Integer(i64::from_le_bytes(buf))
                }
                3 => {
                    if cursor + 8 > bytes.len() {
                        return Err(NovaBytecodeError::UnexpectedEof);
                    }
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&bytes[cursor..cursor + 8]);
                    cursor += 8;
                    Constant::Float(f64::from_le_bytes(buf))
                }
                4 => {
                    if cursor + 4 > bytes.len() {
                        return Err(NovaBytecodeError::UnexpectedEof);
                    }
                    let mut len_buf = [0u8; 4];
                    len_buf.copy_from_slice(&bytes[cursor..cursor + 4]);
                    cursor += 4;
                    let len = u32::from_le_bytes(len_buf) as usize;
                    if cursor + len > bytes.len() {
                        return Err(NovaBytecodeError::UnexpectedEof);
                    }
                    let slice = &bytes[cursor..cursor + len];
                    cursor += len;
                    let text = String::from_utf8(slice.to_vec()).map_err(|err| {
                        NovaBytecodeError::InvalidUtf8(err.utf8_error().valid_up_to())
                    })?;
                    Constant::String(text)
                }
                other => return Err(NovaBytecodeError::UnknownConstantTag(other)),
            };
            constants.push(constant);
        }

        if cursor + 4 > bytes.len() {
            return Err(NovaBytecodeError::UnexpectedEof);
        }
        let debug_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        let mut debug_symbols = Vec::with_capacity(debug_len);
        for _ in 0..debug_len {
            if cursor + 4 > bytes.len() {
                return Err(NovaBytecodeError::UnexpectedEof);
            }
            let name_len =
                u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + name_len + 8 > bytes.len() {
                return Err(NovaBytecodeError::UnexpectedEof);
            }
            let name = String::from_utf8(bytes[cursor..cursor + name_len].to_vec())
                .map_err(|err| NovaBytecodeError::InvalidUtf8(err.utf8_error().valid_up_to()))?;
            cursor += name_len;
            let line = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            let column = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            debug_symbols.push(DebugSymbol {
                file: name,
                line,
                column,
            });
        }

        if cursor + 4 > bytes.len() {
            return Err(NovaBytecodeError::UnexpectedEof);
        }
        let functions_len =
            u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        let mut functions = Vec::with_capacity(functions_len);
        for _ in 0..functions_len {
            if cursor + 4 > bytes.len() {
                return Err(NovaBytecodeError::UnexpectedEof);
            }
            let name_len =
                u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + name_len + 2 + 2 + 4 > bytes.len() {
                return Err(NovaBytecodeError::UnexpectedEof);
            }
            let name = String::from_utf8(bytes[cursor..cursor + name_len].to_vec())
                .map_err(|err| NovaBytecodeError::InvalidUtf8(err.utf8_error().valid_up_to()))?;
            cursor += name_len;
            let arity = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;
            let locals = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;
            let inst_len =
                u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            let mut instructions = Vec::with_capacity(inst_len);
            for _ in 0..inst_len {
                if cursor + 13 > bytes.len() {
                    return Err(NovaBytecodeError::UnexpectedEof);
                }
                let opcode = Opcode::try_from(bytes[cursor])?;
                cursor += 1;
                let operand_a = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                cursor += 4;
                let operand_b = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                cursor += 4;
                let debug_idx = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                cursor += 4;
                let debug = if debug_idx == u32::MAX {
                    None
                } else if (debug_idx as usize) < debug_symbols.len() {
                    Some(debug_idx)
                } else {
                    return Err(NovaBytecodeError::InvalidDebugSymbol(debug_idx));
                };
                instructions.push(Instruction {
                    opcode,
                    operand_a,
                    operand_b,
                    debug,
                });
            }
            functions.push(FunctionDescriptor {
                name,
                arity,
                locals,
                instructions,
            });
        }

        Ok(NovaBytecode {
            constants,
            functions,
            debug_symbols,
            entry,
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NovaBytecodeError {
    #[error("unexpected end of bytecode")]
    UnexpectedEof,
    #[error("invalid magic header")]
    InvalidMagic,
    #[error("unsupported bytecode version {0}")]
    UnsupportedVersion(u16),
    #[error("unknown opcode {0}")]
    UnknownOpcode(u8),
    #[error("unknown constant tag {0}")]
    UnknownConstantTag(u8),
    #[error("invalid utf8 at byte {0}")]
    InvalidUtf8(usize),
    #[error("invalid debug symbol index {0}")]
    InvalidDebugSymbol(u32),
    #[error("assembly error: {0}")]
    Assembly(String),
}
