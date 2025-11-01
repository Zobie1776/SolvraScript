use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::task::{JoinHandle, LocalSet};
use tokio::time::sleep;

use solvra_core::vm::bytecode::{VmBytecode, VmConstant};
use solvra_core::vm::instruction::{Instruction, Opcode};
use solvra_core::{SolvraError, SolvraResult, StackFrame, Value};

use super::builtins::Builtins;

/// Shared bytecode handle passed into the runtime.
pub type SolvraProgram = Arc<VmBytecode>;

/// Runtime flags controlling tracing and diagnostics.
#[derive(Clone)]
pub struct RuntimeOptions {
    pub trace: bool,
    pub async_timeout_ms: Option<u64>,
    pub memory_tracker: Option<MemoryTracker>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            trace: false,
            async_timeout_ms: None,
            memory_tracker: None,
        }
    }
}

impl RuntimeOptions {
    pub fn with_trace(trace: bool) -> Self {
        Self {
            trace,
            ..Self::default()
        }
    }

    /// Configure an async/await timeout that aborts tasks exceeding `timeout`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_async_timeout(mut self, timeout_ms: u64) -> Self {
        self.async_timeout_ms = Some(timeout_ms);
        self
    }

    /// Attach a shared memory tracker used for introspection during tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_memory_tracker(mut self, tracker: MemoryTracker) -> Self {
        self.memory_tracker = Some(tracker);
        self
    }
}

/// Shared instrumentation collecting runtime allocation statistics.
#[derive(Clone, Default)]
pub struct MemoryTracker {
    inner: Arc<Mutex<MemoryStats>>,
}

impl MemoryTracker {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new() -> Self {
        Self::default()
    }

    fn record_stack(&self, depth: usize) {
        if let Ok(mut stats) = self.inner.lock() {
            stats.last_stack_depth = depth;
            stats.max_stack_depth = stats.max_stack_depth.max(depth);
        }
    }

    fn record_constant(&self, index: usize) {
        if let Ok(mut stats) = self.inner.lock() {
            stats.constant_loads += 1;
            stats.unique_constants.insert(index);
            *stats.constant_hits.entry(index).or_insert(0) += 1;
        }
    }

    fn record_task_spawn(&self) {
        if let Ok(mut stats) = self.inner.lock() {
            stats.task_spawns += 1;
        }
    }

    fn record_timeout(&self) {
        if let Ok(mut stats) = self.inner.lock() {
            stats.timeouts += 1;
        }
    }

    /// Return a point-in-time view of collected statistics.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn snapshot(&self) -> MemoryStats {
        self.inner
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }
}

/// Memory counters captured during VM execution.
#[derive(Clone, Debug, Default)]
pub struct MemoryStats {
    pub max_stack_depth: usize,
    pub last_stack_depth: usize,
    pub constant_loads: usize,
    pub unique_constants: HashSet<usize>,
    pub constant_hits: HashMap<usize, usize>,
    pub task_spawns: usize,
    pub timeouts: usize,
}

/// Execute a compiled SolvraScript program to completion.
///
/// @ZNOTE[Phase6 Complete]: Runtime loop upgraded for bytecode + async execution.
pub fn run_bytecode(program: SolvraProgram, options: RuntimeOptions) -> SolvraResult<Value> {
    let context = Arc::new(RuntimeContext::new(program, options));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| SolvraError::Internal(format!("tokio runtime init failed: {err}")))?;
    let local = LocalSet::new();
    local.block_on(&runtime, async move {
        let entry_label = context
            .program
            .functions
            .get(context.program.entry)
            .map(|func| func.name.clone());
        let mut executor = RuntimeExecutor::new(
            Arc::clone(&context),
            context.program.entry,
            Vec::new(),
            entry_label,
        )?;
        executor.run().await
    })
}

struct RuntimeContext {
    program: SolvraProgram,
    builtins: Arc<Builtins>,
    options: RuntimeOptions,
}

impl RuntimeContext {
    fn new(program: SolvraProgram, options: RuntimeOptions) -> Self {
        Self {
            program,
            builtins: Arc::new(Builtins::default()),
            options,
        }
    }
}

