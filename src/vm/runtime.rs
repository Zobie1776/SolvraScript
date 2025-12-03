#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use tokio::task::{JoinHandle, LocalSet};
use tokio::time::sleep;

use crate::ir::interpreter::RuntimeValue;
use crate::ir::ir::SolvraIrModule;
use solvra_core::concurrency::executor::{TaskExecutor, TaskHandle};
use solvra_core::jit::dispatcher::{DeoptEvent, JitDispatcher};
use solvra_core::jit::execute_tier0::execute_tier0;
use solvra_core::jit::tier0_codegen::{Tier0Artifact, Tier0FunctionId};
use solvra_core::jit::tier1_mir::{MirFunctionId, MirModule};
use solvra_core::jit::tier1_osr::Tier1OsrRegistry;
use solvra_core::memory::deterministic::{ArenaAllocator, Handle, HeapObject};
use solvra_core::vm::bytecode::{VmBytecode, VmConstant};
use solvra_core::vm::instruction::{Instruction, Opcode};
use solvra_core::{SolvraError, SolvraResult, StackFrame, Value};

use super::async_control::AsyncControl;
use super::builtins::{BuiltinContext, Builtins};
use super::core_builtins::{core_stub_message, is_core_stub_call};
use super::profiling::RuntimeProfile;
use serde::Serialize;

/// Shared bytecode handle passed into the runtime.
pub type SolvraProgram = Arc<VmBytecode>;

const DYNAMIC_CALL_TARGET: u32 = u32::MAX;
type ObjectHandle = Handle<HeapObject>;

/// Runtime flags controlling tracing and diagnostics.
#[derive(Clone)]
pub struct RuntimeOptions {
    pub trace: bool,
    pub async_timeout_ms: Option<u64>,
    pub memory_tracker: Option<MemoryTracker>,
    pub telemetry_hook: Option<TelemetryHook>,
    pub telemetry_collector: Option<TelemetryCollector>,
    pub executor: TaskExecutor,
    pub jit_tier0: bool,
    pub jit_tier1: bool,
    pub jit_stats: bool,
    pub jit_ir_module: Option<Arc<SolvraIrModule>>,
    pub tier1_mir_module: Option<Arc<MirModule>>,
    pub tier1_osr_registry: Option<Arc<Tier1OsrRegistry>>,
    pub jit_deopt_debug: bool,
    pub jit_tier1_fused_debug: bool,
    pub jit_osr_debug: bool,
    pub jit_transfer_debug: bool,
    pub jit_osr_validate: bool,
    pub jit_tier2: bool,
    pub jit_osr_tier2_debug: bool,
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
            jit_tier0: false,
            jit_tier1: false,
            jit_stats: false,
            jit_ir_module: None,
            tier1_mir_module: None,
            tier1_osr_registry: None,
            jit_deopt_debug: false,
            jit_tier1_fused_debug: false,
            jit_osr_debug: false,
            jit_transfer_debug: false,
            jit_osr_validate: false,
            jit_tier2: false,
            jit_osr_tier2_debug: false,
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
        let result = executor.run().await;
        if executor.ctx.options.jit_deopt_debug {
            if let Some(events) = executor.drain_deopt_events() {
                for event in &events {
                    println!(
                        "[tier1][deopt] func_id={} site={} pc={}",
                        event.function_id, event.deopt_site.0, event.snapshot.pc
                    );
                }
            }
        }
        result
    })
}

struct RuntimeContext {
    program: SolvraProgram,
    builtins: Arc<Builtins>,
    options: RuntimeOptions,
    async_control: AsyncControl,
    arena: Arc<Mutex<ArenaAllocator>>,
    jit_dispatcher: Option<Mutex<JitDispatcher>>,
}

impl RuntimeContext {
    fn new(program: SolvraProgram, options: RuntimeOptions) -> Self {
        let async_control = AsyncControl::new();
        let builtin_context = BuiltinContext {
            memory_tracker: options.memory_tracker.clone(),
            telemetry: options.telemetry_collector.clone(),
            async_control: Some(async_control.clone()),
        };
        let jit_dispatcher = if options.jit_tier0 || options.jit_tier1 || options.jit_stats {
            Some(Mutex::new(JitDispatcher::new()))
        } else {
            None
        };
        Self {
            program,
            builtins: Arc::new(Builtins::with_context(builtin_context)),
            options,
            async_control,
            arena: Arc::new(Mutex::new(ArenaAllocator::new())),
            jit_dispatcher,
        }
    }
}

