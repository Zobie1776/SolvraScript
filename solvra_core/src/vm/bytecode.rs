use super::instruction::Instruction;
use crate::Value;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("invalid bytecode magic")]
    InvalidMagic,
    #[error("unsupported bytecode version {0}")]
    UnsupportedVersion(u16),
    #[error("unexpected end of bytecode")]
    UnexpectedEof,
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for VmError {
    fn from(err: std::io::Error) -> Self {
        VmError::Io(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmBytecode {
    pub version: u16,
    pub constants: Vec<VmConstant>,
    pub functions: Vec<VmFunction>,
    pub entry: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VmConstant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmFunction {
    pub name: String,
    pub arity: u16,
    pub locals: u16,
    pub instructions: Vec<Instruction>,
}

impl VmBytecode {
    const MAGIC: &'static [u8; 4] = b"NOVM";
    const VERSION: u16 = 1;

    pub fn encode<W: Write>(&self, mut writer: W) -> Result<(), VmError> {
        writer.write_all(Self::MAGIC)?;
        writer.write_all(&Self::VERSION.to_le_bytes())?;
        let entry =
            u64::try_from(self.entry).map_err(|_| VmError::Io("entry index overflow".into()))?;
        writer.write_all(&entry.to_le_bytes())?;

        writer.write_all(&(self.constants.len() as u32).to_le_bytes())?;
        for constant in &self.constants {
            encode_constant(constant, &mut writer)?;
        }

        writer.write_all(&(self.functions.len() as u32).to_le_bytes())?;
        for function in &self.functions {
            let name_bytes = function.name.as_bytes();
            writer.write_all(&(name_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(name_bytes)?;
            writer.write_all(&function.arity.to_le_bytes())?;
            writer.write_all(&function.locals.to_le_bytes())?;
            writer.write_all(&(function.instructions.len() as u32).to_le_bytes())?;
            for inst in &function.instructions {
                let encoded =
                    bincode::serialize(inst).map_err(|err| VmError::Io(err.to_string()))?;
                writer.write_all(&(encoded.len() as u32).to_le_bytes())?;
                writer.write_all(&encoded)?;
            }
        }
        Ok(())
    }

    pub fn serialize(&self) -> Result<Vec<u8>, VmError> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }

    pub fn decode<R: Read>(mut reader: R) -> Result<Self, VmError> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != Self::MAGIC {
            return Err(VmError::InvalidMagic);
        }

        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != Self::VERSION {
            return Err(VmError::UnsupportedVersion(version));
        }

        let mut entry_bytes = [0u8; 8];
        reader.read_exact(&mut entry_bytes)?;
        let entry = u64::from_le_bytes(entry_bytes) as usize;

        let constants = read_vec(&mut reader, |r| decode_constant(r))?;

        let functions = read_vec(&mut reader, |r| {
            let name_len = read_u32(r)? as usize;
            let mut name_buf = vec![0u8; name_len];
            r.read_exact(&mut name_buf)?;
            let name = String::from_utf8(name_buf).map_err(|err| VmError::Io(err.to_string()))?;

            let mut arity_bytes = [0u8; 2];
            r.read_exact(&mut arity_bytes)?;
            let arity = u16::from_le_bytes(arity_bytes);

            let mut locals_bytes = [0u8; 2];
            r.read_exact(&mut locals_bytes)?;
            let locals = u16::from_le_bytes(locals_bytes);

            let instructions = read_vec(r, |r| {
                let inst_len = read_u32(r)? as usize;
                let mut buf = vec![0u8; inst_len];
                r.read_exact(&mut buf)?;
                bincode::deserialize(&buf).map_err(|err| VmError::Io(err.to_string()))
            })?;

            Ok(VmFunction {
                name,
                arity,
                locals,
                instructions,
            })
        })?;

        Ok(VmBytecode {
            version,
            constants,
            functions,
            entry,
        })
    }
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, VmError> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_vec<R: Read, T, F>(reader: &mut R, mut f: F) -> Result<Vec<T>, VmError>
where
    F: FnMut(&mut R) -> Result<T, VmError>,
{
    let len = read_u32(reader)? as usize;
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(f(reader)?);
    }
    Ok(values)
}

fn encode_constant<W: Write>(constant: &VmConstant, writer: &mut W) -> Result<(), VmError> {
    match constant {
        VmConstant::Null => writer.write_all(&[0])?,
        VmConstant::Bool(value) => {
            writer.write_all(&[1])?;
            writer.write_all(&[*value as u8])?;
        }
        VmConstant::Int(value) => {
            writer.write_all(&[2])?;
            writer.write_all(&value.to_le_bytes())?;
        }
        VmConstant::Float(value) => {
            writer.write_all(&[3])?;
            writer.write_all(&value.to_le_bytes())?;
        }
        VmConstant::String(value) => {
            writer.write_all(&[4])?;
            let bytes = value.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
    }
    Ok(())
}

fn decode_constant<R: Read>(reader: &mut R) -> Result<VmConstant, VmError> {
    let mut tag = [0u8; 1];
    reader.read_exact(&mut tag)?;
    match tag[0] {
        0 => Ok(VmConstant::Null),
        1 => {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf)?;
            Ok(VmConstant::Bool(buf[0] != 0))
        }
        2 => {
            let mut buf = [0u8; 8];
            reader.read_exact(&mut buf)?;
            Ok(VmConstant::Int(i64::from_le_bytes(buf)))
        }
        3 => {
            let mut buf = [0u8; 8];
            reader.read_exact(&mut buf)?;
            Ok(VmConstant::Float(f64::from_le_bytes(buf)))
        }
        4 => {
            let len = read_u32(reader)? as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;
            let string = String::from_utf8(buf).map_err(|err| VmError::Io(err.to_string()))?;
            Ok(VmConstant::String(string))
        }
        other => Err(VmError::Io(format!("unknown constant tag {other}"))),
    }
}

impl From<VmConstant> for Value {
    fn from(constant: VmConstant) -> Self {
        match constant {
            VmConstant::Null => Value::Null,
            VmConstant::Bool(b) => Value::Boolean(b),
            VmConstant::Int(i) => Value::Integer(i),
            VmConstant::Float(f) => Value::Float(f),
            VmConstant::String(s) => Value::String(s),
        }
    }
}