struct AsyncTask {
    label: String,
    handle: JoinHandle<SolvraResult<Value>>,
    started_at: Instant,
}

struct RuntimeExecutor {
    ctx: Arc<RuntimeContext>,
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    tasks: HashMap<u64, AsyncTask>,
    next_task_id: u64,
    task_label: Option<String>,
    task_started_at: Instant,
}

impl RuntimeExecutor {
    fn new(
        ctx: Arc<RuntimeContext>,
        function_index: usize,
        args: Vec<Value>,
        label: Option<String>,
    ) -> SolvraResult<Self> {
        let mut executor = Self {
            ctx,
            frames: Vec::new(),
            stack: Vec::new(),
            tasks: HashMap::new(),
            next_task_id: 0,
            task_label: label,
            task_started_at: Instant::now(),
        };
        executor
            .call_function(function_index, args)
            .map_err(|err| executor.enrich_error(err))?;
        Ok(executor)
    }

    async fn run(&mut self) -> SolvraResult<Value> {
        self.record_stack_depth();
        loop {
            if let Some(error) = self.enforce_timeouts() {
                return Err(error);
            }

            let frame_index = match self.frames.len().checked_sub(1) {
                Some(index) => index,
                None => return Ok(Value::Null),
            };

            let instruction = self
                .current_instruction(frame_index)
                .map_err(|err| self.enrich_error(err))?
                .clone();
            if self.ctx.options.trace {
                self.emit_trace(frame_index, &instruction);
            }

            let mut advance_ip = true;
            match instruction.opcode {
                Opcode::LoadConst => {
                    let value = self.load_constant(&instruction)?;
                    self.stack.push(value);
                }
                Opcode::LoadVar => {
                    let slot = instruction.operand_a as usize;
                    let value = self.frames[frame_index]
                        .locals
                        .get(slot)
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.stack.push(value);
                }
                Opcode::StoreVar => {
                    let slot = instruction.operand_a as usize;
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    if let Some(local) = self.frames[frame_index].locals.get_mut(slot) {
                        *local = value;
                    }
                }
                Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod => {
                    let rhs = self.stack.pop().unwrap_or(Value::Null);
                    let lhs = self.stack.pop().unwrap_or(Value::Null);
                    let value = execute_arithmetic(instruction.opcode, lhs, rhs)
                        .map_err(|err| self.enrich_error(err))?;
                    self.stack.push(value);
                }
                Opcode::Neg => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let negated = negate_value(value).map_err(|err| self.enrich_error(err))?;
                    self.stack.push(negated);
                }
                Opcode::Not => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Boolean(!value.is_truthy()));
                }
                Opcode::Pop => {
                    self.stack.pop();
                }
                Opcode::Jump => {
                    self.frames[frame_index].ip = instruction.operand_a as usize;
                    advance_ip = false;
                }
                Opcode::JumpIfFalse => {
                    let condition = self.stack.pop().unwrap_or(Value::Null);
                    if !condition.is_truthy() {
                        self.frames[frame_index].ip = instruction.operand_a as usize;
                        advance_ip = false;
                    }
                }
                Opcode::MakeList => {
                    let count = instruction.operand_a as usize;
                    let representation = build_list_string(&mut self.stack, count)
                        .map_err(|err| self.enrich_error(err))?;
                    self.stack.push(Value::String(representation));
                }
                Opcode::LoadLambda => {
                    let id = instruction.operand_a as i64;
                    self.stack.push(Value::Integer(id));
                }
                Opcode::Equal
                | Opcode::NotEqual
                | Opcode::Less
                | Opcode::LessEqual
                | Opcode::Greater
                | Opcode::GreaterEqual => {
                    let rhs = self.stack.pop().unwrap_or(Value::Null);
                    let lhs = self.stack.pop().unwrap_or(Value::Null);
                    let value = execute_comparison(instruction.opcode, lhs, rhs)
                        .map_err(|err| self.enrich_error(err))?;
                    self.stack.push(value);
                }
                Opcode::And | Opcode::Or => {
                    let rhs = self.stack.pop().unwrap_or(Value::Null);
                    let lhs = self.stack.pop().unwrap_or(Value::Null);
                    self.stack
                        .push(execute_logical(instruction.opcode, lhs, rhs));
                }
                Opcode::Call => {
                    let function_index = instruction.operand_a as usize;
                    let arg_count = instruction.operand_b as usize;
                    let args = self.collect_args(arg_count);
                    self.call_function(function_index, args)
                        .map_err(|err| self.enrich_error(err))?;
                    advance_ip = false;
                }
                Opcode::CallBuiltin => {
                    let name = self
                        .string_constant(instruction.operand_a as usize)
                        .ok_or_else(|| {
                            self.runtime_exception(format!(
                                "invalid builtin name constant {}",
                                instruction.operand_a
                            ))
                        })?;
                    let arg_count = instruction.operand_b as usize;
                    let args = self.collect_args(arg_count);
                    let result = self
                        .ctx
                        .builtins
                        .invoke_sync(&name, &args)
                        .map_err(|err| self.enrich_error(err))?;
                    self.stack.push(result);
                }
                Opcode::CallAsync => {
                    let function_index = instruction.operand_a as usize;
                    let arg_count = instruction.operand_b as usize;
                    let args = self.collect_args(arg_count);
                    let task_id = self
                        .spawn_async_function(function_index, args)
                        .map_err(|err| self.enrich_error(err))?;
                    self.stack.push(Value::Integer(task_id as i64));
                }
                Opcode::Await => {
                    let task_id_value = self.stack.pop().unwrap_or(Value::Null);
                    let task_id =
                        extract_task_id(task_id_value).map_err(|err| self.enrich_error(err))?;
                    let AsyncTask {
                        label,
                        handle,
                        started_at,
                    } = self.tasks.remove(&task_id).ok_or_else(|| {
                        self.runtime_exception(format!("await on unknown task {task_id}"))
                    })?;
                    let join_result = if let Some(limit_ms) = self.ctx.options.async_timeout_ms {
                        let duration = Duration::from_millis(limit_ms);
                        let handle_fut = handle;
                        tokio::pin!(handle_fut);
                        tokio::select! {
                            res = &mut handle_fut => res,
                            _ = sleep(duration) => {
                                handle_fut.as_ref().get_ref().abort();
                                self.abort_all_tasks();
                                self.record_timeout_event();
                                let elapsed_ms = started_at.elapsed().as_millis() as u64;
                                let error = self.timeout_runtime_exception(&label, elapsed_ms);
                                self.clear_state();
                                return Err(error);
                            }
                        }
                    } else {
                        handle.await
                    };
                    let joined = join_result.map_err(|err| {
                        self.runtime_exception(format!("async task {label} panic: {err}"))
                    })?;
                    let value = joined.map_err(|err| self.attach_stack(err))?;
                    self.stack.push(value);
                }
                Opcode::Return => {
                    let return_value = self.stack.pop().unwrap_or(Value::Null);
                    let frame = self.frames.pop().expect("frame must exist");
                    self.stack.truncate(frame.stack_base);
                    if self.frames.is_empty() {
                        self.record_stack_depth();
                        return Ok(return_value);
                    } else {
                        if let Some(parent) = self.frames.last_mut() {
                            parent.ip += 1;
                        }
                        self.stack.push(return_value);
                        continue;
                    }
                }
                Opcode::Nop => {}
            }

            if advance_ip && let Some(frame) = self.frames.last_mut() {
                frame.ip += 1;
            }
            self.record_stack_depth();
        }
    }

    fn current_instruction(&self, frame_index: usize) -> SolvraResult<&Instruction> {
        let frame = self.frames.get(frame_index).ok_or_else(|| {
            SolvraError::Internal(format!("frame index {frame_index} out of bounds"))
        })?;
        let function = self
            .ctx
            .program
            .functions
            .get(frame.function_index)
            .ok_or_else(|| {
                SolvraError::Internal(format!(
                    "function index {} out of bounds",
                    frame.function_index
                ))
            })?;
        function.instructions.get(frame.ip).ok_or_else(|| {
            SolvraError::Internal(format!("instruction pointer {} out of bounds", frame.ip))
        })
    }

    fn call_function(&mut self, function_index: usize, args: Vec<Value>) -> SolvraResult<()> {
        let function = self
            .ctx
            .program
            .functions
            .get(function_index)
            .cloned()
            .ok_or_else(|| {
                SolvraError::Internal(format!("invalid function index {function_index}"))
            })?;

        if args.len() != function.arity as usize {
            return Err(SolvraError::Internal(format!(
                "function '{}' expected {} args, received {}",
                function.name,
                function.arity,
                args.len()
            )));
        }

        let mut locals = vec![Value::Null; function.locals as usize];
        for (index, arg) in args.into_iter().enumerate() {
            if let Some(slot) = locals.get_mut(index) {
                *slot = arg;
            }
        }

        let frame = CallFrame {
            function_index,
            ip: 0,
            locals,
            stack_base: self.stack.len(),
        };
        self.frames.push(frame);
        Ok(())
    }

    fn collect_args(&mut self, count: usize) -> Vec<Value> {
        let mut args = Vec::with_capacity(count);
        for _ in 0..count {
            args.push(self.stack.pop().unwrap_or(Value::Null));
        }
        args.reverse();
        args
    }

    fn string_constant(&self, index: usize) -> Option<String> {
        self.ctx
            .program
            .constants
            .get(index)
            .and_then(|constant| match constant {
                VmConstant::String(value) => Some(value.clone()),
                _ => None,
            })
    }

    fn load_constant(&self, instruction: &Instruction) -> SolvraResult<Value> {
        let index = instruction.operand_a as usize;
        let constant = self
            .ctx
            .program
            .constants
            .get(index)
            .cloned()
            .ok_or_else(|| SolvraError::Internal(format!("constant index {index} out of range")))?;
        self.record_constant_load(index);
        Ok(vm_constant_to_value(constant))
    }

    fn emit_trace(&self, frame_index: usize, instruction: &Instruction) {
        let frame = &self.frames[frame_index];
        let function = &self.ctx.program.functions[frame.function_index];
        let opcode = instruction.opcode;
        let operands = match opcode {
            Opcode::Call | Opcode::CallAsync => {
                let target = instruction.operand_a as usize;
                let name = self
                    .ctx
                    .program
                    .functions
                    .get(target)
                    .map(|func| func.name.as_str())
                    .unwrap_or("<invalid>");
                format!("{name} ({} args)", instruction.operand_b)
            }
            Opcode::CallBuiltin => {
                let name = self
                    .string_constant(instruction.operand_a as usize)
                    .unwrap_or_else(|| format!("#{}", instruction.operand_a));
                format!("{name} ({} args)", instruction.operand_b)
            }
            Opcode::Jump
            | Opcode::JumpIfFalse
            | Opcode::LoadConst
            | Opcode::LoadVar
            | Opcode::StoreVar
            | Opcode::MakeList
            | Opcode::LoadLambda => {
                format!("{}", instruction.operand_a)
            }
            _ => String::new(),
        };
        let trace = format!(
            "[TRACE] [frame={}] {:04}: {} {}",
            function.name,
            frame.ip,
            opcode_name(opcode),
            operands
        );
        println!("{trace}");
    }

    fn spawn_async_function(
        &mut self,
        function_index: usize,
        args: Vec<Value>,
    ) -> SolvraResult<u64> {
        let function_label = self
            .ctx
            .program
            .functions
            .get(function_index)
            .map(|func| func.name.clone())
            .unwrap_or_else(|| format!("#{}", function_index));
        let ctx = Arc::clone(&self.ctx);
        let started_at = Instant::now();
        let task_id = {
            let id = self.next_task_id;
            self.next_task_id += 1;
            id
        };

        let async_label = function_label.clone();
        let handle = tokio::task::spawn_local(async move {
            let mut executor = RuntimeExecutor::new(ctx, function_index, args, Some(async_label))?;
            match executor.run().await {
                Ok(value) => Ok(value),
                Err(err) => Err(executor.enrich_error(err)),
            }
        });
        let label = format!("{}#{}", function_label, task_id);
        self.tasks.insert(
            task_id,
            AsyncTask {
                label,
                handle,
                started_at,
            },
        );
        self.record_task_spawn();
        Ok(task_id)
    }

    fn runtime_exception(&self, message: impl Into<String>) -> SolvraError {
        SolvraError::RuntimeException {
            message: message.into(),
            stack: self.capture_stack_trace(),
        }
    }

    fn enforce_timeouts(&mut self) -> Option<SolvraError> {
        let timeout_ms = self.ctx.options.async_timeout_ms?;
        let limit = Duration::from_millis(timeout_ms);
        let now = Instant::now();

        if now.duration_since(self.task_started_at) > limit {
            let label = self
                .task_label
                .clone()
                .unwrap_or_else(|| "<task>".to_string());
            self.abort_all_tasks();
            self.record_timeout_event();
            let elapsed_ms = now.duration_since(self.task_started_at).as_millis() as u64;
            let error = self.timeout_runtime_exception(&label, elapsed_ms);
            self.clear_state();
            return Some(error);
        }

        if !self.tasks.is_empty() {
            let mut timed_out_task: Option<(String, Instant)> = None;
            for task in self.tasks.values() {
                if now.duration_since(task.started_at) > limit {
                    timed_out_task = Some((task.label.clone(), task.started_at));
                    break;
                }
            }

            if let Some((label, started_at)) = timed_out_task {
                self.abort_all_tasks();
                self.record_timeout_event();
                let elapsed_ms = now.duration_since(started_at).as_millis() as u64;
                let error = self.timeout_runtime_exception(&label, elapsed_ms);
                self.clear_state();
                return Some(error);
            }
        }

        None
    }

    fn timeout_runtime_exception(&self, task_label: &str, elapsed_ms: u64) -> SolvraError {
        SolvraError::RuntimeException {
            message: format!(
                "RuntimeException::Timeout {{ task: {task_label}, elapsed_ms: {elapsed_ms} }}"
            ),
            stack: self.capture_stack_trace(),
        }
    }

    fn abort_all_tasks(&mut self) {
        for (_, task) in self.tasks.drain() {
            task.handle.abort();
        }
    }

    fn clear_state(&mut self) {
        self.frames.clear();
        self.stack.clear();
        self.tasks.clear();
        self.record_stack_depth();
    }

    fn capture_stack_trace(&self) -> Vec<StackFrame> {
        let mut trace = Vec::new();
        for frame in &self.frames {
            let function = self
                .ctx
                .program
                .functions
                .get(frame.function_index)
                .map(|func| func.name.clone())
                .unwrap_or_else(|| format!("#{}", frame.function_index));
            trace.push(StackFrame {
                function,
                location: None,
            });
        }
        if trace.is_empty() {
            let entry = self
                .ctx
                .program
                .functions
                .get(self.ctx.program.entry)
                .map(|func| func.name.clone())
                .unwrap_or_else(|| "<entry>".into());
            trace.push(StackFrame {
                function: entry,
                location: None,
            });
        }
        trace
    }

    fn enrich_error(&self, err: SolvraError) -> SolvraError {
        match err {
            SolvraError::Internal(message) => self.runtime_exception(message),
            other => other,
        }
    }

    fn attach_stack(&self, err: SolvraError) -> SolvraError {
        match err {
            SolvraError::RuntimeException { message, mut stack } => {
                let mut current = self.capture_stack_trace();
                if let (Some(last_existing), Some(first_new)) = (stack.last(), current.first()) {
                    if last_existing.function == first_new.function
                        && last_existing.location == first_new.location
                    {
                        // Skip the duplicate head frame when both stacks share the same current execution context.
                        current.remove(0);
                    }
                }
                if !current.is_empty() {
                    stack.extend(current);
                }
                SolvraError::RuntimeException { message, stack }
            }
            other => self.enrich_error(other),
        }
    }

    fn record_stack_depth(&self) {
        if let Some(tracker) = &self.ctx.options.memory_tracker {
            tracker.record_stack(self.stack.len());
        }
    }

    fn record_constant_load(&self, index: usize) {
        if let Some(tracker) = &self.ctx.options.memory_tracker {
            tracker.record_constant(index);
        }
    }

    fn record_task_spawn(&self) {
        if let Some(tracker) = &self.ctx.options.memory_tracker {
            tracker.record_task_spawn();
        }
    }

    fn record_timeout_event(&self) {
        if let Some(tracker) = &self.ctx.options.memory_tracker {
            tracker.record_timeout();
        }
    }
}

