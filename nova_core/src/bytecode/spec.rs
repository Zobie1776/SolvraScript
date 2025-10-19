use std::convert::TryFrom;

use super::ast::{Ast, BinaryOp, Expr};
use thiserror::Error;

pub const MAGIC: &[u8; 4] = b"NVBC";
pub const VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    Halt = 0,
    LoadConst = 1,
    Add = 2,
    Subtract = 3,
    Multiply = 4,
    Divide = 5,
    Return = 6,
}

impl TryFrom<u8> for Opcode {
    type Error = NovaBytecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Opcode::Halt),
            1 => Ok(Opcode::LoadConst),
            2 => Ok(Opcode::Add),
            3 => Ok(Opcode::Subtract),
            4 => Ok(Opcode::Multiply),
            5 => Ok(Opcode::Divide),
            6 => Ok(Opcode::Return),
            other => Err(NovaBytecodeError::UnknownOpcode(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Option<u32>,
}

impl Instruction {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.opcode as u8);
        match self.operand {
            Some(value) => bytes.extend_from_slice(&value.to_le_bytes()),
            None => bytes.extend_from_slice(&0u32.to_le_bytes()),
        }
    }
}

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
            Constant::Boolean(true) => {
                bytes.push(1);
                bytes.push(1);
            }
            Constant::Boolean(false) => {
                bytes.push(1);
                bytes.push(0);
            }
            Constant::Integer(i) => {
                bytes.push(2);
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            Constant::Float(f) => {
                bytes.push(3);
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            Constant::String(s) => {
                bytes.push(4);
                let bytes_value = s.as_bytes();
                let len = u32::try_from(bytes_value.len())
                    .expect("strings longer than u32::MAX not supported");
                bytes.extend_from_slice(&len.to_le_bytes());
                bytes.extend_from_slice(bytes_value);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct NovaBytecode {
    constants: Vec<Constant>,
    instructions: Vec<Instruction>,
}

impl NovaBytecode {
    pub fn new(constants: Vec<Constant>, instructions: Vec<Instruction>) -> Self {
        Self {
            constants,
            instructions,
        }
    }

    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());

        let const_len = u32::try_from(self.constants.len()).unwrap_or(0);
        bytes.extend_from_slice(&const_len.to_le_bytes());
        for constant in &self.constants {
            constant.encode(&mut bytes);
        }

        let inst_len = u32::try_from(self.instructions.len()).unwrap_or(0);
        bytes.extend_from_slice(&inst_len.to_le_bytes());
        for instruction in &self.instructions {
            instruction.encode(&mut bytes);
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NovaBytecodeError> {
        if bytes.len() < 6 {
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
        if bytes.len() < cursor + 4 {
            return Err(NovaBytecodeError::UnexpectedEof);
        }
        let constant_len =
            u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        let mut constants = Vec::with_capacity(constant_len);
        for _ in 0..constant_len {
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

        if bytes.len() < cursor + 4 {
            return Err(NovaBytecodeError::UnexpectedEof);
        }
        let inst_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        let mut instructions = Vec::with_capacity(inst_len);
        for _ in 0..inst_len {
            if cursor + 5 > bytes.len() {
                return Err(NovaBytecodeError::UnexpectedEof);
            }
            let opcode = Opcode::try_from(bytes[cursor])?;
            cursor += 1;
            let operand = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            let operand = match opcode {
                Opcode::LoadConst => Some(operand),
                _ => None,
            };
            instructions.push(Instruction { opcode, operand });
        }

        Ok(NovaBytecode::new(constants, instructions))
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
}

pub fn assemble(ast: &Ast) -> Result<NovaBytecode, NovaBytecodeError> {
    let mut builder = Assembler::default();
    builder.emit_ast(ast)?;
    Ok(builder.finish())
}

#[derive(Default)]
struct Assembler {
    constants: Vec<Constant>,
    instructions: Vec<Instruction>,
}

impl Assembler {
    fn finish(mut self) -> NovaBytecode {
        self.instructions.push(Instruction {
            opcode: Opcode::Return,
            operand: None,
        });
        self.instructions.push(Instruction {
            opcode: Opcode::Halt,
            operand: None,
        });
        NovaBytecode::new(self.constants, self.instructions)
    }

    fn emit_ast(&mut self, ast: &Ast) -> Result<(), NovaBytecodeError> {
        for expr in &ast.expressions {
            self.emit_expr(expr)?;
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<(), NovaBytecodeError> {
        match expr {
            Expr::Number(value) => {
                let idx = self.push_constant(Constant::Float(*value));
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadConst,
                    operand: Some(idx),
                });
            }
            Expr::Boolean(value) => {
                let idx = self.push_constant(Constant::Boolean(*value));
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadConst,
                    operand: Some(idx),
                });
            }
            Expr::String(value) => {
                let idx = self.push_constant(Constant::String(value.clone()));
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadConst,
                    operand: Some(idx),
                });
            }
            Expr::Binary { left, op, right } => {
                self.emit_expr(left)?;
                self.emit_expr(right)?;
                let opcode = match op {
                    BinaryOp::Add => Opcode::Add,
                    BinaryOp::Subtract => Opcode::Subtract,
                    BinaryOp::Multiply => Opcode::Multiply,
                    BinaryOp::Divide => Opcode::Divide,
                };
                self.instructions.push(Instruction {
                    opcode,
                    operand: None,
                });
            }
        }
        Ok(())
    }

    fn push_constant(&mut self, constant: Constant) -> u32 {
        if let Some(index) = self
            .constants
            .iter()
            .position(|existing| existing == &constant)
        {
            index as u32
        } else {
            let index = self.constants.len();
            self.constants.push(constant);
            index as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bytecode() {
        let ast = Ast::from_expr(Expr::binary(
            BinaryOp::Add,
            Expr::number(1.0),
            Expr::number(2.0),
        ));
        let bytecode = assemble(&ast).expect("assemble");
        let bytes = bytecode.clone().into_bytes();
        let decoded = NovaBytecode::from_bytes(&bytes).expect("decode");
        assert_eq!(decoded.constants(), bytecode.constants());
        assert_eq!(decoded.instructions(), bytecode.instructions());
    }
}
