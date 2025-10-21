use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use rand::Rng;

use crate::memory::gc::{Collector, GcObject};
use crate::module::ModuleLoader;
use crate::sys::{fs, net};
use crate::{NovaError, NovaResult, RuntimeConfig, StackFrame, Value};

use super::spec::{Constant, DebugSymbol, NovaBytecode, Opcode};

/// Virtual machine executing NovaBytecode instructions.
pub struct Vm {
    _config: RuntimeConfig,
    bytecode: Arc<NovaBytecode>,
    _modules: Arc<RwLock<ModuleLoader>>,
    cost_limit: u64,
    consumed_cost: u64,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    catch_stack: Vec<CatchFrame>,
    globals: HashMap<String, Value>,
    gc: Collector,
    builtins: NativeRegistry,
}

impl Vm {
    pub fn new(
        config: RuntimeConfig,
        bytecode: Arc<NovaBytecode>,
        modules: Arc<RwLock<ModuleLoader>>,
    ) -> Self {
        Self {
            cost_limit: config.cost_limit,
            consumed_cost: 0,
            _config: config,
            bytecode,
            _modules: modules,
            stack: Vec::new(),
            frames: Vec::new(),
            catch_stack: Vec::new(),
            globals: HashMap::new(),
            gc: Collector::new(),
            builtins: NativeRegistry::new(),
        }
    }

    pub fn execute(&mut self) -> NovaResult<Value> {
        self.call_function(self.bytecode.entry(), 0)?;
        self.run()
    }

