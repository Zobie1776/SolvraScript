use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde_json::Value as JsonValue;
use serde_json::json;
use solvra_core::{SolvraError, SolvraResult, Value};

use super::async_control::AsyncControl;
use super::legacy_builtins;
use super::metrics::TelemetryCollector;
use super::runtime::{MemoryStats, MemoryTracker};

type SyncBuiltin = fn(&Builtins, &[Value]) -> SolvraResult<Value>;
type AsyncBuiltin = fn(Vec<Value>) -> Pin<Box<dyn Future<Output = SolvraResult<Value>> + 'static>>;

#[derive(Clone)]
pub struct Builtins {
    sync: HashMap<String, SyncBuiltin>,
    #[allow(dead_code)]
    async_map: HashMap<String, AsyncBuiltin>,
    context: BuiltinContext,
    toml_cache: Arc<Mutex<HashMap<PathBuf, TomlCacheEntry>>>,
    start_time: Instant,
}

#[derive(Clone)]
struct TomlCacheEntry {
    modified: Option<SystemTime>,
    json: String,
}

impl Builtins {
    #[allow(dead_code)] // Reserved for external callers that construct builtins directly.
    pub fn default() -> Self {
        Self::with_context(BuiltinContext::default())
    }

    pub fn with_context(context: BuiltinContext) -> Self {
        let mut builtins = Self {
            sync: HashMap::new(),
            async_map: HashMap::new(),
            context,
            toml_cache: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        };
        builtins.register_sync("now_ms", builtin_now_ms);
        builtins.register_sync("print", builtin_print);
        builtins.register_sync("prt", builtin_print);
        builtins.register_sync("std::io::prt", legacy_builtins::io_print);
        builtins.register_sync("io::prt", legacy_builtins::io_print);
        builtins.register_sync("legacy_io_prt", legacy_builtins::io_print);
        builtins.register_sync("std::io::print", legacy_builtins::io_print);
        builtins.register_sync("io::print", legacy_builtins::io_print);
        builtins.register_sync("legacy_io_print", legacy_builtins::io_print);
        builtins.register_sync("println", builtin_println);
        builtins.register_sync("std::io::println", legacy_builtins::io_println);
        builtins.register_sync("io::println", legacy_builtins::io_println);
        builtins.register_sync("legacy_io_println", legacy_builtins::io_println);
        builtins.register_sync("input", builtin_input);
        builtins.register_sync("std::io::input", legacy_builtins::io_input);
        builtins.register_sync("io::input", legacy_builtins::io_input);
        builtins.register_sync("legacy_io_input", legacy_builtins::io_input);
        builtins.register_sync("inp", builtin_inp);
        builtins.register_sync("std::io::inp", legacy_builtins::io_inp);
        builtins.register_sync("io::inp", legacy_builtins::io_inp);
        builtins.register_sync("legacy_io_inp", legacy_builtins::io_inp);
        builtins.register_sync("sleep", builtin_sleep);
        builtins.register_sync("std::io::sleep", legacy_builtins::io_sleep);
        builtins.register_sync("io::sleep", legacy_builtins::io_sleep);
        builtins.register_sync("legacy_io_sleep", legacy_builtins::io_sleep);
        builtins.register_sync("push", builtin_array_push);
        builtins.register_sync("len", legacy_builtins::string_len);
        builtins.register_sync("std::string::len", legacy_builtins::string_len);
        builtins.register_sync("string::len", legacy_builtins::string_len);
        builtins.register_sync("legacy_string_len", legacy_builtins::string_len);
        builtins.register_sync("to_string", legacy_builtins::string_to_string);
        builtins.register_sync("std::string::to_string", legacy_builtins::string_to_string);
        builtins.register_sync("string::to_string", legacy_builtins::string_to_string);
        builtins.register_sync("legacy_string_to_string", legacy_builtins::string_to_string);
        builtins.register_sync("parse_int", legacy_builtins::string_parse_int);
        builtins.register_sync("std::string::parse_int", legacy_builtins::string_parse_int);
        builtins.register_sync("string::parse_int", legacy_builtins::string_parse_int);
        builtins.register_sync("legacy_string_parse_int", legacy_builtins::string_parse_int);
        builtins.register_sync("parse_float", legacy_builtins::string_parse_float);
        builtins.register_sync(
            "std::string::parse_float",
            legacy_builtins::string_parse_float,
        );
        builtins.register_sync("string::parse_float", legacy_builtins::string_parse_float);
        builtins.register_sync(
            "legacy_string_parse_float",
            legacy_builtins::string_parse_float,
        );
        builtins.register_sync("sqrt", legacy_builtins::math_sqrt);
        builtins.register_sync("std::math::sqrt", legacy_builtins::math_sqrt);
        builtins.register_sync("math::sqrt", legacy_builtins::math_sqrt);
        builtins.register_sync("legacy_math_sqrt", legacy_builtins::math_sqrt);
        builtins.register_sync("pow", legacy_builtins::math_pow);
        builtins.register_sync("std::math::pow", legacy_builtins::math_pow);
        builtins.register_sync("math::pow", legacy_builtins::math_pow);
        builtins.register_sync("legacy_math_pow", legacy_builtins::math_pow);
        builtins.register_sync("sin", legacy_builtins::math_sin);
        builtins.register_sync("std::math::sin", legacy_builtins::math_sin);
        builtins.register_sync("math::sin", legacy_builtins::math_sin);
        builtins.register_sync("legacy_math_sin", legacy_builtins::math_sin);
        builtins.register_sync("cos", legacy_builtins::math_cos);
        builtins.register_sync("std::math::cos", legacy_builtins::math_cos);
        builtins.register_sync("math::cos", legacy_builtins::math_cos);
        builtins.register_sync("legacy_math_cos", legacy_builtins::math_cos);
        builtins.register_sync("abs", legacy_builtins::math_abs);
        builtins.register_sync("std::math::abs", legacy_builtins::math_abs);
        builtins.register_sync("math::abs", legacy_builtins::math_abs);
        builtins.register_sync("legacy_math_abs", legacy_builtins::math_abs);
        builtins.register_sync("min", legacy_builtins::math_min);
        builtins.register_sync("std::math::min", legacy_builtins::math_min);
        builtins.register_sync("math::min", legacy_builtins::math_min);
        builtins.register_sync("legacy_math_min", legacy_builtins::math_min);
        builtins.register_sync("max", legacy_builtins::math_max);
        builtins.register_sync("std::math::max", legacy_builtins::math_max);
        builtins.register_sync("math::max", legacy_builtins::math_max);
        builtins.register_sync("legacy_math_max", legacy_builtins::math_max);
        builtins.register_sync("core_index", builtin_core_index);
        builtins.register_sync("__slice", builtin_slice);
        builtins.register_sync("toml::load_file", builtin_toml_load_file);
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
            func(self, args)
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
            let builtins = self.clone();
            let fut = Box::pin(async move { sync_fn(&builtins, &args_clone) });
            Ok(Some(fut))
        } else {
            Err(SolvraError::Internal(format!(
                "unknown async builtin '{name}'"
            )))
        }
    }

    fn load_toml_json(&self, path: &Path) -> SolvraResult<String> {
        let metadata = fs::metadata(path).map_err(|err| {
            SolvraError::Internal(format!(
                "failed to read toml metadata {}: {err}",
                path.display()
            ))
        })?;
        let modified = metadata.modified().ok();

        // Fast path: served from cache if unchanged.
        {
            let cache = self
                .toml_cache
                .lock()
                .map_err(|_| SolvraError::Internal("toml cache lock poisoned".into()))?;
            if let Some(entry) = cache.get(path) {
                if entry.modified == modified {
                    return Ok(entry.json.clone());
                }
            }
        }

        let data = fs::read_to_string(path).map_err(|err| {
            SolvraError::Internal(format!(
                "failed to read toml file {}: {err}",
                path.display()
            ))
        })?;
        let parsed = parse_toml_value(&data, path)?;
        let json_string = serde_json::to_string(&parsed).map_err(|err| {
            SolvraError::Internal(format!("failed to serialise toml to json: {err}"))
        })?;

        let mut cache = self
            .toml_cache
            .lock()
            .map_err(|_| SolvraError::Internal("toml cache lock poisoned".into()))?;
        cache.insert(
            path.to_path_buf(),
            TomlCacheEntry {
                modified,
                json: json_string.clone(),
            },
        );
        Ok(json_string)
    }
}

