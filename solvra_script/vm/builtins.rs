use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::thread;
use std::time::Duration;

use serde_json::json;
use solvra_core::{SolvraError, SolvraResult, Value};

use super::async_control::AsyncControl;
use super::metrics::TelemetryCollector;
use super::runtime::{MemoryStats, MemoryTracker};

type SyncBuiltin = fn(&[Value]) -> SolvraResult<Value>;
type AsyncBuiltin = fn(Vec<Value>) -> Pin<Box<dyn Future<Output = SolvraResult<Value>> + 'static>>;

#[derive(Clone)]
pub struct Builtins {
    sync: HashMap<String, SyncBuiltin>,
    #[allow(dead_code)]
    async_map: HashMap<String, AsyncBuiltin>,
    context: BuiltinContext,
}

impl Builtins {
    pub fn default() -> Self {
        Self::with_context(BuiltinContext::default())
    }

    pub fn with_context(context: BuiltinContext) -> Self {
        let mut builtins = Self {
            sync: HashMap::new(),
            async_map: HashMap::new(),
            context,
        };
        builtins.register_sync("print", builtin_print);
        builtins.register_sync("println", builtin_println);
        builtins.register_sync("sleep", builtin_sleep);
        builtins
    }

    pub fn register_sync(&mut self, name: &str, func: SyncBuiltin) {
        self.sync.insert(name.to_string(), func);
    }

    #[allow(dead_code)]
    pub fn register_async(&mut self, name: &str, func: AsyncBuiltin) {
        self.async_map.insert(name.to_string(), func);
    }

    pub fn invoke_sync(&self, name: &str, args: &[Value]) -> SolvraResult<Value> {
        match name {
            "core_memory_events" => return self.core_memory_events(),
            "core_timeout_stats" => return self.core_timeout_stats(),
            "core_cancel_task" => return self.core_cancel_task(args),
            "core_with_deadline" => return self.core_with_deadline(args),
            _ => {}
        }
        if let Some(func) = self.sync.get(name) {
            func(args)
        } else {
            Err(SolvraError::Internal(format!(
                "unknown builtin function '{name}'"
            )))
        }
    }

    #[allow(dead_code)]
    pub fn invoke_async(
        &self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Option<Pin<Box<dyn Future<Output = SolvraResult<Value>> + 'static>>>, SolvraError>
    {
        if let Some(func) = self.async_map.get(name) {
            Ok(Some(func(args)))
        } else if let Some(sync) = self.sync.get(name) {
            let args_clone = args;
            let sync_fn = *sync;
            let fut = Box::pin(async move { sync_fn(&args_clone) });
            Ok(Some(fut))
        } else {
            Err(SolvraError::Internal(format!(
                "unknown async builtin '{name}'"
            )))
        }
    }
}

fn builtin_print(args: &[Value]) -> SolvraResult<Value> {
    if let Some(value) = args.first() {
        print!("{}", value_to_string(value));
    }
    Ok(Value::Null)
}

#[derive(Clone, Default)]
pub struct BuiltinContext {
    pub memory_tracker: Option<MemoryTracker>,
    pub telemetry: Option<TelemetryCollector>,
    pub async_control: Option<AsyncControl>,
}

impl Builtins {
    fn core_memory_events(&self) -> SolvraResult<Value> {
        if let Some(collector) = &self.context.telemetry {
            let events = collector.snapshot();
            let payload = serde_json::to_string(&json!({ "events": events })).map_err(|err| {
                SolvraError::Internal(format!("failed to serialize telemetry events: {err}"))
            })?;
            return Ok(Value::String(payload));
        }
        Ok(Value::String("{\"events\":[]}".to_string()))
    }

    fn core_timeout_stats(&self) -> SolvraResult<Value> {
        let stats = if let Some(tracker) = &self.context.memory_tracker {
            tracker.snapshot()
        } else {
            MemoryStats::default()
        };
        let payload = serde_json::to_string(&serde_json::json!({
            "timeouts": stats.timeouts,
            "timeout_stack_samples": stats.timeout_stack_samples,
            "timeout_constant_samples": stats.timeout_constant_samples,
            "scheduler_ticks": stats.scheduler_ticks,
            "peak_task_elapsed_ms": stats.peak_task_elapsed_ms,
        }))
        .map_err(|err| {
            SolvraError::Internal(format!("failed to serialize timeout stats: {err}"))
        })?;
        Ok(Value::String(payload))
    }

    fn core_cancel_task(&self, args: &[Value]) -> SolvraResult<Value> {
        let control = match &self.context.async_control {
            Some(control) => control,
            None => return Ok(Value::Boolean(false)),
        };
        let task_id = args
            .get(0)
            .and_then(extract_integer)
            .ok_or_else(|| SolvraError::Internal("core_cancel_task expects task id".into()))?;
        Ok(Value::Boolean(control.cancel(task_id as u64)))
    }

    fn core_with_deadline(&self, args: &[Value]) -> SolvraResult<Value> {
        let control = match &self.context.async_control {
            Some(control) => control,
            None => return Ok(Value::Boolean(false)),
        };
        let task_id = args
            .get(0)
            .and_then(extract_integer)
            .ok_or_else(|| SolvraError::Internal("core_with_deadline expects task id".into()))?;
        let deadline_ms = args.get(1).and_then(extract_integer).ok_or_else(|| {
            SolvraError::Internal("core_with_deadline expects deadline in ms".into())
        })?;
        if deadline_ms <= 0 {
            Ok(Value::Boolean(control.clear_deadline(task_id as u64)))
        } else {
            let duration = Duration::from_millis(deadline_ms as u64);
            Ok(Value::Boolean(
                control.set_deadline(task_id as u64, duration),
            ))
        }
    }
}

fn builtin_println(args: &[Value]) -> SolvraResult<Value> {
    if let Some(value) = args.first() {
        println!("{}", value_to_string(value));
    } else {
        println!();
    }
    Ok(Value::Null)
}

fn builtin_sleep(args: &[Value]) -> SolvraResult<Value> {
    let millis = args.get(0).and_then(extract_integer).unwrap_or(0);
    if millis > 0 {
        thread::sleep(Duration::from_millis(millis as u64));
    }
    Ok(Value::Null)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Integer(int) => int.to_string(),
        Value::Float(float) => {
            if float.fract() == 0.0 {
                format!("{:.0}", float)
            } else {
                float.to_string()
            }
        }
        Value::String(text) => text.clone(),
        Value::Object(_) => "<object>".into(),
    }
}

fn extract_integer(value: &Value) -> Option<i64> {
    match value {
        Value::Integer(int) => Some(*int),
        _ => None,
    }
}