struct CallFrame {
    function_index: usize,
    ip: usize,
    locals: Vec<Value>,
    stack_base: usize,
}

fn extract_task_id(value: Value) -> SolvraResult<u64> {
    match value {
        Value::Integer(id) if id >= 0 => Ok(id as u64),
        other => Err(SolvraError::Internal(format!(
            "await expects task identifier, received {other:?}"
        ))),
    }
}

fn build_list_string(stack: &mut Vec<Value>, count: usize) -> SolvraResult<String> {
    if count > stack.len() {
        return Err(SolvraError::Internal("list construction underflow".into()));
    }
    let start = stack.len() - count;
    let values = stack.drain(start..).collect::<Vec<_>>();
    let mut parts = Vec::with_capacity(values.len());
    for value in values.iter() {
        parts.push(value_to_string(value));
    }
    Ok(format!("[{}]", parts.join(", ")))
}

fn execute_arithmetic(opcode: Opcode, lhs: Value, rhs: Value) -> SolvraResult<Value> {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => execute_integer_arithmetic(opcode, a, b),
        (Value::Float(a), Value::Float(b)) => execute_float_arithmetic(opcode, a, b),
        (Value::Integer(a), Value::Float(b)) => execute_float_arithmetic(opcode, a as f64, b),
        (Value::Float(a), Value::Integer(b)) => execute_float_arithmetic(opcode, a, b as f64),
        (Value::Null, Value::Integer(b)) if opcode == Opcode::Add => {
            execute_integer_arithmetic(opcode, 0, b)
        }
        (Value::Null, Value::Float(b)) if opcode == Opcode::Add => {
            execute_float_arithmetic(opcode, 0.0, b)
        }
        other => Err(SolvraError::Internal(format!(
            "unsupported operands for arithmetic: {other:?}"
        ))),
    }
}