fn builtin_print(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    legacy_builtins::io_print(_builtins, args)
}

fn builtin_toml_load_file(builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let path_value = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("toml::load_file expects file path".into()))?;
    let path_string = match path_value {
        Value::String(s) => s.clone(),
        other => legacy_builtins::value_to_string(other),
    };

    let resolved_path = resolve_file_path(&path_string).ok_or_else(|| {
        SolvraError::Internal(format!(
            "toml::load_file could not locate path {path_string}"
        ))
    })?;

    let canonical = canonicalize_path(&resolved_path);
    let json_string = builtins.load_toml_json(&canonical)?;
    Ok(Value::String(json_string))
}

fn builtin_core_index(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    if args.len() < 2 {
        return Err(SolvraError::Internal(
            "core_index expects collection and key".into(),
        ));
    }

    let collection = &args[0];
    let key_value = &args[1];

    match collection {
        Value::String(text) => {
            if let Ok(json) = serde_json::from_str::<JsonValue>(text) {
                let key = legacy_builtins::value_to_string(key_value);
                if let Some(found) = resolve_json_path(&json, &key) {
                    return Ok(json_to_value(found));
                }
                return Ok(Value::Null);
            }

            if let Some(index) = extract_integer(key_value) {
                if index < 0 {
                    return Ok(Value::Null);
                }
                let chars: Vec<char> = text.chars().collect();
                let ch = chars
                    .get(index as usize)
                    .map(|c| Value::String(c.to_string()))
                    .unwrap_or(Value::Null);
                return Ok(ch);
            }

            Ok(Value::Null)
        }
        Value::Null => Ok(Value::Null),
        _ => Err(SolvraError::Internal(format!(
            "core_index received unsupported collection type {}",
            collection.type_name()
        ))),
    }
}

