use crate::{NovaError, NovaResult, Value};

use super::spec::{Constant, Instruction, Opcode};

/// Virtual machine executing NovaBytecode instructions.
#[derive(Debug)]
pub struct Vm {
    cost_limit: u64,
    consumed_cost: u64,
    constants: Vec<Constant>,
}

impl Vm {
    pub fn new(cost_limit: u64, constants: Vec<Constant>) -> Self {
        Self {
            cost_limit,
            consumed_cost: 0,
            constants,
        }
    }

    pub fn execute(&mut self, instructions: &[Instruction]) -> NovaResult<Value> {
        let mut stack: Vec<Value> = Vec::new();
        for instruction in instructions {
            self.step()?;
            match instruction.opcode {
                Opcode::LoadConst => {
                    let Some(index) = instruction.operand else {
                        return Err(NovaError::Internal("load const missing operand".into()));
                    };
                    let constant = self
                        .constants
                        .get(index as usize)
                        .ok_or_else(|| NovaError::Internal("constant out of range".into()))?;
                    stack.push(Value::from_constant(constant));
                }
                Opcode::Add | Opcode::Subtract | Opcode::Multiply | Opcode::Divide => {
                    let rhs = self.pop_number(&mut stack)?;
                    let lhs = self.pop_number(&mut stack)?;
                    let result = match instruction.opcode {
                        Opcode::Add => lhs + rhs,
                        Opcode::Subtract => lhs - rhs,
                        Opcode::Multiply => lhs * rhs,
                        Opcode::Divide => {
                            if rhs == 0.0 {
                                return Err(NovaError::Internal("division by zero".into()));
                            }
                            lhs / rhs
                        }
                        _ => unreachable!(),
                    };
                    stack.push(Value::Float(result));
                }
                Opcode::Return => {
                    return stack.pop().ok_or(NovaError::StackUnderflow);
                }
                Opcode::Halt => break,
            }
        }
        stack.pop().ok_or(NovaError::StackUnderflow)
    }

    fn step(&mut self) -> NovaResult<()> {
        self.consumed_cost += 1;
        if self.consumed_cost > self.cost_limit {
            return Err(NovaError::CostLimitExceeded);
        }
        Ok(())
    }

    fn pop_number(&self, stack: &mut Vec<Value>) -> NovaResult<f64> {
        let value = stack.pop().ok_or(NovaError::StackUnderflow)?;
        match value {
            Value::Integer(i) => Ok(i as f64),
            Value::Float(f) => Ok(f),
            Value::Boolean(b) => Ok(if b { 1.0 } else { 0.0 }),
            Value::Null => Ok(0.0),
            Value::String(_) => Err(NovaError::Internal(
                "cannot use string in arithmetic".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::spec::assemble;
    use crate::bytecode::{ast::Ast, ast::BinaryOp, ast::Expr};

    #[test]
    fn executes_addition() {
        let ast = Ast::from_expr(Expr::binary(
            BinaryOp::Add,
            Expr::number(4.0),
            Expr::number(6.0),
        ));
        let bytecode = assemble(&ast).expect("assemble");
        let mut vm = Vm::new(100, bytecode.constants().to_vec());
        let result = vm.execute(bytecode.instructions()).expect("exec");
        assert_eq!(result, Value::Float(10.0));
    }
}