fn execute_integer_arithmetic(opcode: Opcode, lhs: i64, rhs: i64) -> SolvraResult<Value> {
    use Opcode::*;
    let value = match opcode {
        Add => Value::Integer(lhs + rhs),
        Sub => Value::Integer(lhs - rhs),
        Mul => Value::Integer(lhs * rhs),
        Div => {
            if rhs == 0 {
                return Err(SolvraError::Internal("integer division by zero".into()));
            }
            Value::Integer(lhs / rhs)
        }
        Mod => {
            if rhs == 0 {
                return Err(SolvraError::Internal("integer modulo by zero".into()));
            }
            Value::Integer(lhs % rhs)
        }
        _ => {
            return Err(SolvraError::Internal(format!(
                "unsupported integer opcode {opcode:?}"
            )));
        }
    };
    Ok(value)
}

fn execute_float_arithmetic(opcode: Opcode, lhs: f64, rhs: f64) -> SolvraResult<Value> {
    use Opcode::*;
    let value = match opcode {
        Add => Value::Float(lhs + rhs),
        Sub => Value::Float(lhs - rhs),
        Mul => Value::Float(lhs * rhs),
        Div => {
            if rhs == 0.0 {
                return Err(SolvraError::Internal("float division by zero".into()));
            }
            Value::Float(lhs / rhs)
        }
        Mod => {
            if rhs == 0.0 {
                return Err(SolvraError::Internal("float modulo by zero".into()));
            }
            Value::Float(lhs % rhs)
        }
        _ => {
            return Err(SolvraError::Internal(format!(
                "unsupported float opcode {opcode:?}"
            )));
        }
    };
    Ok(value)
}