fn builtin_slice(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    if args.len() < 4 {
        return Err(SolvraError::Internal(
            "__slice expects (target, start, end, step)".into(),
        ));
    }
    let target = &args[0];
    let start = parse_optional_index(&args[1])?;
    let end = parse_optional_index(&args[2])?;
    let step = parse_step(&args[3])?;

    match target {
        Value::Array(items) => {
            let indices = compute_slice_indices(items.len(), start, end, step)?;
            let mut out = Vec::with_capacity(indices.len());
            for i in indices {
                if let Some(val) = items.get(i).cloned() {
                    out.push(val);
                }
            }
            Ok(Value::Array(out))
        }
        Value::String(text) => {
            let chars: Vec<char> = text.chars().collect();
            let indices = compute_slice_indices(chars.len(), start, end, step)?;
            let mut out = String::new();
            for i in indices {
                if let Some(ch) = chars.get(i) {
                    out.push(*ch);
                }
            }
            Ok(Value::String(out))
        }
        _ => Err(SolvraError::Internal(
            "slice target must be array or string".into(),
        )),
    }
}

#[derive(Clone, Default)]
pub struct BuiltinContext {
    pub memory_tracker: Option<MemoryTracker>,
    pub telemetry: Option<TelemetryCollector>,
    pub async_control: Option<AsyncControl>,
}

impl Builtins {
    fn elapsed_ms(&self) -> i64 {
        let elapsed = self.start_time.elapsed().as_millis();
        let capped = elapsed.min(i64::MAX as u128);
        capped as i64
    }

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

fn builtin_println(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    legacy_builtins::io_println(_builtins, args)
}

fn builtin_input(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    legacy_builtins::io_input(_builtins, args)
}

fn builtin_inp(builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    legacy_builtins::io_inp(builtins, args)
}

fn builtin_sleep(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    legacy_builtins::io_sleep(_builtins, args)
}

fn builtin_now_ms(builtins: &Builtins, _args: &[Value]) -> SolvraResult<Value> {
    Ok(Value::Integer(builtins.elapsed_ms()))
}

fn builtin_array_push(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    if let Some(Value::Array(items)) = args.get(0) {
        let mut next = items.clone();
        let value = args.get(1).cloned().unwrap_or(Value::Null);
        next.push(value);
        Ok(Value::Array(next))
    } else {
        let type_name = args.get(0).map(|value| value.type_name()).unwrap_or("null");
        Err(SolvraError::Internal(format!(
            "push expects array as first argument, got {type_name}"
        )))
    }
}

fn resolve_json_path<'a>(value: &'a JsonValue, path: &str) -> Option<JsonValue> {
    if path.is_empty() {
        return Some(value.clone());
    }

    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        current = match current {
            JsonValue::Object(map) => map.get(segment)?,
            JsonValue::Array(items) => {
                let index: usize = segment.parse().ok()?;
                items.get(index)?
            }
            _ => return None,
        };
    }
    Some(current.clone())
}

fn json_to_value(value: JsonValue) -> Value {
    match value {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(flag) => Value::Boolean(flag),
        JsonValue::Number(num) => {
            if let Some(int) = num.as_i64() {
                Value::Integer(int)
            } else if let Some(float) = num.as_f64() {
                Value::Float(float)
            } else {
                Value::Null
            }
        }
        JsonValue::String(text) => Value::String(text),
        JsonValue::Array(_) | JsonValue::Object(_) => serde_json::to_string(&value)
            .map(Value::String)
            .unwrap_or(Value::Null),
    }
}