    fn run(&mut self) -> NovaResult<Value> {
        loop {
            let frame_index = match self.frames.len() {
                0 => return Ok(Value::Null),
                len => len - 1,
            };
            let (function_index, ip) = {
                let frame = &mut self.frames[frame_index];
                let Some(function) = self.bytecode.functions().get(frame.function) else {
                    return Err(self.runtime_error(
                        "frame references unknown function",
                        frame_index,
                        None,
                    ));
                };
                if frame.ip >= function.instructions.len() {
                    return Err(self.runtime_error(
                        "instruction pointer out of bounds",
                        frame_index,
                        None,
                    ));
                }
                let ip = frame.ip;
                frame.ip += 1;
                (frame.function, ip)
            };
            let instruction = {
                let function = &self.bytecode.functions()[function_index];
                function.instructions[ip].clone()
            };
            self.step()?;
            let debug = instruction.debug;
            match instruction.opcode {
                Opcode::LoadConst => {
                    let index = instruction.operand_a as usize;
                    let constant = match self.bytecode.constants().get(index) {
                        Some(value) => value,
                        None => {
                            return Err(self.runtime_error(
                                "constant index out of range",
                                frame_index,
                                debug,
                            ))
                        }
                    };
                    self.stack.push(Value::from_constant(constant));
                }
                Opcode::LoadLocal => {
                    let index = instruction.operand_a as usize;
                    let base = self.frames[frame_index].base;
                    let value = match self.stack.get(base + index) {
                        Some(value) => value.clone(),
                        None => {
                            return Err(self.runtime_error(
                                "local index out of bounds",
                                frame_index,
                                debug,
                            ))
                        }
                    };
                    self.stack.push(value);
                }
                Opcode::StoreLocal => {
                    let index = instruction.operand_a as usize;
                    let base = self.frames[frame_index].base;
                    let value = self.pop()?;
                    let slot = match self.stack.get_mut(base + index) {
                        Some(slot) => slot,
                        None => {
                            return Err(self.runtime_error(
                                "local index out of bounds",
                                frame_index,
                                debug,
                            ))
                        }
                    };
                    *slot = value;
                }
                Opcode::LoadGlobal => {
                    let name = self.read_string_constant(instruction.operand_a as usize)?;
                    let value = self.globals.get(name).cloned().unwrap_or(Value::Null);
                    self.stack.push(value);
                }
                Opcode::StoreGlobal => {
                    let name = self
                        .read_string_constant(instruction.operand_a as usize)?
                        .to_string();
                    let value = self.pop()?;
                    self.globals.insert(name, value);
                }
                Opcode::Jump => {
                    self.frames[frame_index].ip = instruction.operand_a as usize;
                }
                Opcode::JumpIfFalse => {
                    let condition = self.pop()?;
                    if !condition.is_truthy() {
                        self.frames[frame_index].ip = instruction.operand_a as usize;
                    }
                }
                Opcode::JumpIfTrue => {
                    let condition = self.pop()?;
                    if condition.is_truthy() {
                        self.frames[frame_index].ip = instruction.operand_a as usize;
                    }
                }
                Opcode::Add
                | Opcode::Subtract
                | Opcode::Multiply
                | Opcode::Divide
                | Opcode::Modulo
                | Opcode::Equals
                | Opcode::NotEquals
                | Opcode::Less
                | Opcode::LessEqual
                | Opcode::Greater
                | Opcode::GreaterEqual => {
                    let rhs = self.pop()?;
                    let lhs = self.pop()?;
                    let value =
                        self.apply_binary(instruction.opcode, lhs, rhs, frame_index, debug)?;
                    self.stack.push(value);
                }
                Opcode::LogicalAnd | Opcode::LogicalOr => {
                    let rhs = self.pop()?;
                    let lhs = self.pop()?;
                    let result = match instruction.opcode {
                        Opcode::LogicalAnd => Value::Boolean(lhs.is_truthy() && rhs.is_truthy()),
                        Opcode::LogicalOr => Value::Boolean(lhs.is_truthy() || rhs.is_truthy()),
                        _ => unreachable!(),
                    };
                    self.stack.push(result);
                }
                Opcode::LogicalNot => {
                    let value = self.pop()?;
                    self.stack.push(Value::Boolean(!value.is_truthy()));
                }
                Opcode::Negate => {
                    let value = self.pop()?;
                    let number = value
                        .as_number()
                        .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
                    self.stack.push(Value::Float(-number));
                }
                Opcode::Call => {
                    let function_index = instruction.operand_a as usize;
                    let args = instruction.operand_b as usize;
                    self.call_function(function_index, args)?;
                }
                Opcode::CallNative => {
                    let native_index = instruction.operand_a as usize;
                    let args = instruction.operand_b as usize;
                    let result = self.invoke_native(native_index, args, frame_index, debug)?;
                    self.stack.push(result);
                }
                Opcode::Return => {
                    let value = self.pop().unwrap_or(Value::Null);
                    let frame = self.frames.pop().expect("frame must exist");
                    self.catch_stack
                        .retain(|catch| catch.frame_index < self.frames.len());
                    self.stack.truncate(frame.base);
                    if self.frames.is_empty() {
                        return Ok(value);
                    } else {
                        self.stack.push(value);
                    }
                }
                Opcode::Pop => {
                    self.pop()?;
                }
                Opcode::BuildList => {
                    let count = instruction.operand_a as usize;
                    if count > self.stack.len() {
                        return Err(self.runtime_error(
                            "list construction underflow",
                            frame_index,
                            debug,
                        ));
                    }
                    let start = self.stack.len() - count;
                    let mut values = self.stack.drain(start..).collect::<Vec<_>>();
                    values.reverse();
                    let value = self.alloc_list(values);
                    self.stack.push(value);
                }
                Opcode::Index => {
                    let index_value = self.pop()?;
                    let target = self.pop()?;
                    let value = self.index_value(target, index_value, frame_index, debug)?;
                    self.stack.push(value);
                }
                Opcode::StoreIndex => {
                    let store_value = self.pop()?;
                    let index_value = self.pop()?;
                    let target = self.pop()?;
                    self.store_index(target, index_value, store_value, frame_index, debug)?;
                }
                Opcode::PushCatch => {
                    self.catch_stack.push(CatchFrame {
                        frame_index: self.frames.len() - 1,
                        handler_ip: instruction.operand_a as usize,
                        stack_size: self.stack.len(),
                    });
                }
                Opcode::PopCatch => {
                    self.catch_stack.pop();
                }
                Opcode::Throw => {
                    let value = self.pop()?;
                    self.raise(value, debug)?;
                }
                Opcode::Halt => {
                    let value = self.pop().unwrap_or(Value::Null);
                    let frame = self.frames.pop().expect("frame exists");
                    self.stack.truncate(frame.base);
                    if self.frames.is_empty() {
                        return Ok(value);
                    } else {
                        self.stack.push(value);
                    }
                }
                Opcode::DebugTrap => {
                    return Err(self.runtime_error("debug trap", frame_index, debug));
                }
            }
        }
    }