fn negate_value(value: Value) -> SolvraResult<Value> {
    match value {
        Value::Integer(int) => Ok(Value::Integer(-int)),
        Value::Float(float) => Ok(Value::Float(-float)),
        Value::Null => Ok(Value::Integer(0)),
        other => Err(SolvraError::Internal(format!(
            "cannot negate value {other:?}"
        ))),
    }
}

fn execute_comparison(opcode: Opcode, lhs: Value, rhs: Value) -> SolvraResult<Value> {
    use Opcode::*;
    let result = match opcode {
        Equal => Value::Boolean(lhs == rhs),
        NotEqual => Value::Boolean(lhs != rhs),
        Less | LessEqual | Greater | GreaterEqual => {
            let left = value_to_number(&lhs)?;
            let right = value_to_number(&rhs)?;
            let cmp = match opcode {
                Less => left < right,
                LessEqual => left <= right,
                Greater => left > right,
                GreaterEqual => left >= right,
                _ => unreachable!(),
            };
            Value::Boolean(cmp)
        }
        _ => {
            return Err(SolvraError::Internal(format!(
                "unsupported comparison opcode {opcode:?}"
            )));
        }
    };
    Ok(result)
}

fn execute_logical(opcode: Opcode, lhs: Value, rhs: Value) -> Value {
    match opcode {
        Opcode::And => Value::Boolean(lhs.is_truthy() && rhs.is_truthy()),
        Opcode::Or => Value::Boolean(lhs.is_truthy() || rhs.is_truthy()),
        _ => Value::Null,
    }
}

