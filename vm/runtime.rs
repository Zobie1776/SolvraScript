use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use tokio::task::{JoinHandle, LocalSet};
use tokio::time::sleep;

use solvra_core::concurrency::executor::TaskExecutor;
use solvra_core::vm::bytecode::{VmBytecode, VmConstant};
use solvra_core::vm::instruction::{Instruction, Opcode};
use solvra_core::{SolvraError, SolvraResult, StackFrame, Value};

use super::async_control::AsyncControl;
use super::builtins::{BuiltinContext, Builtins};
use serde::Serialize;

/// Shared bytecode handle passed into the runtime.
pub type SolvraProgram = Arc<VmBytecode>;

/// Runtime flags controlling tracing and diagnostics.
#[derive(Clone)]
pub struct RuntimeOptions {
    pub trace: bool,
    pub async_timeout_ms: Option<u64>,
    pub memory_tracker: Option<MemoryTracker>,
    pub telemetry_hook: Option<TelemetryHook>,
    pub telemetry_collector: Option<TelemetryCollector>,
    pub executor: TaskExecutor,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            trace: false,
            async_timeout_ms: None,
            memory_tracker: None,
            telemetry_hook: None,
            telemetry_collector: None,
            executor: TaskExecutor::default(),
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

    /// Register a telemetry hook invoked on runtime events.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_telemetry_hook(mut self, hook: TelemetryHook) -> Self {
        self.telemetry_hook = Some(hook);
        self
    }

    /// Attach a TelemetryCollector and wire its hook automatically.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_telemetry_collector(mut self, collector: TelemetryCollector) -> Self {
        self.telemetry_collector = Some(collector.clone());
        self.telemetry_hook = Some(collector.hook());
        self
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_executor(mut self, executor: TaskExecutor) -> Self {
        self.executor = executor;
        self
    }
}

/// Telemetry callback signature for SolvraAI integration.
use super::metrics::{TelemetryCollector, TelemetryEvent, TelemetryEventKind, TelemetryHook};

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

    fn record_timeout(&self, stack_depth: usize) {
        if let Ok(mut stats) = self.inner.lock() {
            stats.timeouts += 1;
            stats.timeout_stack_samples.push(stack_depth);
            let constant_loads = stats.constant_loads;
            stats.timeout_constant_samples.push(constant_loads);
        }
    }

    fn record_scheduler_tick(&self, snapshots: Vec<TaskSnapshot>) {
        if let Ok(mut stats) = self.inner.lock() {
            stats.scheduler_ticks += 1;
            stats.last_tick_tasks = snapshots;
            if let Some(max_elapsed) = stats
                .last_tick_tasks
                .iter()
                .map(|snapshot| snapshot.elapsed_ms)
                .max()
            {
                stats.peak_task_elapsed_ms = stats.peak_task_elapsed_ms.max(max_elapsed);
            }
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

/// Snapshot captured during scheduler tick to support predictive scheduling.
#[allow(dead_code)]
#[derive(Clone, Debug, Default, Serialize)]
pub struct TaskSnapshot {
    pub label: String,
    pub elapsed_ms: u64,
}

/// Memory counters captured during VM execution.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MemoryStats {
    pub max_stack_depth: usize,
    pub last_stack_depth: usize,
    pub constant_loads: usize,
    pub unique_constants: HashSet<usize>,
    pub constant_hits: HashMap<usize, usize>,
    pub task_spawns: usize,
    pub timeouts: usize,
    pub timeout_stack_samples: Vec<usize>,
    pub timeout_constant_samples: Vec<usize>,
    pub scheduler_ticks: usize,
    pub last_tick_tasks: Vec<TaskSnapshot>,
    pub peak_task_elapsed_ms: u64,
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
            None,
            Vec::new(),
        )?;
        executor.run().await
    })
}

struct RuntimeContext {
    program: SolvraProgram,
    builtins: Arc<Builtins>,
    options: RuntimeOptions,
    async_control: AsyncControl,
}

impl RuntimeContext {
    fn new(program: SolvraProgram, options: RuntimeOptions) -> Self {
        let async_control = AsyncControl::new();
        let builtin_context = BuiltinContext {
            memory_tracker: options.memory_tracker.clone(),
            telemetry: options.telemetry_collector.clone(),
            async_control: Some(async_control.clone()),
        };
        Self {
            program,
            builtins: Arc::new(Builtins::with_context(builtin_context)),
            options,
            async_control,
        }
    }
}