struct AsyncTask {
    label: String,
    handle: JoinHandle<SolvraResult<Value>>,
    started_at: Instant,
    core_completion: Arc<AtomicBool>,
    core_task: TaskHandle,
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
    profile: RuntimeProfile,
    mir_function_map: HashMap<MirFunctionId, usize>,
    pending_deopt_frame: bool,
    pending_deopt_events: Vec<DeoptEvent>,
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
            profile: RuntimeProfile::new(),
            mir_function_map: HashMap::new(),
            pending_deopt_frame: false,
            pending_deopt_events: Vec::new(),
        };
        if let Some(hook) = &executor.ctx.options.telemetry_hook {
            executor.telemetry = Some(Arc::clone(hook));
        }
        executor.initialize_mir_function_map();
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
        self.profile.begin();
        let result = self.main_loop().await;
        self.finish_profile();
        result
    }

    fn initialize_mir_function_map(&mut self) {
        self.mir_function_map.clear();
        let Some(module_arc) = self.ctx.options.tier1_mir_module.as_ref() else {
            return;
        };
        let module = module_arc.as_ref();
        let mut by_name = HashMap::new();
        for (index, function) in self.ctx.program.functions.iter().enumerate() {
            by_name.insert(function.name.clone(), index);
        }
        for function in module.functions() {
            if let Some(index) = by_name.get(&function.name) {
                self.mir_function_map.insert(function.id, *index);
            }
        }
    }

    async fn main_loop(&mut self) -> SolvraResult<Value> {
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
                Opcode::Halt => {
                    return Ok(self.stack.pop().unwrap_or(Value::Null));
                }
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
                    if count > self.stack.len() {
                        return Err(self.runtime_exception("list construction underflow"));
                    }
                    let start = self.stack.len() - count;
                    let values = self.stack.drain(start..).collect::<Vec<_>>();
                    self.stack.push(Value::Array(values));
                }
                Opcode::MakeArray => {
                    let capacity = instruction.operand_a as usize;
                    let array = if capacity == 0 {
                        Vec::new()
                    } else {
                        Vec::with_capacity(capacity)
                    };
                    self.stack.push(Value::Array(array));
                }
                Opcode::MakeObject => {
                    let field_count = instruction.operand_a as usize;
                    if self.stack.len() < field_count * 2 {
                        return Err(self.runtime_exception("object construction underflow"));
                    }
                    let mut map = HashMap::with_capacity(field_count);
                    for _ in 0..field_count {
                        let value = self.stack.pop().unwrap_or(Value::Null);
                        let key_value = self.stack.pop().unwrap_or(Value::Null);
                        let key = self.expect_string_key(key_value, "MakeObject")?;
                        map.insert(key, value);
                    }
                    let object = self.allocate_object(map)?;
                    self.stack.push(object);
                }
                Opcode::Push => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let mut target = self.stack.pop().unwrap_or(Value::Null);
                    if let Value::Array(ref mut arr) = target {
                        arr.push(value);
                        self.stack.push(target);
                    } else {
                        return Err(self.runtime_exception("Push called without array context"));
                    }
                }
                Opcode::Index => {
                    let index_value = self.stack.pop().unwrap_or(Value::Null);
                    let collection = self.stack.pop().unwrap_or(Value::Null);
                    match collection {
                        Value::Array(items) => {
                            let idx = self.expect_index(index_value, "Index")?;
                            let value = items.get(idx).cloned().unwrap_or(Value::Null);
                            self.stack.push(value);
                        }
                        other => {
                            // Fallback to the generic core_index builtin for strings/objects.
                            let value = self
                                .ctx
                                .builtins
                                .invoke_sync("core_index", &[other, index_value])?;
                            self.stack.push(value);
                        }
                    }
                }
                Opcode::SetIndex => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let index_value = self.stack.pop().unwrap_or(Value::Null);
                    let mut array = self.stack.pop().unwrap_or(Value::Null);
                    let idx = self.expect_index(index_value, "SetIndex")?;
                    if let Value::Array(ref mut items) = array {
                        if let Some(slot) = items.get_mut(idx) {
                            *slot = value;
                            self.stack.push(array);
                        } else {
                            return Err(
                                self.runtime_exception(format!("SetIndex out of bounds: {idx}"))
                            );
                        }
                    } else {
                        return Err(
                            self.runtime_exception("SetIndex expects array source on stack")
                        );
                    }
                }
                Opcode::LoadMember => {
                    let name = self
                        .string_constant(instruction.operand_a as usize)
                        .ok_or_else(|| {
                            self.runtime_exception(format!(
                                "invalid property name constant {}",
                                instruction.operand_a
                            ))
                        })?;
                    let target = self.stack.pop().unwrap_or(Value::Null);
                    let value = self.load_member_value(target, &name)?;
                    self.stack.push(value);
                }
                Opcode::SetMember => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let key_value = self.stack.pop().unwrap_or(Value::Null);
                    let target = self.stack.pop().unwrap_or(Value::Null);
                    let key = self.expect_string_key(key_value, "SetMember")?;
                    let reference = self.expect_object_reference(target, "SetMember")?;
                    self.set_object_field(reference, key, value.clone())?;
                    self.stack.push(value);
                }
                Opcode::Print => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    print!("{}", value.stringify());
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
                    let arg_count = instruction.operand_b as usize;
                    let args = self.collect_args(arg_count);
                    if instruction.operand_a == DYNAMIC_CALL_TARGET {
                        let callee_value = self.stack.pop().unwrap_or(Value::Null);
                        let method_name = if instruction.operand_c != 0 {
                            self.string_constant(instruction.operand_c as usize)
                        } else {
                            None
                        };
                        self.call_dynamic(callee_value, args, method_name)
                            .map_err(|err| self.enrich_error(err))?;
                        advance_ip = false;
                    } else {
                        let function_index = instruction.operand_a as usize;
                        if let Some(value) =
                            self.execute_tier1_if_available(function_index, &args)?
                        {
                            self.stack.push(value);
                            continue;
                        }
                        if self.consume_pending_deopt_frame() {
                            continue;
                        }
                        match self
                            .execute_tier0_if_available(function_index, &args)
                            .map_err(|err| self.enrich_error(err))?
                        {
                            Some(value) => {
                                self.stack.push(value);
                            }
                            None => {
                                self.call_function(function_index, args)
                                    .map_err(|err| self.enrich_error(err))?;
                                advance_ip = false;
                            }
                        }
                    }
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
                    let result = match name.as_str() {
                        "keys" | "object::keys" | "std::object::keys" => {
                            self.builtin_object_keys(&args)
                        }
                        "values" | "object::values" | "std::object::values" => {
                            self.builtin_object_values(&args)
                        }
                        "has_key" | "object::has_key" | "std::object::has_key" => {
                            self.builtin_object_has_key(&args)
                        }
                        "len" | "std::string::len" | "string::len" => {
                            self.builtin_len_extended(&args)
                        }
                        _ => self.ctx.builtins.invoke_sync(&name, &args),
                    }
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
                Opcode::CoreCall => {
                    let name = self
                        .string_constant(instruction.operand_a as usize)
                        .ok_or_else(|| {
                            self.runtime_exception(format!(
                                "invalid core builtin name constant {}",
                                instruction.operand_a
                            ))
                        })?;
                    let arg_count = instruction.operand_b as usize;
                    let args = self.collect_args(arg_count);
                    let result =
                        invoke_core_builtin(&name, &args).map_err(|err| self.enrich_error(err))?;
                    self.stack.push(result);
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
                        core_task,
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
                        core_task.cancel();
                        core_completion.store(true, Ordering::SeqCst);
                        core_task.wait();
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
                                core_task.cancel();
                                core_completion.store(true, Ordering::SeqCst);
                                core_task.wait();
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
                                    core_task.cancel();
                                    core_completion.store(true, Ordering::SeqCst);
                                    core_task.wait();
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
                    core_task.wait();
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
                Opcode::Return | Opcode::CoreReturn => {
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
                Opcode::CoreYield => {
                    eprintln!(
                        "[solvrascript] warning: CoreYield opcode is not implemented; returning null"
                    );
                    return Ok(Value::Null);
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
        self.profile.record_function(&function.name);
        if self.ctx.options.jit_tier0 && self.profile.hot_functions.is_hot(&function.name) {
            self.request_tier0(&function.name);
        }

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
            transfer_locals: None,
            transfer_debug: None,
        };
        self.frames.push(frame);
        Ok(())
    }

    fn execute_tier1_if_available(
        &mut self,
        function_index: usize,
        args: &[Value],
    ) -> SolvraResult<Option<Value>> {
        // @TIER1 integration
        if !self.ctx.options.jit_tier1 {
            return Ok(None);
        }
        if self.mir_function_map.is_empty() {
            self.initialize_mir_function_map();
        }
        let Some(function) = self.ctx.program.functions.get(function_index) else {
            return Ok(None);
        };
        let Some(mir_module) = self.ctx.options.tier1_mir_module.clone() else {
            return Ok(None);
        };
        let Some(osr_registry) = self.ctx.options.tier1_osr_registry.clone() else {
            return Ok(None);
        };
        let Some(dispatcher_mutex) = &self.ctx.jit_dispatcher else {
            return Ok(None);
        };
        if let Ok(mut dispatcher) = dispatcher_mutex.lock() {
            dispatcher.set_ic_debug(self.ctx.options.jit_deopt_debug);
            dispatcher.set_osr_validate(self.ctx.options.jit_osr_validate);
            dispatcher.prepare_tier1_module(mir_module.as_ref(), Some(osr_registry.as_ref()));
            if self.ctx.options.jit_deopt_debug {
                println!("[tier1][exec] {}", function.name);
            }
            if self.ctx.options.jit_osr_debug {
                if let Some(func_id) = mir_module.function_id(&function.name) {
                    let _ = dispatcher.try_osr_landing_pad(func_id, 0);
                }
            }
            if let Some(result) = dispatcher.try_execute_tier1(&function.name, args) {
                return Ok(Some(result));
            }
        }
        if let Some(events) = self.drain_deopt_events() {
            if self.ctx.options.jit_deopt_debug {
                for event in &events {
                    println!(
                        "[tier1][deopt] func_id={} site={} pc={}",
                        event.function_id, event.deopt_site.0, event.snapshot.pc
                    );
                }
            }
            if self.handle_deopt_events(function_index, events)? {
                return Ok(None);
            }
        }
        Ok(None)
    }

    fn handle_deopt_events(
        &mut self,
        function_index: usize,
        mut events: Vec<DeoptEvent>,
    ) -> SolvraResult<bool> {
        if !self.pending_deopt_events.is_empty() {
            events.extend(self.pending_deopt_events.drain(..));
        }
        let mut unmatched = Vec::new();
        for event in events {
            let mapped = self.mir_function_map.get(&event.function_id).copied();
            if mapped == Some(function_index) {
                if self.ctx.options.jit_transfer_debug {
                    if let Some(plan) = event.transfer_plan.as_ref() {
                        let reconstructed =
                            solvra_core::jit::tier1_osr::reconstruct_locals_with_transfer_plan(
                                plan,
                                &event.snapshot,
                            );
                        println!(
                            "[transfer][debug] func_id={} locals={} incomplete={} missing={}",
                            event.function_id,
                            reconstructed.locals.len(),
                            reconstructed.incomplete,
                            reconstructed.missing_fields
                        );
                    } else {
                        println!(
                            "[transfer][debug] func_id={} no transfer plan available",
                            event.function_id
                        );
                    }
                }
                if self.apply_deopt_resume(event)? {
                    return Ok(true);
                }
            } else {
                unmatched.push(event);
            }
        }
        self.pending_deopt_events.extend(unmatched);
        Ok(false)
    }

    fn apply_deopt_resume(&mut self, event: DeoptEvent) -> SolvraResult<bool> {
        let Some(&function_index) = self.mir_function_map.get(&event.function_id) else {
            return Ok(false);
        };
        let Some(function) = self.ctx.program.functions.get(function_index) else {
            return Ok(false);
        };
        let expected_locals = function.locals as usize;
        let mut transfer_locals = None;
        let mut transfer_debug = None;
        if let Some(plan) = event.transfer_plan.as_ref() {
            let reconstructed = solvra_core::jit::tier1_osr::reconstruct_locals_with_transfer_plan(
                plan,
                &event.snapshot,
            );
            let ready = !reconstructed.incomplete && reconstructed.locals.len() == expected_locals;
            transfer_debug = Some((reconstructed.incomplete, reconstructed.missing_fields));
            if ready {
                transfer_locals = Some(reconstructed.locals);
            }
        }
        let mut locals = event.snapshot.locals;
        if locals.len() > expected_locals {
            locals.truncate(expected_locals);
        } else if locals.len() < expected_locals {
            locals.resize(expected_locals, Value::Null);
        }
        let max_ip = function.instructions.len();
        let ip = if max_ip == 0 {
            0
        } else {
            event.snapshot.pc.min(max_ip - 1)
        };
        let stack_base = self.stack.len();
        for value in event.snapshot.stack {
            self.stack.push(value);
        }
        let frame = CallFrame {
            function_index,
            ip,
            locals,
            stack_base,
            transfer_locals,
            transfer_debug,
        };
        self.frames.push(frame);
        self.pending_deopt_frame = true;
        Ok(true)
    }

    fn consume_pending_deopt_frame(&mut self) -> bool {
        if self.pending_deopt_frame {
            self.pending_deopt_frame = false;
            if let Some(frame) = self.frames.last_mut() {
                let mut applied = false;
                if self.ctx.options.jit_transfer_debug {
                    println!("[transfer][debug] snapshot locals: {:?}", frame.locals);
                    if let Some(info) = frame.transfer_debug {
                        println!(
                            "[transfer][debug] metrics incomplete={} missing={}",
                            info.0, info.1
                        );
                    }
                    if let Some(ref tlocals) = frame.transfer_locals {
                        println!("[transfer][debug] transfer locals: {:?}", tlocals);
                    } else {
                        println!("[transfer][debug] transfer locals: (none)");
                    }
                }
                if self.ctx.options.jit_transfer_debug {
                    if let Some(ref tlocals) = frame.transfer_locals {
                        if tlocals.len() == frame.locals.len() {
                            frame.locals = tlocals.clone();
                            applied = true;
                        }
                    }
                }
                if self.ctx.options.jit_transfer_debug {
                    println!("[transfer][debug] applied_transfer_locals={}", applied);
                }
            }
            true
        } else {
            false
        }
    }

    fn execute_tier0_if_available(
        &mut self,
        function_index: usize,
        args: &[Value],
    ) -> SolvraResult<Option<Value>> {
        if !self.ctx.options.jit_tier0 {
            return Ok(None);
        }
        let Some(program_function) = self.ctx.program.functions.get(function_index).cloned() else {
            return Ok(None);
        };
        if !self.profile.hot_functions.is_hot(&program_function.name) {
            return Ok(None);
        }
        let Some(module) = self.ctx.options.jit_ir_module.clone() else {
            return Ok(None);
        };
        let Some(artifact) = self.fetch_tier0_artifact(&program_function.name)? else {
            return Ok(None);
        };
        let Some(runtime_args) = self.convert_args_to_runtime(args) else {
            return Ok(None);
        };
        let result = match execute_tier0(&artifact, module.as_ref(), &runtime_args) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let Some(vm_value) = self.convert_runtime_to_vm(result) else {
            return Ok(None);
        };
        self.record_tier0_execution(artifact.function_id);
        self.profile.record_function(&program_function.name);
        Ok(Some(vm_value))
    }

    fn fetch_tier0_artifact(&self, function_name: &str) -> SolvraResult<Option<Tier0Artifact>> {
        let Some(module) = &self.ctx.options.jit_ir_module else {
            return Ok(None);
        };
        let Some(dispatcher_mutex) = &self.ctx.jit_dispatcher else {
            return Ok(None);
        };
        let Some(function) = module.function_by_name(function_name) else {
            return Ok(None);
        };
        let artifact = dispatcher_mutex
            .lock()
            .map_err(|_| self.runtime_exception("Tier-0 dispatcher lock poisoned"))?
            .get_or_compile_tier0(function)
            .clone();
        Ok(Some(artifact))
    }

    fn record_tier0_execution(&self, function_id: Tier0FunctionId) {
        if let Some(dispatcher_mutex) = &self.ctx.jit_dispatcher {
            if let Ok(mut dispatcher) = dispatcher_mutex.lock() {
                dispatcher.record_execution(function_id);
            }
        }
    }

    fn convert_args_to_runtime(&self, args: &[Value]) -> Option<Vec<RuntimeValue>> {
        let mut converted = Vec::with_capacity(args.len());
        for value in args {
            converted.push(self.convert_vm_value_to_runtime(value)?);
        }
        Some(converted)
    }

    fn convert_vm_value_to_runtime(&self, value: &Value) -> Option<RuntimeValue> {
        match value {
            Value::Null => Some(RuntimeValue::Null),
            Value::Boolean(v) => Some(RuntimeValue::Bool(*v)),
            Value::Integer(v) => Some(RuntimeValue::Int(*v)),
            Value::Float(v) => Some(RuntimeValue::Float(*v)),
            Value::String(text) => Some(RuntimeValue::String(text.clone())),
            Value::Array(items) => {
                let mut converted = Vec::with_capacity(items.len());
                for item in items {
                    converted.push(self.convert_vm_value_to_runtime(item)?);
                }
                Some(RuntimeValue::Array(Rc::new(RefCell::new(converted))))
            }
            Value::Object(reference) => self.convert_object_to_runtime(*reference),
        }
    }

    fn convert_object_to_runtime(&self, reference: ObjectHandle) -> Option<RuntimeValue> {
        enum Snapshot {
            Map(HashMap<String, Value>),
            List(Vec<Value>),
        }

        let snapshot = {
            let arena = self.ctx.arena.lock().ok()?;
            let object = arena.get(reference)?;
            match object {
                HeapObject::Map(map) => Snapshot::Map(map.clone()),
                HeapObject::List(items) => Snapshot::List(items.clone()),
                HeapObject::Native(_) => return None,
            }
        };

        match snapshot {
            Snapshot::Map(map) => {
                let mut converted = HashMap::new();
                for (key, value) in map {
                    converted.insert(key, self.convert_vm_value_to_runtime(&value)?);
                }
                Some(RuntimeValue::Object(Rc::new(RefCell::new(converted))))
            }
            Snapshot::List(items) => {
                let mut converted = Vec::with_capacity(items.len());
                for item in items {
                    converted.push(self.convert_vm_value_to_runtime(&item)?);
                }
                Some(RuntimeValue::Array(Rc::new(RefCell::new(converted))))
            }
        }
    }

    fn convert_runtime_to_vm(&self, value: RuntimeValue) -> Option<Value> {
        match value {
            RuntimeValue::Int(v) => Some(Value::Integer(v)),
            RuntimeValue::Float(v) => Some(Value::Float(v)),
            RuntimeValue::Bool(v) => Some(Value::Boolean(v)),
            RuntimeValue::String(text) => Some(Value::String(text)),
            RuntimeValue::Null => Some(Value::Null),
            RuntimeValue::Function(func) => Some(Value::Integer(func.index() as i64)),
            RuntimeValue::Array(values) => {
                let items = values
                    .borrow()
                    .iter()
                    .cloned()
                    .map(|item| self.convert_runtime_to_vm(item))
                    .collect::<Option<Vec<_>>>()?;
                Some(Value::Array(items))
            }
            RuntimeValue::Object(map) => {
                let mut converted = HashMap::new();
                for (key, value) in map.borrow().iter() {
                    converted.insert(key.clone(), self.convert_runtime_to_vm(value.clone())?);
                }
                self.allocate_object(converted).ok()
            }
        }
    }

    fn call_dynamic(
        &mut self,
        callee: Value,
        args: Vec<Value>,
        method_name: Option<String>,
    ) -> SolvraResult<()> {
        match callee {
            Value::Integer(id) if id >= 0 => self.call_function(id as usize, args),
            other => {
                let message = if let Some(name) = method_name {
                    format!("TypeError: member '{name}' is not callable")
                } else {
                    format!("TypeError: {} is not callable", other.type_name())
                };
                Err(self.runtime_exception(message))
            }
        }
    }

    fn collect_args(&mut self, count: usize) -> Vec<Value> {
        let mut args = Vec::with_capacity(count);
        for _ in 0..count {
            args.push(self.stack.pop().unwrap_or(Value::Null));
        }
        args.reverse();
        args
    }

    fn request_tier0(&self, function_name: &str) {
        if !self.ctx.options.jit_tier0 {
            return;
        }
        let module = match &self.ctx.options.jit_ir_module {
            Some(module) => module,
            None => return,
        };
        let dispatcher = match &self.ctx.jit_dispatcher {
            Some(dispatcher) => dispatcher,
            None => return,
        };
        if let Some(function) = module.function_by_name(function_name) {
            if let Ok(mut guard) = dispatcher.lock() {
                guard.request_tier0(function);
            }
        }
    }

    fn expect_index(&self, value: Value, context: &str) -> SolvraResult<usize> {
        match value {
            Value::Integer(idx) if idx >= 0 => Ok(idx as usize),
            Value::Integer(_) => {
                Err(self.runtime_exception(format!("{context} expects non-negative integer index")))
            }
            other => Err(self.runtime_exception(format!(
                "{context} expects integer index but found {}",
                other.type_name()
            ))),
        }
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

    fn arena_lock(&self) -> SolvraResult<std::sync::MutexGuard<'_, ArenaAllocator>> {
        self.ctx
            .arena
            .lock()
            .map_err(|_| self.runtime_exception("arena lock poisoned"))
    }

    fn allocate_object(&self, fields: HashMap<String, Value>) -> SolvraResult<Value> {
        let mut arena = self.arena_lock()?;
        let reference = arena.allocate(HeapObject::Map(fields));
        Ok(Value::Object(reference))
    }

    fn expect_string_key(&self, value: Value, context: &str) -> SolvraResult<String> {
        match value {
            Value::String(text) => Ok(text),
            other => Err(self.runtime_exception(format!(
                "{context} expects string key but found {}",
                other.type_name()
            ))),
        }
    }

    fn expect_object_reference(&self, value: Value, context: &str) -> SolvraResult<ObjectHandle> {
        if let Value::Object(reference) = value {
            Ok(reference)
        } else {
            Err(self.runtime_exception(format!(
                "{context} expects object receiver but found {}",
                value.type_name()
            )))
        }
    }

    fn load_member_value(&self, target: Value, property: &str) -> SolvraResult<Value> {
        let reference = self.expect_object_reference(target, "LoadMember")?;
        let arena = self.arena_lock()?;
        let value = match arena.get(reference) {
            Some(HeapObject::Map(map)) => map.get(property).cloned().unwrap_or(Value::Null),
            Some(HeapObject::List(_)) => {
                return Err(self.runtime_exception("LoadMember cannot access list entries"));
            }
            Some(HeapObject::Native(_)) => {
                return Err(
                    self.runtime_exception("LoadMember cannot access native object members")
                );
            }
            None => return Err(self.runtime_exception("dangling object reference")),
        };
        Ok(value)
    }

    fn set_object_field(
        &self,
        reference: ObjectHandle,
        key: String,
        value: Value,
    ) -> SolvraResult<()> {
        let mut arena = self.arena_lock()?;
        let object = arena
            .get_mut(reference)
            .ok_or_else(|| self.runtime_exception("dangling object reference"))?;
        match object {
            HeapObject::Map(map) => {
                map.insert(key, value);
                Ok(())
            }
            HeapObject::List(_) => {
                Err(self.runtime_exception("SetMember cannot target list entries"))
            }
            HeapObject::Native(_) => {
                Err(self.runtime_exception("SetMember cannot mutate native objects"))
            }
        }
    }

    fn builtin_object_keys(&self, args: &[Value]) -> SolvraResult<Value> {
        let Some(target) = args.get(0) else {
            return Err(self.runtime_exception("keys() expects object argument"));
        };
        let reference = self.expect_object_reference(target.clone(), "keys")?;
        let arena = self.arena_lock()?;
        let object = arena
            .get(reference)
            .ok_or_else(|| self.runtime_exception("dangling object reference"))?;
        if let HeapObject::Map(map) = object {
            let keys = map
                .keys()
                .map(|key| Value::String(key.clone()))
                .collect::<Vec<_>>();
            Ok(Value::Array(keys))
        } else {
            Err(self.runtime_exception("keys() expects object input"))
        }
    }

    fn builtin_object_values(&self, args: &[Value]) -> SolvraResult<Value> {
        let Some(target) = args.get(0) else {
            return Err(self.runtime_exception("values() expects object argument"));
        };
        let reference = self.expect_object_reference(target.clone(), "values")?;
        let arena = self.arena_lock()?;
        let object = arena
            .get(reference)
            .ok_or_else(|| self.runtime_exception("dangling object reference"))?;
        if let HeapObject::Map(map) = object {
            let values = map.values().cloned().collect::<Vec<_>>();
            Ok(Value::Array(values))
        } else {
            Err(self.runtime_exception("values() expects object input"))
        }
    }

    fn builtin_object_has_key(&self, args: &[Value]) -> SolvraResult<Value> {
        if args.len() != 2 {
            return Err(self.runtime_exception("has_key() expects object and key arguments"));
        }
        let reference = self.expect_object_reference(args[0].clone(), "has_key")?;
        let key = self.expect_string_key(args[1].clone(), "has_key")?;
        let arena = self.arena_lock()?;
        let object = arena
            .get(reference)
            .ok_or_else(|| self.runtime_exception("dangling object reference"))?;
        if let HeapObject::Map(map) = object {
            Ok(Value::Boolean(map.contains_key(&key)))
        } else {
            Err(self.runtime_exception("has_key() expects object input"))
        }
    }

    fn builtin_len_extended(&self, args: &[Value]) -> SolvraResult<Value> {
        let Some(target) = args.get(0) else {
            return Err(self.runtime_exception("len() expects one argument"));
        };
        match target {
            Value::String(text) => Ok(Value::Integer(text.chars().count() as i64)),
            Value::Array(items) => Ok(Value::Integer(items.len() as i64)),
            Value::Object(_) => {
                let reference = self.expect_object_reference(target.clone(), "len")?;
                let arena = self.arena_lock()?;
                let object = arena
                    .get(reference)
                    .ok_or_else(|| self.runtime_exception("dangling object reference"))?;
                if let HeapObject::Map(map) = object {
                    Ok(Value::Integer(map.len() as i64))
                } else {
                    Err(self.runtime_exception("len() expects object input"))
                }
            }
            other => {
                Err(self
                    .runtime_exception(format!("len() not supported for {}", other.type_name())))
            }
        }
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
            | Opcode::MakeArray
            | Opcode::MakeObject
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
        let watchdog_label = format!("task-watchdog#{task_id}");
        let core_task = self
            .ctx
            .options
            .executor
            .spawn_with(Some(watchdog_label), move |ctx| {
                while !completion_clone.load(Ordering::SeqCst) {
                    if ctx.is_cancelled() {
                        break;
                    }
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
                core_task,
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
            task.core_task.cancel();
            task.core_completion.store(true, Ordering::SeqCst);
            task.core_task.wait();
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

    fn drain_deopt_events(&self) -> Option<Vec<DeoptEvent>> {
        let dispatcher_mutex = self.ctx.jit_dispatcher.as_ref()?;
        let mut dispatcher = dispatcher_mutex.lock().ok()?;
        if dispatcher.recent_deopts().is_empty() {
            return None;
        }
        let events = dispatcher.recent_deopts().to_vec();
        dispatcher.clear_recent_deopts();
        Some(events)
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

    fn finish_profile(&mut self) {
        self.profile.end();
        if let Some(duration) = self.profile.total_duration {
            self.emit_telemetry_event(
                TelemetryEventKind::RuntimeSummary,
                self.task_label.clone(),
                Some(duration.as_millis() as u64),
                None,
            );
        }
        let mut fused_printed = false;
        self.print_jit_stats();
        if self.ctx.options.jit_stats && self.ctx.options.jit_tier1 {
            fused_printed = true;
        }
        if self.ctx.options.jit_tier1_fused_debug && !fused_printed {
            if let Some(dispatcher_mutex) = &self.ctx.jit_dispatcher {
                if let Ok(dispatcher) = dispatcher_mutex.lock() {
                    if let Some(cache) = dispatcher.tier1_code_cache() {
                        crate::compiler::tier1::dump_fused_ic_summary(cache);
                    } else {
                        println!("[tier1][fused] (no Tier-1 code cache available)");
                    }
                }
            }
        }
        if self.ctx.options.jit_osr_debug {
            if let Some(dispatcher_mutex) = &self.ctx.jit_dispatcher {
                if let Ok(dispatcher) = dispatcher_mutex.lock() {
                    if let (Some(mir), Some(registry)) = (
                        self.ctx.options.tier1_mir_module.as_ref(),
                        self.ctx.options.tier1_osr_registry.as_ref(),
                    ) {
                        println!("[tier1][osr] landing pads:");
                        for function in mir.functions() {
                            let mut printed = false;
                            for pad in registry.landing_pad_iter(function.id) {
                                if !printed {
                                    println!("fn {}:", function.name);
                                    printed = true;
                                }
                                println!(
                                    "  pad ?: bb {}, osr {}, schema=locals:{} temps:{} stack:{}",
                                    pad.bb_id.0,
                                    pad.osr_point.0,
                                    pad.snapshot_schema.locals,
                                    pad.snapshot_schema.temps,
                                    pad.snapshot_schema.stack
                                );
                            }
                            if !printed {
                                println!("fn {}:", function.name);
                                println!("  (no landing pads)");
                            }
                        }
                    } else if let (Some(cache), Some(mir)) = (
                        dispatcher.tier1_code_cache(),
                        self.ctx.options.tier1_mir_module.as_ref(),
                    ) {
                        crate::compiler::tier1::dump_osr_landing_pads(mir.as_ref(), cache);
                    } else {
                        println!("[tier1][osr] landing pads unavailable");
                    }
                }
            }
        }
    }

    fn print_jit_stats(&self) {
        if !self.ctx.options.jit_stats {
            return;
        }
        let snapshot = self.profile.hot_functions.snapshot();
        let Some(dispatcher_mutex) = &self.ctx.jit_dispatcher else {
            println!("[JIT] Tier-0 dispatcher unavailable.");
            return;
        };
        let Ok(dispatcher) = dispatcher_mutex.lock() else {
            println!("[JIT] Tier-0 dispatcher unavailable.");
            return;
        };
        println!();
        if dispatcher.is_empty() {
            println!("[JIT] Tier-0 compiled functions: (none)");
            return;
        }
        println!("[JIT] Tier-0 compiled functions:");
        for artifact in dispatcher.compiled_functions() {
            let calls = snapshot.get(&artifact.name).copied().unwrap_or(0);
            let hot = self.profile.hot_functions.is_hot(&artifact.name);
            let execs = dispatcher.execution_count(artifact.function_id);
            println!(
                " - {} (calls: {}, hot: {}, tier0_exec_count: {})",
                artifact.name, calls, hot, execs
            );
        }
        if self.ctx.options.jit_tier1 {
            if let Some(cache) = dispatcher.tier1_code_cache() {
                crate::compiler::tier1::dump_fused_ic_summary(cache);
            } else {
                println!("[JIT] Tier-1 fused ICs: (unavailable)");
            }
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
    transfer_locals: Option<Vec<Value>>,
    transfer_debug: Option<(bool, usize)>,
}

fn extract_task_id(value: Value) -> SolvraResult<u64> {
    match value {
        Value::Integer(id) if id >= 0 => Ok(id as u64),
        other => Err(SolvraError::Internal(format!(
            "await expects task identifier, received {other:?}"
        ))),
    }
}

fn invoke_core_builtin(name: &str, args: &[Value]) -> SolvraResult<Value> {
    match name {
        "core::print" => {
            let payload = args
                .first()
                .map(|value| value.stringify())
                .unwrap_or_default();
            print!("{payload}");
            Ok(Value::Null)
        }
        "core::println" => {
            let payload = args
                .first()
                .map(|value| value.stringify())
                .unwrap_or_default();
            println!("{payload}");
            Ok(Value::Null)
        }
        other if is_core_stub_call(other) => {
            let message = core_stub_message(other);
            eprintln!("[runtime] {message}");
            Err(SolvraError::Internal(message))
        }
        other => Err(SolvraError::Internal(format!(
            "unknown core builtin function '{other}'"
        ))),
    }
}

fn execute_arithmetic(opcode: Opcode, lhs: Value, rhs: Value) -> SolvraResult<Value> {
    if opcode == Opcode::Add {
        if let Some(result) = string_add(&lhs, &rhs) {
            return Ok(Value::String(result));
        }
    }
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

fn string_add(lhs: &Value, rhs: &Value) -> Option<String> {
    match (lhs, rhs) {
        (Value::String(left), Value::String(right)) => {
            let mut result = left.clone();
            result.push_str(right);
            Some(result)
        }
        (Value::String(left), other) => {
            let mut result = left.clone();
            result.push_str(&value_to_string(other));
            Some(result)
        }
        (other, Value::String(right)) => {
            let mut result = value_to_string(other);
            result.push_str(right);
            Some(result)
        }
        _ => None,
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
    value.stringify()
}

fn opcode_name(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::Halt => "Halt",
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
        Opcode::MakeArray => "MakeArray",
        Opcode::MakeObject => "MakeObject",
        Opcode::LoadMember => "LoadMember",
        Opcode::SetMember => "SetMember",
        Opcode::Index => "Index",
        Opcode::SetIndex => "SetIndex",
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
        Opcode::CoreCall => "CoreCall",
        Opcode::Await => "Await",
        Opcode::CoreReturn => "CoreReturn",
        Opcode::Return => "Return",
        Opcode::CoreYield => "CoreYield",
        Opcode::Push => "Push",
        Opcode::Print => "Print",
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