fn value_to_number(value: &Value) -> SolvraResult<f64> {
    match value {
        Value::Integer(int) => Ok(*int as f64),
        Value::Float(float) => Ok(*float),
        Value::Boolean(flag) => Ok(if *flag { 1.0 } else { 0.0 }),
        Value::Null => Ok(0.0),
        other => Err(SolvraError::Internal(format!(
            "{} cannot be converted to number",
            other.type_name()
        ))),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Integer(int) => int.to_string(),
        Value::Float(float) => float.to_string(),
        Value::String(text) => text.clone(),
        Value::Object(_) => "<object>".into(),
    }
}

fn opcode_name(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::LoadConst => "LoadConst",
        Opcode::LoadVar => "LoadVar",
        Opcode::StoreVar => "StoreVar",
        Opcode::Add => "Add",
        Opcode::Sub => "Sub",
        Opcode::Mul => "Mul",
        Opcode::Div => "Div",
        Opcode::Mod => "Mod",
        Opcode::Neg => "Neg",
        Opcode::Not => "Not",
        Opcode::Pop => "Pop",
        Opcode::Jump => "Jump",
        Opcode::JumpIfFalse => "JumpIfFalse",
        Opcode::MakeList => "MakeList",
        Opcode::LoadLambda => "LoadLambda",
        Opcode::Equal => "Equal",
        Opcode::NotEqual => "NotEqual",
        Opcode::Less => "Less",
        Opcode::LessEqual => "LessEqual",
        Opcode::Greater => "Greater",
        Opcode::GreaterEqual => "GreaterEqual",
        Opcode::And => "And",
        Opcode::Or => "Or",
        Opcode::Call => "Call",
        Opcode::CallBuiltin => "CallBuiltin",
        Opcode::CallAsync => "CallAsync",
        Opcode::Await => "Await",
        Opcode::Return => "Return",
        Opcode::Nop => "Nop",
    }
}

fn vm_constant_to_value(constant: VmConstant) -> Value {
    match constant {
        VmConstant::Null => Value::Null,
        VmConstant::Bool(flag) => Value::Boolean(flag),
        VmConstant::Int(value) => Value::Integer(value),
        VmConstant::Float(value) => Value::Float(value),
        VmConstant::String(value) => Value::String(value),
    }
}
