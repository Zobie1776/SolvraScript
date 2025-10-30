use std::sync::Arc;

use crate::{SolvraError, SolvraResult, Value};

use super::bytecode::{VmBytecode, VmConstant, VmFunction};
use super::instruction::Opcode;

pub struct StackVm {
    bytecode: Arc<VmBytecode>,
    stack: Vec<Value>,
}

impl StackVm {
    pub fn new(bytecode: Arc<VmBytecode>) -> Self {
        Self {
            bytecode,
            stack: Vec::new(),
        }
    }

    pub fn execute(&mut self) -> SolvraResult<Value> {
        let entry_index = self.bytecode.entry;
        let entry = self
            .bytecode
            .functions
            .get(entry_index)
            .ok_or_else(|| SolvraError::Internal("invalid entry function index".into()))?;
        if entry.arity != 0 {
            return Err(SolvraError::Internal(format!(
                "entry function '{}' expects {} arguments",
                entry.name, entry.arity
            )));
        }
        self.call_function(entry_index, &[])
    }

    pub fn call_function(&mut self, function_index: usize, args: &[Value]) -> SolvraResult<Value> {
        let function = self
            .bytecode
            .functions
            .get(function_index)
            .cloned()
            .ok_or_else(|| SolvraError::Internal("invalid function index".into()))?;
        if args.len() != function.arity as usize {
            return Err(SolvraError::Internal(format!(
                "function '{}' expects {} arguments, received {}",
                function.name,
                function.arity,
                args.len()
            )));
        }
        self.run_function(function, args)
    }

    fn run_function(&mut self, function: VmFunction, args: &[Value]) -> SolvraResult<Value> {
        let mut locals = vec![Value::Null; function.locals as usize];
        for (index, arg) in args.iter().enumerate() {
            if let Some(slot) = locals.get_mut(index) {
                *slot = arg.clone();
            }
        }

        let mut ip = 0usize;
        while ip < function.instructions.len() {
            let instruction = &function.instructions[ip];
            match instruction.opcode {
                Opcode::Nop => {}
                Opcode::LoadConst => {
                    let value = self
                        .bytecode
                        .constants
                        .get(instruction.operand_a as usize)
                        .cloned()
                        .unwrap_or(VmConstant::Null)
                        .into();
                    self.stack.push(value);
                }
                Opcode::LoadVar => {
                    let value = locals
                        .get(instruction.operand_a as usize)
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.stack.push(value);
                }
                Opcode::StoreVar => {
                    if let Some(slot) = locals.get_mut(instruction.operand_a as usize) {
                        *slot = self.stack.pop().unwrap_or(Value::Null);
                    }
                }
                Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod => {
                    let rhs = self.stack.pop().unwrap_or(Value::Null);
                    let lhs = self.stack.pop().unwrap_or(Value::Null);
                    let value = execute_arithmetic(instruction.opcode, lhs, rhs)?;
                    self.stack.push(value);
                }
                Opcode::Neg => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let result = negate_value(value)?;
                    self.stack.push(result);
                }
                Opcode::Not => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let result = Value::Boolean(!value.is_truthy());
                    self.stack.push(result);
                }
                Opcode::Pop => {
                    self.stack.pop();
                }
                Opcode::Jump => {
                    ip = instruction.operand_a as usize;
                    continue;
                }
                Opcode::JumpIfFalse => {
                    let condition = self.stack.pop().unwrap_or(Value::Null);
                    if !condition.is_truthy() {
                        ip = instruction.operand_a as usize;
                        continue;
                    }
                }
                Opcode::MakeList => {
                    let count = instruction.operand_a as usize;
                    if count > self.stack.len() {
                        return Err(SolvraError::Internal(
                            "stack underflow while building list".into(),
                        ));
                    }
                    let start = self.stack.len() - count;
                    let values = self.stack.drain(start..).collect::<Vec<_>>();
                    let list_repr = build_list_string(&values);
                    self.stack.push(Value::String(list_repr));
                }
                Opcode::LoadLambda => {
                    self.stack
                        .push(Value::Integer(i64::from(instruction.operand_a)));
                }
                Opcode::Equal
                | Opcode::NotEqual
                | Opcode::Less
                | Opcode::LessEqual
                | Opcode::Greater
                | Opcode::GreaterEqual => {
                    let rhs = self.stack.pop().unwrap_or(Value::Null);
                    let lhs = self.stack.pop().unwrap_or(Value::Null);
                    let value = execute_comparison(instruction.opcode, lhs, rhs)?;
                    self.stack.push(value);
                }
                Opcode::And | Opcode::Or => {
                    let rhs = self.stack.pop().unwrap_or(Value::Null);
                    let lhs = self.stack.pop().unwrap_or(Value::Null);
                    let value = execute_logical(instruction.opcode, lhs, rhs);
                    self.stack.push(value);
                }
                Opcode::Call => {
                    let callee_index = instruction.operand_a as usize;
                    let arg_count = instruction.operand_b as usize;
                    let mut call_args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        call_args.push(self.stack.pop().unwrap_or(Value::Null));
                    }
                    call_args.reverse();
                    let result = self.call_function(callee_index, &call_args)?;
                    self.stack.push(result);
                }
                Opcode::CallBuiltin => {
                    let name_index = instruction.operand_a as usize;
                    let arg_count = instruction.operand_b as usize;
                    let mut call_args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        call_args.push(self.stack.pop().unwrap_or(Value::Null));
                    }
                    call_args.reverse();
                    let name = self
                        .bytecode
                        .constants
                        .get(name_index)
                        .and_then(|constant| match constant {
                            VmConstant::String(value) => Some(value.clone()),
                            _ => None,
                        })
                        .ok_or_else(|| {
                            SolvraError::Internal(format!(
                                "builtin call expects string constant at index {name_index}"
                            ))
                        })?;
                    let result = invoke_builtin(&name, &call_args)?;
                    self.stack.push(result);
                }
                Opcode::CallAsync => {
                    let callee_index = instruction.operand_a as usize;
                    let arg_count = instruction.operand_b as usize;
                    let mut call_args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        call_args.push(self.stack.pop().unwrap_or(Value::Null));
                    }
                    call_args.reverse();
                    let result = self.call_function(callee_index, &call_args)?;
                    self.stack.push(result);
                }
                Opcode::Await => {
                    // Placeholder async integration: value is already on stack.
                }
                Opcode::Return => {
                    return Ok(self.stack.pop().unwrap_or(Value::Null));
                }
            }
            ip += 1;
        }
        Ok(Value::Null)
    }
}