fn resolve_file_path(original: &str) -> Option<PathBuf> {
    let provided = PathBuf::from(original);
    if provided.exists() {
        return Some(canonicalize_path(&provided));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(root) = manifest_dir.parent() {
        let direct = root.join(original);
        if direct.exists() {
            return Some(canonicalize_path(&direct));
        }
        let solvra_ai = root.join("solvra_ai").join(original);
        if solvra_ai.exists() {
            return Some(canonicalize_path(&solvra_ai));
        }
    }

    None
}

fn canonicalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn extract_integer(value: &Value) -> Option<i64> {
    match value {
        Value::Integer(int) => Some(*int),
        _ => None,
    }
}

fn parse_optional_index(value: &Value) -> SolvraResult<Option<i64>> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    if let Some(idx) = extract_integer(value) {
        return Ok(Some(idx));
    }
    Err(SolvraError::Internal(
        "slice indices must be integers or null".into(),
    ))
}

fn parse_step(value: &Value) -> SolvraResult<i64> {
    if matches!(value, Value::Null) {
        return Ok(1);
    }
    if let Some(step) = extract_integer(value) {
        if step == 0 {
            return Err(SolvraError::Internal("slice step cannot be zero".into()));
        }
        return Ok(step);
    }
    Err(SolvraError::Internal(
        "slice step must be integer or null".into(),
    ))
}

fn compute_slice_indices(
    len: usize,
    start: Option<i64>,
    end: Option<i64>,
    step: i64,
) -> SolvraResult<Vec<usize>> {
    if step == 0 {
        return Err(SolvraError::Internal("slice step cannot be zero".into()));
    }
    let len_i = len as i64;
    let normalize = |idx: i64| if idx < 0 { idx + len_i } else { idx };

    let default_start = if step > 0 { 0 } else { len_i - 1 };
    let default_end = if step > 0 { len_i } else { -1 - len_i };

    let start = normalize(start.unwrap_or(default_start));
    let end = normalize(end.unwrap_or(default_end));

    let start = if step > 0 {
        start.clamp(0, len_i)
    } else {
        start.clamp(-1, len_i - 1)
    };
    let end = if step > 0 {
        end.clamp(0, len_i)
    } else {
        end.clamp(-1, len_i)
    };

    let mut indices = Vec::new();
    let mut idx = start;

    if step > 0 {
        while idx < end {
            if let Some(idx_usize) = usize::try_from(idx).ok() {
                indices.push(idx_usize);
            }
            idx += step;
        }
    } else {
        while idx > end {
            if (0..len_i).contains(&idx) {
                indices.push(idx as usize);
            }
            idx += step;
        }
    }
    Ok(indices)
}

fn parse_toml_value(content: &str, path: &Path) -> SolvraResult<toml::Value> {
    match toml::from_str(content) {
        Ok(value) => Ok(value),
        Err(first_err) => {
            let sanitized = sanitize_toml(content);
            toml::from_str(&sanitized).map_err(|_| {
                SolvraError::Internal(format!(
                    "failed to parse toml file {}: {first_err}",
                    path.display()
                ))
            })
        }
    }
}

fn sanitize_toml(input: &str) -> String {
    input
        .lines()
        .map(|line| sanitize_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_line(line: &str) -> String {
    if let Some(pos) = line.rfind('"') {
        let remainder = &line[pos + 1..];
        let trimmed = remainder.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            return line[..=pos].to_string();
        }
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value as JsonValue;
    use std::thread::sleep;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn toml_cache_reuses_and_refreshes() {
        let dir = tempdir().expect("create temp dir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[agents.eolas]
provider = "openai"
model = "gpt-4o-mini"
"#,
        )
        .expect("write initial toml");

        let builtins = Builtins::default();
        let arg_path = path.to_string_lossy().to_string();

        let first = builtins
            .invoke_sync("toml::load_file", &[Value::String(arg_path.clone())])
            .expect("load toml");
        let first_json: JsonValue = match first {
            Value::String(text) => serde_json::from_str(&text).expect("parse json"),
            other => panic!("expected string, got {other:?}"),
        };
        assert_eq!(first_json["agents"]["eolas"]["provider"], "openai");

        // Second call should use cache but still return same content.
        let second = builtins
            .invoke_sync("toml::load_file", &[Value::String(arg_path.clone())])
            .expect("load toml from cache");
        let second_json: JsonValue = match second {
            Value::String(text) => serde_json::from_str(&text).expect("parse json"),
            other => panic!("expected string, got {other:?}"),
        };
        assert_eq!(second_json, first_json);

        // Modify file and ensure cache refreshes.
        sleep(Duration::from_millis(20));
        fs::write(
            &path,
            r#"
[agents.eolas]
provider = "anthropic"
model = "claude"
"#,
        )
        .expect("write updated toml");

        let third = builtins
            .invoke_sync("toml::load_file", &[Value::String(arg_path)])
            .expect("reload toml after change");
        let third_json: JsonValue = match third {
            Value::String(text) => serde_json::from_str(&text).expect("parse json"),
            other => panic!("expected string, got {other:?}"),
        };
        assert_eq!(third_json["agents"]["eolas"]["provider"], "anthropic");
    }
}