struct AsyncTask {
    label: String,
    handle: JoinHandle<SolvraResult<Value>>,
    started_at: Instant,
    core_completion: Arc<AtomicBool>,
}

struct RuntimeExecutor {
    ctx: Arc<RuntimeContext>,
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    tasks: HashMap<u64, AsyncTask>,
    next_task_id: u64,
    task_label: Option<String>,
    task_started_at: Instant,
    telemetry: Option<Arc<dyn Fn(&TelemetryEvent) + Send + Sync>>,
    async_control: AsyncControl,
    executor_id: Option<u64>,
    lineage: Vec<String>,
}

impl RuntimeExecutor {
    fn new(
        ctx: Arc<RuntimeContext>,
        function_index: usize,
        args: Vec<Value>,
        label: Option<String>,
        executor_id: Option<u64>,
        lineage: Vec<String>,
    ) -> SolvraResult<Self> {
        let async_control = ctx.async_control.clone();
        let mut executor = Self {
            ctx,
            frames: Vec::new(),
            stack: Vec::new(),
            tasks: HashMap::new(),
            next_task_id: 0,
            task_label: label,
            task_started_at: Instant::now(),
            telemetry: None,
            async_control,
            executor_id,
            lineage,
        };
        if let Some(hook) = &executor.ctx.options.telemetry_hook {
            executor.telemetry = Some(Arc::clone(hook));
        }
        executor
            .call_function(function_index, args)
            .map_err(|err| executor.enrich_error(err))?;
        executor.emit_telemetry_event(
            TelemetryEventKind::TaskSpawn,
            executor.task_label.clone(),
            Some(0),
            executor.ctx.options.async_timeout_ms,
        );
        Ok(executor)
    }

    async fn run(&mut self) -> SolvraResult<Value> {
        self.record_stack_depth();
        self.record_scheduler_snapshot();
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
                        core_completion,
                    } = self.tasks.remove(&task_id).ok_or_else(|| {
                        self.runtime_exception(format!("await on unknown task {task_id}"))
                    })?;

                    if self.async_control.is_cancelled(task_id) {
                        self.async_control.complete(task_id);
                        handle.abort();
                        self.abort_all_tasks();
                        self.emit_telemetry_event(
                            TelemetryEventKind::TaskCancel,
                            Some(label.clone()),
                            Some(started_at.elapsed().as_millis() as u64),
                            self.ctx.options.async_timeout_ms,
                        );
                        core_completion.store(true, Ordering::SeqCst);
                        let error = self.cancellation_runtime_exception(&label);
                        self.clear_state();
                        return Err(error);
                    }

                    let join_result =
                        if let Some(deadline) = self.task_deadline(task_id, started_at) {
                            let now = Instant::now();
                            if now >= deadline {
                                self.async_control.complete(task_id);
                                self.abort_all_tasks();
                                self.emit_telemetry_event(
                                    TelemetryEventKind::TaskTimeout,
                                    Some(label.clone()),
                                    Some(now.duration_since(started_at).as_millis() as u64),
                                    self.ctx.options.async_timeout_ms,
                                );
                                self.record_timeout_event(self.stack.len());
                                let elapsed_ms = now.duration_since(started_at).as_millis() as u64;
                                let error = self.timeout_runtime_exception(&label, elapsed_ms);
                                self.clear_state();
                                return Err(error);
                            }
                            let duration = deadline.saturating_duration_since(now);
                            let handle_fut = handle;
                            tokio::pin!(handle_fut);
                            tokio::select! {
                                res = &mut handle_fut => res,
                                _ = sleep(duration) => {
                                    handle_fut.as_ref().get_ref().abort();
                                    self.async_control.complete(task_id);
                                    self.abort_all_tasks();
                                    self.emit_telemetry_event(
                                        TelemetryEventKind::TaskTimeout,
                                        Some(label.clone()),
                                        Some(started_at.elapsed().as_millis() as u64),
                                        self.ctx.options.async_timeout_ms,
                                    );
                                    self.record_timeout_event(self.stack.len());
                                    let elapsed_ms = started_at.elapsed().as_millis() as u64;
                                    core_completion.store(true, Ordering::SeqCst);
                                    let error = self.timeout_runtime_exception(&label, elapsed_ms);
                                    self.clear_state();
                                    return Err(error);
                                }
                            }
                        } else {
                            handle.await
                        };
                    self.async_control.complete(task_id);
                    core_completion.store(true, Ordering::SeqCst);
                    match join_result {
                        Ok(inner) => match inner {
                            Ok(value) => {
                                self.emit_telemetry_event(
                                    TelemetryEventKind::TaskJoin,
                                    Some(label.clone()),
                                    Some(started_at.elapsed().as_millis() as u64),
                                    self.ctx.options.async_timeout_ms,
                                );
                                self.stack.push(value);
                            }
                            Err(err) => {
                                self.emit_telemetry_event(
                                    TelemetryEventKind::TaskPanic,
                                    Some(label.clone()),
                                    Some(started_at.elapsed().as_millis() as u64),
                                    self.ctx.options.async_timeout_ms,
                                );
                                return Err(self.attach_stack(err));
                            }
                        },
                        Err(err) => {
                            self.emit_telemetry_event(
                                TelemetryEventKind::TaskPanic,
                                Some(label.clone()),
                                Some(started_at.elapsed().as_millis() as u64),
                                self.ctx.options.async_timeout_ms,
                            );
                            return Err(
                                self.runtime_exception(format!("async task {label} panic: {err}"))
                            );
                        }
                    }
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
            self.record_scheduler_snapshot();
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
        let started_at = Instant::now();
        let task_id = {
            let id = self.next_task_id;
            self.next_task_id += 1;
            id
        };
        let lineage = self.current_lineage();
        let async_control = self.async_control.clone();
        async_control.register(task_id);
        let completion_flag = Arc::new(AtomicBool::new(false));
        let completion_clone = completion_flag.clone();
        self.ctx.options.executor.spawn(move || {
            while !completion_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
        });