fn build_list_string(values: &[Value]) -> String {
    let mut parts = Vec::with_capacity(values.len());
    for value in values {
        parts.push(value_to_string(value));
    }
    format!("[{}]", parts.join(", "))
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Integer(int) => int.to_string(),
        Value::Float(float) => float.to_string(),
        Value::String(text) => text.clone(),
        Value::Object(_) => "<object>".to_string(),
    }
}

fn invoke_builtin(name: &str, args: &[Value]) -> SolvraResult<Value> {
    match name {
        "print" => {
            let payload = args.first().map(value_to_string).unwrap_or_default();
            print!("{payload}");
            Ok(Value::Null)
        }
        "println" => {
            let payload = args.first().map(value_to_string).unwrap_or_default();
            println!("{payload}");
            Ok(Value::Null)
        }
        other => Err(SolvraError::Internal(format!(
            "unknown builtin function '{other}'"
        ))),
    }
}

fn execute_arithmetic(opcode: Opcode, lhs: Value, rhs: Value) -> SolvraResult<Value> {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => execute_integer_arithmetic(opcode, a, b),
        (Value::Float(a), Value::Float(b)) => execute_float_arithmetic(opcode, a, b),
        (Value::Integer(a), Value::Float(b)) => execute_float_arithmetic(opcode, a as f64, b),
        (Value::Float(a), Value::Integer(b)) => execute_float_arithmetic(opcode, a, b as f64),
        other => Err(SolvraError::Internal(format!(
            "unsupported operands for arithmetic: {other:?}"
        ))),
    }
}

fn execute_integer_arithmetic(opcode: Opcode, lhs: i64, rhs: i64) -> SolvraResult<Value> {
    use Opcode::*;
    match opcode {
        Add => Ok(Value::Integer(lhs + rhs)),
        Sub => Ok(Value::Integer(lhs - rhs)),
        Mul => Ok(Value::Integer(lhs * rhs)),
        Div => {
            if rhs == 0 {
                Err(SolvraError::Internal("integer division by zero".into()))
            } else {
                Ok(Value::Integer(lhs / rhs))
            }
        }
        Mod => {
            if rhs == 0 {
                Err(SolvraError::Internal("integer modulo by zero".into()))
            } else {
                Ok(Value::Integer(lhs % rhs))
            }
        }
        _ => Err(SolvraError::Internal("unsupported integer opcode".into())),
    }
}

fn execute_float_arithmetic(opcode: Opcode, lhs: f64, rhs: f64) -> SolvraResult<Value> {
    use Opcode::*;
    match opcode {
        Add => Ok(Value::Float(lhs + rhs)),
        Sub => Ok(Value::Float(lhs - rhs)),
        Mul => Ok(Value::Float(lhs * rhs)),
        Div | Mod => {
            if rhs == 0.0 {
                Err(SolvraError::Internal("float division by zero".into()))
            } else if opcode == Div {
                Ok(Value::Float(lhs / rhs))
            } else {
                Ok(Value::Float(lhs % rhs))
            }
        }
        _ => Err(SolvraError::Internal("unsupported float opcode".into())),
    }
}

fn negate_value(value: Value) -> SolvraResult<Value> {
    match value {
        Value::Integer(v) => {
            let neg = v
                .checked_neg()
                .ok_or_else(|| SolvraError::Internal("integer overflow".into()))?;
            Ok(Value::Integer(neg))
        }
        Value::Float(v) => Ok(Value::Float(-v)),
        other => Err(SolvraError::Internal(format!(
            "negation not supported for value: {other:?}"
        ))),
    }
}

fn execute_comparison(opcode: Opcode, lhs: Value, rhs: Value) -> SolvraResult<Value> {
    use Opcode::*;
    match opcode {
        Equal => Ok(Value::Boolean(lhs == rhs)),
        NotEqual => Ok(Value::Boolean(lhs != rhs)),
        Less | LessEqual | Greater | GreaterEqual => {
            let lhs_num = value_to_number(&lhs)?;
            let rhs_num = value_to_number(&rhs)?;
            let result = match opcode {
                Less => lhs_num < rhs_num,
                LessEqual => lhs_num <= rhs_num,
                Greater => lhs_num > rhs_num,
                GreaterEqual => lhs_num >= rhs_num,
                _ => unreachable!(),
            };
            Ok(Value::Boolean(result))
        }
        _ => Err(SolvraError::Internal(
            "unsupported comparison opcode".into(),
        )),
    }
}

fn execute_logical(opcode: Opcode, lhs: Value, rhs: Value) -> Value {
    match opcode {
        Opcode::And => Value::Boolean(lhs.is_truthy() && rhs.is_truthy()),
        Opcode::Or => Value::Boolean(lhs.is_truthy() || rhs.is_truthy()),
        _ => Value::Null,
    }
}

fn value_to_number(value: &Value) -> SolvraResult<f64> {
    value.as_number().map_err(SolvraError::Internal)
}
