//=============================================
// nova_core/src/backend/lowering.rs
//=============================================
// Author: NovaCore Team
// License: MIT
// Goal: Lower Nova bytecode into SSA IR
// Objective: Provide translation from NovaBytecode to the IR module used by optimisation and codegen stages
//=============================================

use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};

use crate::backend::ir::{
    ConstantValue, FunctionBuilder, FunctionId, FunctionSignature, IrType, Module, Opcode, ValueId,
};
use crate::bytecode::spec::{Constant, Instruction};
use crate::bytecode::{FunctionDescriptor, NovaBytecode, Opcode as BytecodeOpcode};

//=============================================
// SECTION 1: Public API
//=============================================

/// Lower the provided bytecode into an SSA module ready for optimisation/codegen.
pub fn lower_bytecode(bytecode: &NovaBytecode) -> Result<Module> {
    let mut context = LoweringContext::new(bytecode);
    context.lower_module()?;
    Ok(context.module)
}

//=============================================
// SECTION 2: Lowering Context & Helpers
//=============================================

struct LoweringContext<'a> {
    bytecode: &'a NovaBytecode,
    module: Module,
    function_map: HashMap<usize, FunctionId>,
}

impl<'a> LoweringContext<'a> {
    fn new(bytecode: &'a NovaBytecode) -> Self {
        Self {
            bytecode,
            module: Module::new(),
            function_map: HashMap::new(),
        }
    }

    fn lower_module(&mut self) -> Result<()> {
        for (index, function) in self.bytecode.functions().iter().enumerate() {
            let signature =
                FunctionSignature::new(vec![IrType::I64; function.arity as usize], IrType::I64);
            let func_id = self.module.add_function(function.name.clone(), signature);
            self.function_map.insert(index, func_id);
            self.lower_function(func_id, function)?;
        }
        Ok(())
    }

    fn lower_function(
        &mut self,
        func_id: FunctionId,
        descriptor: &FunctionDescriptor,
    ) -> Result<()> {
        let mut builder = FunctionBuilder::new(&mut self.module, func_id);
        let entry = builder.append_block(Some("entry".into()));
        builder.position_at_end(entry);

        let mut stack: Vec<ValueId> = Vec::new();
        let constants = self.bytecode.constants();
        for instruction in &descriptor.instructions {
            lower_instruction(constants, instruction, &mut builder, &mut stack)?;
        }

        if builder.block(entry).terminator.is_none() {
            let value = stack.pop().unwrap_or_else(|| {
                builder.make_constant(
                    ConstantValue::I64(0),
                    IrType::I64,
                    Some("default_ret".into()),
                )
            });
            builder.emit_terminator(Opcode::Return, vec![value], None);
        }
        Ok(())
    }
}

fn lower_instruction(
    constants: &[Constant],
    instruction: &Instruction,
    builder: &mut FunctionBuilder<'_>,
    stack: &mut Vec<ValueId>,
) -> Result<()> {
    match instruction.opcode {
        BytecodeOpcode::LoadConst => {
            let index = instruction.operand_a as usize;
            let (constant, ty) = constant_to_ir(constants, index)?;
            let value = builder.make_constant(constant, ty, None);
            stack.push(value);
        }
        BytecodeOpcode::Add => {
            let rhs = stack.pop().ok_or_else(|| anyhow!("stack underflow"))?;
            let lhs = stack.pop().ok_or_else(|| anyhow!("stack underflow"))?;
            let result = builder.emit_value(Opcode::Add, vec![lhs, rhs], IrType::I64, None);
            stack.push(result);
        }
        BytecodeOpcode::Return => {
            let value = stack.pop().unwrap_or_else(|| {
                builder.make_constant(
                    ConstantValue::I64(0),
                    IrType::I64,
                    Some("default_ret".into()),
                )
            });
            builder.emit_terminator(Opcode::Return, vec![value], None);
        }
        other => bail!("lowering for opcode {:?} not implemented", other),
    }
    Ok(())
}

fn constant_to_ir(constants: &[Constant], index: usize) -> Result<(ConstantValue, IrType)> {
    let constant = constants
        .get(index)
        .ok_or_else(|| anyhow!("constant index {} out of range", index))?;
    let converted = match constant {
        Constant::Integer(value) => (ConstantValue::I64(*value), IrType::I64),
        Constant::Float(value) => (ConstantValue::F64(*value), IrType::F64),
        Constant::Boolean(value) => (ConstantValue::Bool(*value), IrType::Bool),
        Constant::Null => (ConstantValue::I64(0), IrType::I64),
        Constant::String(_) => bail!("string constants not yet supported in IR lowering"),
    };
    Ok(converted)
}

//=============================================
// SECTION 3: Tests
//=============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::spec::{Constant, Instruction};

    #[test]
    fn lowers_simple_addition() {
        let function = FunctionDescriptor::new(
            "add",
            0,
            0,
            vec![
                Instruction::new(BytecodeOpcode::LoadConst, 0, 0, None),
                Instruction::new(BytecodeOpcode::LoadConst, 1, 0, None),
                Instruction::new(BytecodeOpcode::Add, 0, 0, None),
                Instruction::new(BytecodeOpcode::Return, 0, 0, None),
            ],
        );
        let bytecode = NovaBytecode::new(
            vec![Constant::Integer(1), Constant::Integer(2)],
            vec![function],
            vec![],
            0,
        );
        let module = lower_bytecode(&bytecode).expect("lowering succeeded");
        assert_eq!(module.functions().count(), 1);
    }
}