        let ctx = Arc::clone(&self.ctx);
        let label = format!("{}#{}", function_label, task_id);
        let control_clone = async_control.clone();
        let completion_for_task = completion_flag.clone();
        let async_label = function_label.clone();
        let handle = tokio::task::spawn_local(async move {
            let mut executor = RuntimeExecutor::new(
                ctx,
                function_index,
                args,
                Some(async_label.clone()),
                Some(task_id),
                lineage,
            )?;
            let result = executor
                .run()
                .await
                .map_err(|err| executor.enrich_error(err));
            control_clone.complete(task_id);
            completion_for_task.store(true, Ordering::SeqCst);
            result
        });
        self.tasks.insert(
            task_id,
            AsyncTask {
                label: label.clone(),
                handle,
                started_at,
                core_completion: completion_flag,
            },
        );
        self.emit_telemetry_event(
            TelemetryEventKind::TaskSpawn,
            Some(label),
            Some(0),
            self.ctx.options.async_timeout_ms,
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

    fn current_lineage(&self) -> Vec<String> {
        let mut lineage = self.lineage.clone();
        if let Some(label) = &self.task_label {
            lineage.push(label.clone());
        }
        lineage
    }

    fn lineage_string(&self, tail: &str) -> String {
        let mut lineage = self.current_lineage();
        lineage.push(tail.to_string());
        lineage.join(" -> ")
    }

    fn executor_deadline(&self) -> Option<Instant> {
        let mut deadlines = Vec::new();
        if let Some(ms) = self.ctx.options.async_timeout_ms {
            deadlines.push(self.task_started_at + Duration::from_millis(ms));
        }
        if let Some(id) = self.executor_id {
            if let Some(deadline) = self.async_control.deadline(id) {
                deadlines.push(deadline);
            }
        }
        deadlines.into_iter().min()
    }

    fn task_deadline(&self, task_id: u64, started_at: Instant) -> Option<Instant> {
        let mut deadlines = Vec::new();
        if let Some(ms) = self.ctx.options.async_timeout_ms {
            deadlines.push(started_at + Duration::from_millis(ms));
        }
        if let Some(deadline) = self.async_control.deadline(task_id) {
            deadlines.push(deadline);
        }
        deadlines.into_iter().min()
    }

    fn task_overview(&self) -> Vec<(u64, String, Instant)> {
        self.tasks
            .iter()
            .map(|(id, task)| (*id, task.label.clone(), task.started_at))
            .collect()
    }

    fn enforce_timeouts(&mut self) -> Option<SolvraError> {
        let now = Instant::now();

        if let Some(executor_id) = self.executor_id {
            if self.async_control.is_cancelled(executor_id) {
                self.abort_all_tasks();
                self.async_control.complete(executor_id);
                self.emit_telemetry_event(
                    TelemetryEventKind::TaskCancel,
                    self.task_label.clone(),
                    Some(
                        now.saturating_duration_since(self.task_started_at)
                            .as_millis() as u64,
                    ),
                    self.ctx.options.async_timeout_ms,
                );
                let label = self
                    .task_label
                    .clone()
                    .unwrap_or_else(|| "<task>".to_string());
                let error = self.cancellation_runtime_exception(&label);
                self.clear_state();
                return Some(error);
            }
        }

        if let Some(deadline) = self.executor_deadline() {
            if now >= deadline {
                let label = self
                    .task_label
                    .clone()
                    .unwrap_or_else(|| "<task>".to_string());
                self.abort_all_tasks();
                self.emit_telemetry_event(
                    TelemetryEventKind::TaskTimeout,
                    self.task_label.clone(),
                    Some(
                        now.saturating_duration_since(self.task_started_at)
                            .as_millis() as u64,
                    ),
                    self.ctx.options.async_timeout_ms,
                );
                self.record_timeout_event(self.stack.len());
                let elapsed_ms = now
                    .saturating_duration_since(self.task_started_at)
                    .as_millis() as u64;
                let error = self.timeout_runtime_exception(&label, elapsed_ms);
                self.clear_state();
                return Some(error);
            }
        }

        for (task_id, label, started_at) in self.task_overview() {
            if let Some(deadline) = self.task_deadline(task_id, started_at) {
                if now >= deadline {
                    self.abort_all_tasks();
                    self.emit_telemetry_event(
                        TelemetryEventKind::TaskTimeout,
                        Some(label.clone()),
                        Some(now.duration_since(started_at).as_millis() as u64),
                        self.ctx.options.async_timeout_ms,
                    );
                    self.record_timeout_event(self.stack.len());
                    let elapsed_ms = now.duration_since(started_at).as_millis() as u64;
                    let error = self.timeout_runtime_exception(&label, elapsed_ms);
                    self.clear_state();
                    return Some(error);
                }
            }
        }

        None
    }

    fn timeout_runtime_exception(&self, task_label: &str, elapsed_ms: u64) -> SolvraError {
        let lineage = self.lineage_string(task_label);
        SolvraError::RuntimeException {
            message: format!(
                "RuntimeException::Timeout {{ task: {task_label}, elapsed_ms: {elapsed_ms}, lineage: {lineage} }}"
            ),
            stack: self.capture_stack_trace(),
        }
    }

    fn cancellation_runtime_exception(&self, task_label: &str) -> SolvraError {
        let lineage = self.lineage_string(task_label);
        SolvraError::RuntimeException {
            message: format!(
                "RuntimeException::Cancelled {{ task: {task_label}, lineage: {lineage} }}"
            ),
            stack: self.capture_stack_trace(),
        }
    }

    fn abort_all_tasks(&mut self) {
        for (task_id, task) in self.tasks.drain() {
            task.handle.abort();
            self.async_control.complete(task_id);
            task.core_completion.store(true, Ordering::SeqCst);
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

    fn record_timeout_event(&self, stack_depth: usize) {
        if let Some(tracker) = &self.ctx.options.memory_tracker {
            tracker.record_timeout(stack_depth);
        }
    }

    fn emit_telemetry_event(
        &self,
        kind: TelemetryEventKind,
        task_label: Option<String>,
        elapsed_ms: Option<u64>,
        threshold_ms: Option<u64>,
    ) {
        if let Some(hook) = &self.telemetry {
            let event = TelemetryEvent {
                kind,
                task_label,
                elapsed_ms,
                timeout_threshold_ms: threshold_ms,
                stack_depth: self.stack.len(),
                timestamp: Instant::now(),
            };
            hook(&event);
        }
    }

    fn record_scheduler_snapshot(&self) {
        if let Some(tracker) = &self.ctx.options.memory_tracker {
            let mut snapshots = Vec::new();
            if let Some(label) = &self.task_label {
                snapshots.push(TaskSnapshot {
                    label: label.clone(),
                    elapsed_ms: self.task_started_at.elapsed().as_millis() as u64,
                });
            }
            for task in self.tasks.values() {
                snapshots.push(TaskSnapshot {
                    label: task.label.clone(),
                    elapsed_ms: task.started_at.elapsed().as_millis() as u64,
                });
            }
            tracker.record_scheduler_tick(snapshots);
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