    fn step(&mut self) -> NovaResult<()> {
        self.consumed_cost += 1;
        if self.consumed_cost > self.cost_limit {
            Err(NovaError::CostLimitExceeded)
        } else {
            if self.gc.allocated() > self.stack.len() + self.globals.len() + 32 {
                self.collect_garbage();
            }
            Ok(())
        }
    }

    fn pop(&mut self) -> NovaResult<Value> {
        self.stack.pop().ok_or(NovaError::StackUnderflow)
    }

    fn call_function(&mut self, function_index: usize, args: usize) -> NovaResult<()> {
        let function = self
            .bytecode
            .functions()
            .get(function_index)
            .ok_or_else(|| NovaError::Internal("call to unknown function".into()))?;
        if args != function.arity as usize {
            return Err(NovaError::RuntimeException {
                message: format!(
                    "function {} expected {} arguments got {}",
                    function.name, function.arity, args
                ),
                stack: self.build_stack_trace(None),
            });
        }
        if args > self.stack.len() {
            return Err(NovaError::StackUnderflow);
        }
        let base = self.stack.len() - args;
        let total_locals = function.locals as usize;
        if total_locals < args {
            return Err(NovaError::Internal(
                "function locals less than arity".into(),
            ));
        }
        self.stack.resize(base + total_locals, Value::Null);
        self.frames.push(Frame {
            function: function_index,
            ip: 0,
            base,
        });
        Ok(())
    }

    fn invoke_native(
        &mut self,
        native_index: usize,
        args: usize,
        frame_index: usize,
        debug: Option<u32>,
    ) -> NovaResult<Value> {
        if args > self.stack.len() {
            return Err(NovaError::StackUnderflow);
        }
        let start = self.stack.len() - args;
        let arguments = self.stack[start..].to_vec();
        self.stack.truncate(start);
        let registry = self.builtins.clone();
        match registry.call(native_index, self, &arguments) {
            Ok(value) => Ok(value),
            Err(message) => Err(self.runtime_error(message, frame_index, debug)),
        }
    }

    fn apply_binary(
        &self,
        opcode: Opcode,
        lhs: Value,
        rhs: Value,
        frame_index: usize,
        debug: Option<u32>,
    ) -> NovaResult<Value> {
        use Opcode::*;
        match opcode {
            Add => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::String(mut a), Value::String(b)) => {
                    a.push_str(&b);
                    Ok(Value::String(a))
                }
                (a, b) => {
                    let message = format!("cannot add {} and {}", a.type_name(), b.type_name());
                    Err(self.runtime_error(message, frame_index, debug))
                }
            },
            Subtract => self.numeric_op(lhs, rhs, |a, b| a - b, frame_index, debug),
            Multiply => self.numeric_op(lhs, rhs, |a, b| a * b, frame_index, debug),
            Divide => {
                let rhs_number = rhs
                    .as_number()
                    .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
                if rhs_number == 0.0 {
                    return Err(self.runtime_error("division by zero", frame_index, debug));
                }
                let lhs_number = lhs
                    .as_number()
                    .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
                Ok(Value::Float(lhs_number / rhs_number))
            }
            Modulo => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) if b != 0 => Ok(Value::Integer(a % b)),
                _ => Err(self.runtime_error("modulo requires integers", frame_index, debug)),
            },
            Equals => Ok(Value::Boolean(lhs == rhs)),
            NotEquals => Ok(Value::Boolean(lhs != rhs)),
            Less | LessEqual | Greater | GreaterEqual => {
                let lhs_number = lhs
                    .as_number()
                    .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
                let rhs_number = rhs
                    .as_number()
                    .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
                let result = match opcode {
                    Less => lhs_number < rhs_number,
                    LessEqual => lhs_number <= rhs_number,
                    Greater => lhs_number > rhs_number,
                    GreaterEqual => lhs_number >= rhs_number,
                    _ => unreachable!(),
                };
                Ok(Value::Boolean(result))
            }
            _ => Err(self.runtime_error("unsupported binary operator", frame_index, debug)),
        }
    }

    fn numeric_op<F>(
        &self,
        lhs: Value,
        rhs: Value,
        op: F,
        frame_index: usize,
        debug: Option<u32>,
    ) -> NovaResult<Value>
    where
        F: Fn(f64, f64) -> f64,
    {
        let lhs_number = lhs
            .as_number()
            .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
        let rhs_number = rhs
            .as_number()
            .map_err(|msg| self.runtime_error(msg, frame_index, debug))?;
        Ok(Value::Float(op(lhs_number, rhs_number)))
    }

    fn index_value(
        &self,
        target: Value,
        index: Value,
        frame_index: usize,
        debug: Option<u32>,
    ) -> NovaResult<Value> {
        match target {
            Value::String(string) => {
                let idx = index
                    .as_number()
                    .map_err(|msg| self.runtime_error(msg, frame_index, debug))?
                    as usize;
                let ch = string.chars().nth(idx).unwrap_or('\0');
                Ok(Value::String(ch.to_string()))
            }
            Value::Object(reference) => {
                let Some(object) = self.gc.get(reference) else {
                    return Err(self.runtime_error(
                        "dangling object reference",
                        frame_index,
                        debug,
                    ));
                };
                match object {
                    GcObject::List(items) => {
                        let idx = index
                            .as_number()
                            .map_err(|msg| self.runtime_error(msg, frame_index, debug))?
                            as usize;
                        let value = items.get(idx).cloned().unwrap_or(Value::Null);
                        Ok(value)
                    }
                    GcObject::Native(_) => {
                        Err(self.runtime_error("cannot index native object", frame_index, debug))
                    }
                }
            }
            other => Err(self.runtime_error(
                format!("{} is not indexable", other.type_name()),
                frame_index,
                debug,
            )),
        }
    }

    fn store_index(
        &mut self,
        target: Value,
        index: Value,
        value: Value,
        frame_index: usize,
        debug: Option<u32>,
    ) -> NovaResult<()> {
        match target {
            Value::Object(reference) => {
                let idx = index
                    .as_number()
                    .map_err(|msg| self.runtime_error(msg, frame_index, debug))?
                    as usize;
                let error = if let Some(object) = self.gc.get_mut(reference) {
                    match object {
                        GcObject::List(items) => {
                            if idx >= items.len() {
                                Some("list index out of bounds")
                            } else {
                                items[idx] = value;
                                return Ok(());
                            }
                        }
                        GcObject::Native(_) => Some("cannot mutate native object via index"),
                    }
                } else {
                    Some("dangling object reference")
                };
                Err(self.runtime_error(
                    error.expect("store_index error must be set"),
                    frame_index,
                    debug,
                ))
            }
            other => Err(self.runtime_error(
                format!("{} is not indexable", other.type_name()),
                frame_index,
                debug,
            )),
        }
    }

    fn alloc_list(&mut self, elements: Vec<Value>) -> Value {
        let reference = self.gc.allocate(GcObject::List(elements));
        Value::Object(reference)
    }

    fn collect_garbage(&mut self) {
        let mut roots = Vec::new();
        for value in &self.stack {
            if let Value::Object(reference) = value {
                roots.push(*reference);
            }
        }
        for value in self.globals.values() {
            if let Value::Object(reference) = value {
                roots.push(*reference);
            }
        }
        self.gc.collect(roots);
    }

    fn raise(&mut self, value: Value, debug: Option<u32>) -> NovaResult<()> {
        let thrown = value.clone();
        while let Some(catch) = self.catch_stack.pop() {
            if catch.frame_index >= self.frames.len() {
                continue;
            }
            while self.frames.len() - 1 > catch.frame_index {
                self.frames.pop();
            }
            self.stack.truncate(catch.stack_size);
            if let Some(frame) = self.frames.last_mut() {
                frame.ip = catch.handler_ip;
                self.stack.push(thrown);
                return Ok(());
            }
        }
        let message = self.format_value(&value);
        let stack = self.build_stack_trace(debug);
        Err(NovaError::RuntimeException { message, stack })
    }

    fn runtime_error(
        &self,
        message: impl Into<String>,
        frame_index: usize,
        debug: Option<u32>,
    ) -> NovaError {
        let debug = debug.or_else(|| self.frame_debug(frame_index));
        let mut stack = self.build_stack_trace(debug);
        if stack.is_empty() {
            let function = self
                .frames
                .get(frame_index)
                .and_then(|frame| self.bytecode.functions().get(frame.function))
                .map(|f| f.name.clone())
                .unwrap_or_else(|| "<unknown>".into());
            stack.push(StackFrame {
                function,
                location: debug.and_then(|idx| self.debug_symbol(idx).cloned()),
            });
        }
        NovaError::RuntimeException {
            message: message.into(),
            stack,
        }
    }

    fn frame_debug(&self, frame_index: usize) -> Option<u32> {
        let frame = self.frames.get(frame_index)?;
        let function = self.bytecode.functions().get(frame.function)?;
        if frame.ip == 0 {
            return None;
        }
        let instruction = function.instructions.get(frame.ip - 1)?;
        instruction.debug
    }

    fn build_stack_trace(&self, current_debug: Option<u32>) -> Vec<StackFrame> {
        let mut trace = Vec::new();
        for (i, frame) in self.frames.iter().enumerate() {
            let function = match self.bytecode.functions().get(frame.function) {
                Some(func) => func.name.clone(),
                None => "<unknown>".into(),
            };
            let debug = if i == self.frames.len() - 1 {
                current_debug.or_else(|| self.frame_debug(i))
            } else {
                self.frame_debug(i)
            };
            let location = debug.and_then(|idx| self.debug_symbol(idx).cloned());
            trace.push(StackFrame { function, location });
        }
        trace
    }

    fn debug_symbol(&self, index: u32) -> Option<&DebugSymbol> {
        self.bytecode.debug_symbols().get(index as usize)
    }

    fn read_string_constant(&self, index: usize) -> NovaResult<&str> {
        match self.bytecode.constants().get(index) {
            Some(Constant::String(text)) => Ok(text),
            _ => Err(NovaError::Internal("global name must be a string".into())),
        }
    }

    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".into(),
            Value::Boolean(boolean) => boolean.to_string(),
            Value::Integer(integer) => integer.to_string(),
            Value::Float(float) => float.to_string(),
            Value::String(text) => text.clone(),
            Value::Object(reference) => {
                if let Some(object) = self.gc.get(*reference) {
                    match object {
                        GcObject::List(items) => {
                            let parts: Vec<_> =
                                items.iter().map(|v| self.format_value(v)).collect();
                            format!("[{}]", parts.join(", "))
                        }
                        GcObject::Native(_) => "<native>".into(),
                    }
                } else {
                    "<collected>".into()
                }
            }
        }
    }
}

#[derive(Debug)]
struct Frame {
    function: usize,
    ip: usize,
    base: usize,
}

#[derive(Debug)]
struct CatchFrame {
    frame_index: usize,
    handler_ip: usize,
    stack_size: usize,
}

type NativeHandler = Arc<dyn Fn(&mut Vm, &[Value]) -> Result<Value, String> + Send + Sync>;

#[derive(Clone)]
struct NativeFunction {
    name: &'static str,
    handler: NativeHandler,
    arity: usize,
}

#[derive(Clone)]
struct NativeRegistry {
    functions: Vec<NativeFunction>,
}

impl NativeRegistry {
    fn new() -> Self {
        let mut registry = Self {
            functions: Vec::new(),
        };
        registry.register("print", 1, |vm, args| {
            print!("{}", vm.format_value(&args[0]));
            Ok(Value::Null)
        });
        registry.register("println", 1, |vm, args| {
            println!("{}", vm.format_value(&args[0]));
            Ok(Value::Null)
        });
        registry.register("read_file", 1, |_, args| {
            let path = expect_string(&args[0])?;
            let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
            Ok(Value::String(contents))
        });
        registry.register("write_file", 2, |_, args| {
            let path = expect_string(&args[0])?;
            let contents = expect_string(&args[1])?;
            fs::write(path, contents).map_err(|err| err.to_string())?;
            Ok(Value::Null)
        });
        registry.register("read_bytes", 1, |vm, args| {
            let path = expect_string(&args[0])?;
            let bytes = fs::read(path).map_err(|err| err.to_string())?;
            let values = bytes
                .into_iter()
                .map(|byte| Value::Integer(byte as i64))
                .collect::<Vec<_>>();
            Ok(vm.alloc_list(values))
        });
        registry.register("time_now", 0, |_, _| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|err| err.to_string())?;
            Ok(Value::Float(now.as_secs_f64()))
        });
        registry.register("time_sleep", 1, |_, args| {
            let duration_ms = args[0].as_number().map_err(|msg| msg.to_string())?;
            thread::sleep(Duration::from_millis(duration_ms as u64));
            Ok(Value::Null)
        });
        registry.register("rand_int", 1, |_, args| {
            let max = args[0].as_number().map_err(|msg| msg.to_string())? as i64;
            let mut rng = rand::thread_rng();
            Ok(Value::Integer(rng.gen_range(0..=max)))
        });
        registry.register("rand_float", 0, |_, _| {
            let mut rng = rand::thread_rng();
            Ok(Value::Float(rng.gen::<f64>()))
        });
        registry.register("net_udp_bind", 1, |vm, args| {
            let addr = expect_string(&args[0])?;
            let socket = net::UdpSocket::bind(addr).map_err(|err| err.to_string())?;
            let reference = vm.gc.allocate(GcObject::Native(Box::new(socket)));
            Ok(Value::Object(reference))
        });
        registry.register("net_udp_send", 3, |vm, args| {
            let socket_ref = match &args[0] {
                Value::Object(reference) => *reference,
                other => return Err(format!("expected socket object got {}", other.type_name())),
            };
            let addr = expect_string(&args[1])?;
            let message = expect_string(&args[2])?;
            let Some(GcObject::Native(native)) = vm.gc.get_mut(socket_ref) else {
                return Err("invalid socket handle".into());
            };
            let socket = native
                .downcast_mut::<net::UdpSocket>()
                .ok_or_else(|| "invalid socket type".to_string())?;
            socket
                .send_to(message.as_bytes(), addr)
                .map_err(|err| err.to_string())?;
            Ok(Value::Null)
        });
        registry
    }

    fn register<F>(&mut self, name: &'static str, arity: usize, handler: F)
    where
        F: Fn(&mut Vm, &[Value]) -> Result<Value, String> + Send + Sync + 'static,
    {
        self.functions.push(NativeFunction {
            name,
            handler: Arc::new(handler),
            arity,
        });
    }

    fn call(&self, index: usize, vm: &mut Vm, args: &[Value]) -> Result<Value, String> {
        let function = self
            .functions
            .get(index)
            .cloned()
            .ok_or_else(|| "unknown native function".to_string())?;
        if args.len() != function.arity {
            return Err(format!(
                "native {} expected {} arguments got {}",
                function.name,
                function.arity,
                args.len()
            ));
        }
        (function.handler)(vm, args)
    }
}

fn expect_string(value: &Value) -> Result<&str, String> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(format!("expected string got {}", other.type_name())),
    }
}
