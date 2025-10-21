#![allow(dead_code)]

use crate::ast::*;
use chrono::{DateTime, Datelike, Timelike, Utc};
use rand::Rng;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, hash_map::Entry};
use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ureq::Agent;

/// Supported arity constraints for native (built-in) functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArity {
    /// The function expects exactly this many arguments.
    Exact(usize),
    /// The function accepts a range of arguments defined by the inclusive
    /// minimum and an optional maximum. `None` indicates "no upper bound".
    Range { min: usize, max: Option<usize> },
}

impl NativeArity {
    fn accepts(&self, count: usize) -> bool {
        match self {
            NativeArity::Exact(n) => *n == count,
            NativeArity::Range { min, max } => {
                if count < *min {
                    return false;
                }
                match max {
                    Some(max) => count <= *max,
                    None => true,
                }
            }
        }
    }

    fn describe(&self) -> String {
        match self {
            NativeArity::Exact(n) => format!("{}", n),
            NativeArity::Range { min, max } => match max {
                Some(max) if min == max => format!("{}", min),
                Some(max) => format!("{}..={} arguments", min, max),
                None => {
                    if *min == 0 {
                        "any number of arguments".to_string()
                    } else {
                        format!("at least {} arguments", min)
                    }
                }
            },
        }
    }
}

/// NovaScript runtime value types
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        closure: Environment,
    },
    NativeFunction {
        name: String,
        arity: NativeArity,
        func: fn(&mut Interpreter, &[Value]) -> Result<Value, RuntimeError>,
    },
    Handle(u64),
    Null,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;

        match (self, other) {
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => a == b,
            (Bool(a), Bool(b)) => a == b,
            (String(a), String(b)) => a == b,
            (Array(a), Array(b)) => a == b,
            (Object(a), Object(b)) => a == b,
            (
                Function {
                    name: name_a,
                    params: params_a,
                    body: body_a,
                    closure: closure_a,
                },
                Function {
                    name: name_b,
                    params: params_b,
                    body: body_b,
                    closure: closure_b,
                },
            ) => {
                name_a == name_b
                    && params_a == params_b
                    && body_a == body_b
                    && closure_a == closure_b
            }
            (
                NativeFunction {
                    name: name_a,
                    arity: arity_a,
                    ..
                },
                NativeFunction {
                    name: name_b,
                    arity: arity_b,
                    ..
                },
            ) => name_a == name_b && arity_a == arity_b,
            (Handle(a), Handle(b)) => a == b,
            (Null, Null) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, val) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (i, (key, val)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, val)?;
                }
                write!(f, "}}")
            }
            Value::Function { name, .. } => write!(f, "<function {}>", name),
            Value::NativeFunction { name, .. } => write!(f, "<native function {}>", name),
            Value::Handle(id) => write!(f, "<handle {}>", id),
            Value::Null => write!(f, "null"),
        }
    }
}

impl Value {
    /// Check if value is truthy (NovaScript truthiness rules)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Int(0) => false,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            _ => true,
        }
    }

    /// Get the type name of the value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function { .. } => "function",
            Value::NativeFunction { .. } => "native_function",
            Value::Handle(_) => "handle",
            Value::Null => "null",
        }
    }

    /// Convert to number if possible
    pub fn to_number(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            Value::Bool(true) => Some(1.0),
            Value::Bool(false) => Some(0.0),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    VariableNotFound(String),
    TypeError(String),
    ArgumentError(String),
    IndexError(String),
    DivisionByZero,
    StackOverflow,
    NotImplemented(String),
    IoError(String),
    NetworkError(String),
    Exit(i32),
    Return(Value),
    Break,
    Continue,
    Custom(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::VariableNotFound(name) => write!(f, "Variable '{}' not found", name),
            RuntimeError::TypeError(msg) => write!(f, "Type error: {}", msg),
            RuntimeError::ArgumentError(msg) => write!(f, "Argument error: {}", msg),
            RuntimeError::IndexError(msg) => write!(f, "Index error: {}", msg),
            RuntimeError::DivisionByZero => write!(f, "Division by zero"),
            RuntimeError::StackOverflow => write!(f, "Stack overflow"),
            RuntimeError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            RuntimeError::IoError(msg) => write!(f, "I/O error: {}", msg),
            RuntimeError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            RuntimeError::Exit(code) => write!(f, "Script requested exit with code {}", code),
            RuntimeError::Return(val) => write!(f, "Return: {}", val),
            RuntimeError::Break => write!(f, "Break statement outside loop"),
            RuntimeError::Continue => write!(f, "Continue statement outside loop"),
            RuntimeError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<io::Error> for RuntimeError {
    fn from(value: io::Error) -> Self {
        RuntimeError::IoError(value.to_string())
    }
}

impl From<ureq::Error> for RuntimeError {
    fn from(value: ureq::Error) -> Self {
        RuntimeError::NetworkError(value.to_string())
    }
}

type Environment = HashMap<String, Value>;

enum Resource {
    File(File),
}

pub struct Interpreter {
    globals: Environment,
    locals: Vec<Environment>,
    call_stack: Vec<String>,
    max_call_depth: usize,
    events: HashMap<String, Vec<Value>>,
    resources: HashMap<u64, Resource>,
    next_handle_id: u64,
    http_agent: Agent,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            globals: HashMap::new(),
            locals: Vec::new(),
            call_stack: Vec::new(),
            max_call_depth: 1000,
            events: HashMap::new(),
            resources: HashMap::new(),
            next_handle_id: 1,
            http_agent: Agent::new(),
        };
        interpreter.init_builtins();
        interpreter
    }

    fn init_builtins(&mut self) {
        self.register_builtin(
            "print",
            NativeArity::Range { min: 0, max: None },
            Interpreter::builtin_print,
        );
        self.register_builtin(
            "println",
            NativeArity::Range { min: 0, max: None },
            Interpreter::builtin_println,
        );
        self.register_builtin(
            "input",
            NativeArity::Range {
                min: 0,
                max: Some(1),
            },
            Interpreter::builtin_input,
        );
        self.register_builtin(
            "parse_int",
            NativeArity::Range {
                min: 1,
                max: Some(2),
            },
            Interpreter::builtin_parse_int,
        );
        self.register_builtin(
            "parse_float",
            NativeArity::Exact(1),
            Interpreter::builtin_parse_float,
        );
        self.register_builtin(
            "to_string",
            NativeArity::Exact(1),
            Interpreter::builtin_to_string,
        );
        self.register_builtin("len", NativeArity::Exact(1), Interpreter::builtin_len);
        self.register_builtin("type", NativeArity::Exact(1), Interpreter::builtin_type);
        self.register_builtin(
            "random",
            NativeArity::Range {
                min: 0,
                max: Some(2),
            },
            Interpreter::builtin_random,
        );
        self.register_builtin("time", NativeArity::Exact(0), Interpreter::builtin_time);
        self.register_builtin("now", NativeArity::Exact(0), Interpreter::builtin_now);
        self.register_builtin("push", NativeArity::Exact(2), Interpreter::builtin_push);
        self.register_builtin("pop", NativeArity::Exact(1), Interpreter::builtin_pop);
        self.register_builtin("insert", NativeArity::Exact(3), Interpreter::builtin_insert);
        self.register_builtin("remove", NativeArity::Exact(2), Interpreter::builtin_remove);
        self.register_builtin("sin", NativeArity::Exact(1), Interpreter::builtin_sin);
        self.register_builtin("cos", NativeArity::Exact(1), Interpreter::builtin_cos);
        self.register_builtin("tan", NativeArity::Exact(1), Interpreter::builtin_tan);
        self.register_builtin("sqrt", NativeArity::Exact(1), Interpreter::builtin_sqrt);
        self.register_builtin(
            "log",
            NativeArity::Range {
                min: 1,
                max: Some(2),
            },
            Interpreter::builtin_log,
        );
        self.register_builtin("pow", NativeArity::Exact(2), Interpreter::builtin_pow);
        self.register_builtin("abs", NativeArity::Exact(1), Interpreter::builtin_abs);
        self.register_builtin("sleep", NativeArity::Exact(1), Interpreter::builtin_sleep);
        self.register_builtin(
            "exit",
            NativeArity::Range {
                min: 0,
                max: Some(1),
            },
            Interpreter::builtin_exit,
        );
        self.register_builtin(
            "env_get",
            NativeArity::Exact(1),
            Interpreter::builtin_env_get,
        );
        self.register_builtin(
            "env_set",
            NativeArity::Exact(2),
            Interpreter::builtin_env_set,
        );
        self.register_builtin(
            "open_file",
            NativeArity::Range {
                min: 1,
                max: Some(2),
            },
            Interpreter::builtin_open_file,
        );
        self.register_builtin(
            "read_file",
            NativeArity::Exact(1),
            Interpreter::builtin_read_file,
        );
        self.register_builtin(
            "write_file",
            NativeArity::Range {
                min: 2,
                max: Some(3),
            },
            Interpreter::builtin_write_file,
        );
        self.register_builtin(
            "close_file",
            NativeArity::Exact(1),
            Interpreter::builtin_close_file,
        );
        self.register_builtin(
            "http_get",
            NativeArity::Exact(1),
            Interpreter::builtin_http_get,
        );
        self.register_builtin(
            "http_post",
            NativeArity::Range {
                min: 2,
                max: Some(3),
            },
            Interpreter::builtin_http_post,
        );
        self.register_builtin(
            "on_event",
            NativeArity::Exact(2),
            Interpreter::builtin_on_event,
        );
        self.register_builtin(
            "trigger_event",
            NativeArity::Range {
                min: 1,
                max: Some(2),
            },
            Interpreter::builtin_trigger_event,
        );
    }

    fn register_builtin(
        &mut self,
        name: &str,
        arity: NativeArity,
        func: fn(&mut Interpreter, &[Value]) -> Result<Value, RuntimeError>,
    ) {
        self.globals.insert(
            name.to_string(),
            Value::NativeFunction {
                name: name.to_string(),
                arity,
                func,
            },
        );
    }

    fn builtin_print(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let output = join_values(args);
        print!("{}", output);
        io::stdout().flush()?;
        Ok(Value::Null)
    }

    fn builtin_println(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let output = join_values(args);
        println!("{}", output);
        Ok(Value::Null)
    }

    fn builtin_input(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        if let Some(prompt) = args.first() {
            // FIX: Use `first()` to read the optional prompt without indexing and
            // rely on `Display` to avoid redundant `to_string()` allocation.
            print!("{prompt}");
            io::stdout().flush()?;
        }

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        while buffer.ends_with(['\n', '\r']) {
            buffer.pop();
        }
        Ok(Value::String(buffer))
    }

    fn builtin_parse_int(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let text = args[0].to_string();
        let radix = if let Some(base) = args.get(1) {
            match base {
                Value::Int(n) if (2..=36).contains(n) => *n as u32,
                Value::Int(_) => {
                    return Err(RuntimeError::ArgumentError(
                        "parse_int base must be between 2 and 36".to_string(),
                    ));
                }
                other => {
                    return Err(RuntimeError::TypeError(format!(
                        "parse_int base must be an integer, got {}",
                        other.type_name()
                    )));
                }
            }
        } else {
            10
        };

        let trimmed = text.trim();
        match i64::from_str_radix(trimmed, radix) {
            Ok(value) => Ok(Value::Int(value)),
            Err(_) => Err(RuntimeError::ArgumentError(format!(
                "Unable to parse '{trimmed}' as base {radix} integer"
            ))),
        }
    }

    fn builtin_parse_float(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let text = args[0].to_string();
        let trimmed = text.trim();
        match trimmed.parse::<f64>() {
            Ok(num) => Ok(Value::Float(num)),
            Err(_) => Err(RuntimeError::ArgumentError(format!(
                "Unable to parse '{trimmed}' as float"
            ))),
        }
    }

    fn builtin_to_string(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::String(args[0].to_string()))
    }

    fn builtin_len(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let length = match &args[0] {
            Value::String(s) => s.chars().count(),
            Value::Array(arr) => arr.len(),
            Value::Object(obj) => obj.len(),
            other => {
                return Err(RuntimeError::TypeError(format!(
                    "len() not supported for type {}",
                    other.type_name()
                )));
            }
        };
        Ok(Value::Int(length as i64))
    }

    fn builtin_type(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::String(args[0].type_name().to_string()))
    }

    fn builtin_random(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let mut rng = rand::thread_rng();
        match args.len() {
            0 => Ok(Value::Float(rng.r#gen::<f64>())),
            1 => {
                let upper = args[0].to_number().ok_or_else(|| {
                    RuntimeError::TypeError("random upper bound must be numeric".into())
                })?;
                if matches!(args[0], Value::Int(_)) {
                    if upper <= 0.0 {
                        return Err(RuntimeError::ArgumentError(
                            "random upper bound must be positive".into(),
                        ));
                    }
                    Ok(Value::Int(rng.gen_range(0..upper as i64)))
                } else {
                    Ok(Value::Float(rng.r#gen::<f64>() * upper))
                }
            }
            2 => {
                let min = args[0].to_number().ok_or_else(|| {
                    RuntimeError::TypeError("random range start must be numeric".into())
                })?;
                let max = args[1].to_number().ok_or_else(|| {
                    RuntimeError::TypeError("random range end must be numeric".into())
                })?;
                if max < min {
                    return Err(RuntimeError::ArgumentError(
                        "random range end must be >= start".into(),
                    ));
                }
                if matches!(args[0], Value::Int(_)) && matches!(args[1], Value::Int(_)) {
                    Ok(Value::Int(rng.gen_range(min as i64..=max as i64)))
                } else {
                    Ok(Value::Float(rng.gen_range(min..=max)))
                }
            }
            _ => unreachable!(),
        }
    }

    fn builtin_time(&mut self, _args: &[Value]) -> Result<Value, RuntimeError> {
        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .map_err(|err| RuntimeError::Custom(err.to_string()))?;
        Ok(Value::Float(duration.as_secs_f64()))
    }

    fn builtin_now(&mut self, _args: &[Value]) -> Result<Value, RuntimeError> {
        let now: DateTime<Utc> = Utc::now();
        let mut map = HashMap::new();
        map.insert("iso".to_string(), Value::String(now.to_rfc3339()));
        map.insert(
            "timestamp".to_string(),
            Value::Float(now.timestamp_millis() as f64 / 1000.0),
        );
        map.insert("year".to_string(), Value::Int(now.year() as i64));
        map.insert("month".to_string(), Value::Int(now.month() as i64));
        map.insert("day".to_string(), Value::Int(now.day() as i64));
        map.insert("hour".to_string(), Value::Int(now.hour() as i64));
        map.insert("minute".to_string(), Value::Int(now.minute() as i64));
        map.insert("second".to_string(), Value::Int(now.second() as i64));
        map.insert(
            "nanosecond".to_string(),
            Value::Int(now.nanosecond() as i64),
        );
        Ok(Value::Object(map))
    }

    fn builtin_push(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        match &args[0] {
            Value::Array(items) => {
                let mut next = items.clone();
                next.push(args[1].clone());
                Ok(Value::Array(next))
            }
            other => Err(RuntimeError::TypeError(format!(
                "push expects array as first argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn builtin_pop(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        match &args[0] {
            Value::Array(items) => {
                if items.is_empty() {
                    return Ok(Value::Object(HashMap::from([
                        ("array".to_string(), Value::Array(Vec::new())),
                        ("value".to_string(), Value::Null),
                    ])));
                }
                let mut next = items.clone();
                let value = next.pop().unwrap_or(Value::Null);
                Ok(Value::Object(HashMap::from([
                    ("array".to_string(), Value::Array(next)),
                    ("value".to_string(), value),
                ])))
            }
            other => Err(RuntimeError::TypeError(format!(
                "pop expects array as first argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn builtin_insert(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let index = expect_index(&args[1])?;
        match &args[0] {
            Value::Array(items) => {
                if index > items.len() {
                    return Err(RuntimeError::IndexError(format!(
                        "insert index {} out of bounds (len {})",
                        index,
                        items.len()
                    )));
                }
                let mut next = items.clone();
                next.insert(index, args[2].clone());
                Ok(Value::Array(next))
            }
            other => Err(RuntimeError::TypeError(format!(
                "insert expects array as first argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn builtin_remove(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let index = expect_index(&args[1])?;
        match &args[0] {
            Value::Array(items) => {
                if index >= items.len() {
                    return Err(RuntimeError::IndexError(format!(
                        "remove index {} out of bounds (len {})",
                        index,
                        items.len()
                    )));
                }
                let mut next = items.clone();
                let removed = next.remove(index);
                Ok(Value::Object(HashMap::from([
                    ("array".to_string(), Value::Array(next)),
                    ("value".to_string(), removed),
                ])))
            }
            other => Err(RuntimeError::TypeError(format!(
                "remove expects array as first argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub fn eval_program(&mut self, program: &Program) -> Result<Option<Value>, RuntimeError> {
        let mut last = None;
        for stmt in &program.statements {
            match self.eval_stmt(stmt) {
                Ok(val) => last = val,
                Err(RuntimeError::Return(val)) => return Ok(Some(val)),
                Err(e) => return Err(e),
            }
        }
        Ok(last)
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, RuntimeError> {
        match stmt {
            Stmt::VariableDecl { decl } => {
                let val = if let Some(expr) = &decl.initializer {
                    self.eval_expr(expr)?
                } else {
                    Value::Null
                };
                self.set_variable(decl.name.clone(), val);
                Ok(None)
            }

            Stmt::Expression { expr, .. } => {
                let v = self.eval_expr(expr)?;
                Ok(Some(v))
            }

            Stmt::Return { value, .. } => {
                let v = if let Some(expr) = value {
                    self.eval_expr(expr)?
                } else {
                    Value::Null
                };
                Err(RuntimeError::Return(v))
            }

            Stmt::Break { .. } => Err(RuntimeError::Break),
            Stmt::Continue { .. } => Err(RuntimeError::Continue),

            Stmt::Block { statements, .. } => {
                self.push_scope();
                let mut result = Ok(None);
                for stmt in statements {
                    match self.eval_stmt(stmt) {
                        Ok(val) => result = Ok(val),
                        Err(RuntimeError::Break) | Err(RuntimeError::Continue) => {
                            result = Err(RuntimeError::Break);
                            break;
                        }
                        Err(RuntimeError::Return(val)) => {
                            result = Err(RuntimeError::Return(val));
                            break;
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
                self.pop_scope();
                result
            }

            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    self.eval_stmt(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.eval_stmt(else_stmt)
                } else {
                    Ok(None)
                }
            }

            Stmt::While {
                condition, body, ..
            } => {
                let mut result = Ok(None);
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if !cond_val.is_truthy() {
                        break;
                    }

                    match self.eval_stmt(body) {
                        Ok(val) => result = Ok(val),
                        Err(RuntimeError::Break) => break,
                        Err(RuntimeError::Continue) => continue,
                        Err(RuntimeError::Return(val)) => return Err(RuntimeError::Return(val)),
                        Err(e) => return Err(e),
                    }
                }
                result
            }

            Stmt::For {
                variable,
                iterable,
                body,
                ..
            } => {
                let iterable_val = self.eval_expr(iterable)?;
                match iterable_val {
                    Value::Array(elements) => {
                        let mut result = Ok(None);
                        for item in elements {
                            self.push_scope();
                            self.set_variable(variable.clone(), item);
                            match self.eval_stmt(body) {
                                Ok(val) => result = Ok(val),
                                Err(RuntimeError::Break) => {
                                    self.pop_scope();
                                    break;
                                }
                                Err(RuntimeError::Continue) => {
                                    self.pop_scope();
                                    continue;
                                }
                                Err(RuntimeError::Return(v)) => {
                                    self.pop_scope();
                                    return Err(RuntimeError::Return(v));
                                }
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            }
                            self.pop_scope();
                        }
                        result
                    }
                    _ => Err(RuntimeError::TypeError(format!(
                        "Value of type '{}' is not iterable",
                        iterable_val.type_name()
                    ))),
                }
            }

            Stmt::FunctionDecl { decl } => {
                let func = Value::Function {
                    name: decl.name.clone(),
                    params: decl.params.iter().map(|p| p.name.clone()).collect(),
                    body: decl.body.clone(),
                    closure: self.capture_environment(),
                };
                self.set_variable(decl.name.clone(), func);
                Ok(None)
            }

            other => Err(RuntimeError::NotImplemented(format!(
                "Statement: {:?}",
                other
            ))),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal { value, .. } => self.eval_literal(value),
            Expr::Identifier { name, .. } => self
                .get_variable(name)
                .ok_or_else(|| RuntimeError::VariableNotFound(name.clone())),
            Expr::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.eval_binary_op(operator, l, r)
            }
            Expr::Unary {
                operator, operand, ..
            } => {
                let v = self.eval_expr(operand)?;
                self.eval_unary_op(operator, v)
            }
            Expr::StringTemplate { parts, .. } => self.eval_string_template(parts),

            Expr::Assignment { target, value, .. } => {
                if let Expr::Identifier { name, .. } = &**target {
                    let val = self.eval_expr(value)?;
                    self.set_variable(name.clone(), val.clone());
                    Ok(val)
                } else {
                    Err(RuntimeError::TypeError(
                        "Invalid assignment target".to_string(),
                    ))
                }
            }
            Expr::Call { callee, args, .. } => {
                let func = self.eval_expr(callee)?;
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.eval_expr(arg)?);
                }
                self.call_function(func, arg_values)
            }
            Expr::Index { object, index, .. } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.eval_index_access(obj, idx)
            }
            Expr::Member {
                object, property, ..
            } => {
                let obj = self.eval_expr(object)?;
                match obj {
                    Value::Object(map) => Ok(map.get(property).cloned().unwrap_or(Value::Null)),
                    _ => Err(RuntimeError::TypeError(format!(
                        "Cannot access property '{}' on {}",
                        property,
                        obj.type_name()
                    ))),
                }
            }

            other => Err(RuntimeError::NotImplemented(format!(
                "Expression: {:?}",
                other
            ))),
        }
    }

    fn eval_literal(&mut self, lit: &Literal) -> Result<Value, RuntimeError> {
        match lit {
            Literal::Integer(n) => Ok(Value::Int(*n)),
            Literal::Float(f) => Ok(Value::Float(*f)),
            Literal::Boolean(b) => Ok(Value::Bool(*b)),
            Literal::String(s) => Ok(Value::String(s.clone())),
            Literal::Null => Ok(Value::Null),

            Literal::Array(arr) => {
                let mut values = Vec::new();
                for expr in arr {
                    values.push(self.eval_expr(expr)?);
                }
                Ok(Value::Array(values))
            }

            Literal::Object(props) => {
                let mut map = HashMap::new();
                for (key, expr) in props {
                    let value = self.eval_expr(expr)?;
                    map.insert(key.clone(), value);
                }
                Ok(Value::Object(map))
            }
        }
    }

    fn builtin_sin(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::Float(expect_number(&args[0], "sin")?.sin()))
    }

    fn builtin_cos(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::Float(expect_number(&args[0], "cos")?.cos()))
    }

    fn builtin_tan(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::Float(expect_number(&args[0], "tan")?.tan()))
    }

    fn builtin_sqrt(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let value = expect_number(&args[0], "sqrt")?;
        if value < 0.0 {
            return Err(RuntimeError::ArgumentError(
                "sqrt expects non-negative input".into(),
            ));
        }
        Ok(Value::Float(value.sqrt()))
    }

    fn builtin_log(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let value = expect_number(&args[0], "log")?;
        if value <= 0.0 {
            return Err(RuntimeError::ArgumentError(
                "log expects positive input".into(),
            ));
        }
        let base = if let Some(b) = args.get(1) {
            let base_val = expect_number(b, "log")?;
            if base_val <= 0.0 || (base_val - 1.0).abs() < f64::EPSILON {
                return Err(RuntimeError::ArgumentError(
                    "log base must be positive and not equal to 1".into(),
                ));
            }
            base_val
        } else {
            std::f64::consts::E
        };
        Ok(Value::Float(value.log(base)))
    }

    fn builtin_pow(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let base = expect_number(&args[0], "pow")?;
        let exponent = expect_number(&args[1], "pow")?;
        Ok(Value::Float(base.powf(exponent)))
    }

    fn builtin_abs(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        match &args[0] {
            Value::Int(n) => Ok(Value::Int(n.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            other => Err(RuntimeError::TypeError(format!(
                "abs expects numeric argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn builtin_sleep(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let millis = expect_number(&args[0], "sleep")?;
        if millis < 0.0 {
            return Err(RuntimeError::ArgumentError(
                "sleep duration must be non-negative".into(),
            ));
        }
        thread::sleep(Duration::from_secs_f64(millis / 1000.0));
        Ok(Value::Null)
    }

    fn builtin_exit(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let code = if args.is_empty() {
            0
        } else {
            match &args[0] {
                Value::Int(n) => *n as i32,
                other => {
                    return Err(RuntimeError::TypeError(format!(
                        "exit code must be integer, got {}",
                        other.type_name()
                    )));
                }
            }
        };
        Err(RuntimeError::Exit(code))
    }

    fn builtin_env_get(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let key = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("env_get expects string key".into()))?;
        match env::var(key) {
            Ok(value) => Ok(Value::String(value)),
            Err(_) => Ok(Value::Null),
        }
    }

    fn builtin_env_set(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let key = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("env_set expects string key".into()))?;
        // SAFETY: Rust 2024 marks environment mutation as unsafe because it
        // interacts with global process state. NovaScript explicitly opts in
        // here to provide scripting access.
        unsafe {
            env::set_var(key, args[1].to_string());
        }
        Ok(Value::Null)
    }

    fn builtin_open_file(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let path = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("open_file expects string path".into()))?;
        let mode = args.get(1).and_then(Value::as_str).unwrap_or("r");

        let file = match mode {
            "r" => File::open(path)?,
            "w" => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?,
            "a" => OpenOptions::new().append(true).create(true).open(path)?,
            "rw" | "wr" => {
                if !Path::new(path).exists() {
                    // FIX: Create the file explicitly so we can open it without
                    // calling `create(true)`, satisfying Clippy's requirement to
                    // choose between append/truncate when materialising files.
                    File::create(path)?;
                }
                OpenOptions::new().read(true).write(true).open(path)?
            }
            other => {
                return Err(RuntimeError::ArgumentError(format!(
                    "Unsupported file mode '{other}'"
                )));
            }
        };

        Ok(self.allocate_handle(Resource::File(file)))
    }

    fn builtin_read_file(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        match &args[0] {
            Value::String(path) => Ok(Value::String(fs::read_to_string(path)?)),
            Value::Handle(id) => {
                let file = self.get_file_mut(*id)?;
                file.seek(SeekFrom::Start(0))?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                Ok(Value::String(contents))
            }
            other => Err(RuntimeError::TypeError(format!(
                "read_file expects path string or handle, got {}",
                other.type_name()
            ))),
        }
    }

    fn builtin_write_file(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let content = args[1].to_string();
        if let Value::String(path) = &args[0] {
            let append = args
                .get(2)
                .map(|v| matches!(v, Value::Bool(true)))
                .unwrap_or(false);
            if append {
                let mut file = OpenOptions::new().append(true).create(true).open(path)?;
                file.write_all(content.as_bytes())?;
                file.flush()?;
            } else {
                fs::write(path, content)?;
            }
            Ok(Value::Null)
        } else if let Value::Handle(id) = &args[0] {
            let file = self.get_file_mut(*id)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            Ok(Value::Null)
        } else {
            Err(RuntimeError::TypeError(
                "write_file expects string path or handle".into(),
            ))
        }
    }

    fn builtin_close_file(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let id = expect_handle_id(&args[0])?;
        if self.resources.remove(&id).is_none() {
            return Err(RuntimeError::ArgumentError(format!(
                "Unknown file handle {}",
                id
            )));
        }
        Ok(Value::Null)
    }

    fn builtin_http_get(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let url = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("http_get expects string URL".into()))?;
        let response = self.http_agent.get(url).call()?;
        value_from_http_response(response)
    }

    fn builtin_http_post(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let url = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("http_post expects string URL".into()))?;
        let mut request = self.http_agent.post(url);

        if let Some(headers) = args.get(2) {
            if let Value::Object(map) = headers {
                for (key, value) in map {
                    request = request.set(key, &value.to_string());
                }
            } else {
                return Err(RuntimeError::TypeError(
                    "http_post headers must be an object".into(),
                ));
            }
        }

        let response = match &args[1] {
            Value::Object(_) | Value::Array(_) => {
                let json = value_to_json(&args[1]);
                request.send_json(json)?
            }
            Value::String(body) => request.send_string(body)?,
            other => request.send_string(&other.to_string())?,
        };
        value_from_http_response(response)
    }

    fn builtin_on_event(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let event = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("on_event expects string event name".into()))?;
        match &args[1] {
            Value::Function { .. } | Value::NativeFunction { .. } => {
                self.events
                    .entry(event.to_string())
                    .or_default()
                    .push(args[1].clone());
                Ok(Value::Null)
            }
            other => Err(RuntimeError::TypeError(format!(
                "on_event expects callable handler, got {}",
                other.type_name()
            ))),
        }
    }

    fn builtin_trigger_event(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let event = args[0].as_str().ok_or_else(|| {
            RuntimeError::TypeError("trigger_event expects string event name".into())
        })?;
        let payload = args.get(1).cloned().unwrap_or(Value::Null);
        let handlers = self.events.get(event).cloned().unwrap_or_default();
        let mut executed = 0;
        for handler in handlers {
            self.call_function(handler, vec![payload.clone()])?;
            executed += 1;
        }
        Ok(Value::Int(executed))
    }

    fn allocate_handle(&mut self, resource: Resource) -> Value {
        let id = self.next_handle_id;
        self.next_handle_id += 1;
        self.resources.insert(id, resource);
        Value::Handle(id)
    }

    fn get_file_mut(&mut self, id: u64) -> Result<&mut File, RuntimeError> {
        match self.resources.get_mut(&id) {
            Some(Resource::File(file)) => Ok(file),
            None => Err(RuntimeError::ArgumentError(format!(
                "Unknown file handle {}",
                id
            ))),
        }
    }

    fn eval_string_template(&mut self, parts: &[StringPart]) -> Result<Value, RuntimeError> {
        let mut result = String::new();
        for part in parts {
            match part {
                StringPart::Literal(text) => result.push_str(text),
                StringPart::Expression(expr) => {
                    let value = self.eval_expr(expr)?;
                    result.push_str(&value.to_string());
                }
            }
        }
        Ok(Value::String(result))
    }

    fn eval_binary_op(
        &self,
        op: &BinaryOp,
        left: Value,
        right: Value,
    ) -> Result<Value, RuntimeError> {
        use BinaryOp::*;
        use Value::*;
        match op {
            Add => match (left, right) {
                (Int(a), Int(b)) => Ok(Int(a + b)),
                (Float(a), Float(b)) => Ok(Float(a + b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 + b)),
                (Float(a), Int(b)) => Ok(Float(a + b as f64)),
                (String(a), String(b)) => Ok(String(a + &b)),
                (String(a), b) => Ok(String(a + &b.to_string())),
                (a, String(b)) => Ok(String(a.to_string() + &b)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Add not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            Subtract => match (left, right) {
                (Int(a), Int(b)) => Ok(Int(a - b)),
                (Float(a), Float(b)) => Ok(Float(a - b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 - b)),
                (Float(a), Int(b)) => Ok(Float(a - b as f64)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Subtract not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            Multiply => match (left, right) {
                (Int(a), Int(b)) => Ok(Int(a * b)),
                (Float(a), Float(b)) => Ok(Float(a * b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 * b)),
                (Float(a), Int(b)) => Ok(Float(a * b as f64)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Multiply not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            Divide => match (left, right) {
                (Int(_), Int(0))
                | (Float(_), Float(0.0))
                | (Int(_), Float(0.0))
                | (Float(_), Int(0)) => Err(RuntimeError::DivisionByZero),
                (Int(a), Int(b)) => Ok(Int(a / b)),
                (Float(a), Float(b)) => Ok(Float(a / b)),
                (Int(a), Float(b)) => Ok(Float(a as f64 / b)),
                (Float(a), Int(b)) => Ok(Float(a / b as f64)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Divide not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            Modulo => match (left, right) {
                (Int(_), Int(0)) => Err(RuntimeError::DivisionByZero),
                (Int(a), Int(b)) => Ok(Int(a % b)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Modulo not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            Equal => Ok(Value::Bool(left == right)),
            NotEqual => Ok(Value::Bool(left != right)),
            Less => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a < b)),
                (Float(a), Float(b)) => Ok(Bool(a < b)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Less not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            Greater => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a > b)),
                (Float(a), Float(b)) => Ok(Bool(a > b)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "Greater not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            LessEqual => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a <= b)),
                (Float(a), Float(b)) => Ok(Bool(a <= b)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "LessEqual not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            GreaterEqual => match (left, right) {
                (Int(a), Int(b)) => Ok(Bool(a >= b)),
                (Float(a), Float(b)) => Ok(Bool(a >= b)),
                (a, b) => Err(RuntimeError::TypeError(format!(
                    "GreaterEqual not supported for {} and {}",
                    a.type_name(),
                    b.type_name()
                ))),
            },
            And => Ok(Bool(left.is_truthy() && right.is_truthy())),
            Or => Ok(Bool(left.is_truthy() || right.is_truthy())),
            _ => Err(RuntimeError::NotImplemented(format!(
                "Operator {:?} not implemented",
                op
            ))),
        }
    }

    fn eval_unary_op(&self, op: &UnaryOp, operand: Value) -> Result<Value, RuntimeError> {
        use UnaryOp::*;
        use Value::*;

        match (op, operand) {
            (Minus, Int(n)) => Ok(Int(-n)),
            (Minus, Float(f)) => Ok(Float(-f)),
            (Not, val) => Ok(Bool(!val.is_truthy())),
            (_, val) => Err(RuntimeError::TypeError(format!(
                "Unary operation {:?} not supported for {}",
                op,
                val.type_name()
            ))),
        }
    }

    fn eval_index_access(&self, object: Value, index: Value) -> Result<Value, RuntimeError> {
        match (object, index) {
            (Value::Array(arr), Value::Int(idx)) => {
                let i = if idx < 0 { arr.len() as i64 + idx } else { idx };

                if i < 0 || i >= arr.len() as i64 {
                    Err(RuntimeError::IndexError(format!(
                        "Index {} out of bounds",
                        idx
                    )))
                } else {
                    Ok(arr[i as usize].clone())
                }
            }
            (Value::Object(obj), Value::String(key)) => {
                Ok(obj.get(&key).cloned().unwrap_or(Value::Null))
            }
            (Value::String(s), Value::Int(idx)) => {
                let chars: Vec<char> = s.chars().collect();
                let i = if idx < 0 {
                    chars.len() as i64 + idx
                } else {
                    idx
                };

                if i < 0 || i >= chars.len() as i64 {
                    Err(RuntimeError::IndexError(format!(
                        "Index {} out of bounds",
                        idx
                    )))
                } else {
                    Ok(Value::String(chars[i as usize].to_string()))
                }
            }
            (obj, idx) => Err(RuntimeError::TypeError(format!(
                "Cannot index {} with {}",
                obj.type_name(),
                idx.type_name()
            ))),
        }
    }

    fn call_function(&mut self, func: Value, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if self.call_stack.len() >= self.max_call_depth {
            return Err(RuntimeError::StackOverflow);
        }

        match func {
            Value::NativeFunction { name, arity, func } => {
                if !arity.accepts(args.len()) {
                    return Err(RuntimeError::ArgumentError(format!(
                        "Function '{}' expects {}, got {}",
                        name,
                        arity.describe(),
                        args.len()
                    )));
                }
                func(self, &args)
            }

            Value::Function {
                name,
                params,
                body,
                closure: _,
            } => {
                if args.len() != params.len() {
                    return Err(RuntimeError::ArgumentError(format!(
                        "Function '{}' expects {} arguments, got {}",
                        name,
                        params.len(),
                        args.len()
                    )));
                }

                self.call_stack.push(name.clone());

                // Create new scope with closure and parameters
                self.push_scope();

                // Bind parameters to arguments
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.set_variable(param.clone(), arg.clone());
                }

                // Execute function body
                let mut result = Value::Null;
                for stmt in &body {
                    match self.eval_stmt(stmt) {
                        Ok(_) => {}
                        Err(RuntimeError::Return(val)) => {
                            result = val;
                            break;
                        }
                        Err(e) => {
                            self.pop_scope();
                            self.call_stack.pop();
                            return Err(e);
                        }
                    }
                }

                self.pop_scope();
                self.call_stack.pop();
                Ok(result)
            }

            _ => Err(RuntimeError::TypeError(format!(
                "Value of type '{}' is not callable",
                func.type_name()
            ))),
        }
    }

    // Environment management
    fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    fn set_variable(&mut self, name: String, value: Value) {
        if let Some(scope) = self
            .locals
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(&name))
        {
            scope.insert(name, value);
            return;
        }

        if let Entry::Occupied(mut entry) = self.globals.entry(name.clone()) {
            entry.insert(value);
            return;
        }

        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name, value);
        } else {
            self.globals.insert(name, value);
        }
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        // Search local scopes from innermost to outermost
        for scope in self.locals.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        // Search global scope
        self.globals.get(name).cloned()
    }

    fn capture_environment(&self) -> Environment {
        // For now, just capture globals. In a full implementation,
        // you'd capture the current scope chain.
        self.globals.clone()
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

fn join_values(args: &[Value]) -> String {
    args.iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn expect_number(value: &Value, name: &str) -> Result<f64, RuntimeError> {
    value.to_number().ok_or_else(|| {
        RuntimeError::TypeError(format!(
            "{name} expects numeric argument, got {}",
            value.type_name()
        ))
    })
}

fn expect_index(value: &Value) -> Result<usize, RuntimeError> {
    match value {
        Value::Int(n) if *n >= 0 => Ok(*n as usize),
        other => Err(RuntimeError::TypeError(format!(
            "Index must be non-negative integer, got {}",
            other.type_name()
        ))),
    }
}

fn expect_handle_id(value: &Value) -> Result<u64, RuntimeError> {
    match value {
        Value::Handle(id) => Ok(*id),
        other => Err(RuntimeError::TypeError(format!(
            "Expected handle, got {}",
            other.type_name()
        ))),
    }
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Null => JsonValue::Null,
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Int(i) => JsonValue::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Array(values) => JsonValue::Array(values.iter().map(value_to_json).collect()),
        Value::Object(map) => {
            let mut json_map = serde_json::Map::new();
            for (key, val) in map {
                json_map.insert(key.clone(), value_to_json(val));
            }
            JsonValue::Object(json_map)
        }
        Value::Function { name, .. } => JsonValue::String(format!("<function {name}>",)),
        Value::NativeFunction { name, .. } => JsonValue::String(format!("<native {name}>",)),
        Value::Handle(id) => JsonValue::Number((*id).into()),
    }
}

fn json_to_value(json: &JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Number(num) => {
            if let Some(i) = num.as_i64() {
                Value::Int(i)
            } else if let Some(u) = num.as_u64() {
                Value::Int(u as i64)
            } else {
                Value::Float(num.as_f64().unwrap_or_default())
            }
        }
        JsonValue::String(s) => Value::String(s.clone()),
        JsonValue::Array(arr) => Value::Array(arr.iter().map(json_to_value).collect()),
        JsonValue::Object(map) => {
            let mut object = HashMap::new();
            for (key, val) in map {
                object.insert(key.clone(), json_to_value(val));
            }
            Value::Object(object)
        }
    }
}

fn value_from_http_response(response: ureq::Response) -> Result<Value, RuntimeError> {
    let content_type = response.header("Content-Type").map(|s| s.to_owned());
    let body = response.into_string()?;
    if content_type
        .as_deref()
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false)
    {
        match serde_json::from_str::<JsonValue>(&body) {
            Ok(json) => return Ok(json_to_value(&json)),
            Err(err) => {
                return Err(RuntimeError::NetworkError(format!(
                    "Failed to parse JSON response: {}",
                    err
                )));
            }
        }
    }
    Ok(Value::String(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Null.to_string(), "null");
    }

    #[test]
    fn test_value_truthiness() {
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
    }

    #[test]
    fn test_interpreter_creation() {
        let interpreter = Interpreter::new();
        assert!(!interpreter.globals.is_empty()); // Should have builtins
    }

    fn call_builtin(interpreter: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
        let func = interpreter
            .globals
            .get(name)
            .cloned()
            .expect("builtin not found");
        interpreter.call_function(func, args).expect("builtin call")
    }

    #[test]
    fn test_parse_int_builtin() {
        let mut interpreter = Interpreter::new();
        let result = call_builtin(
            &mut interpreter,
            "parse_int",
            vec![Value::String("123".into())],
        );
        assert_eq!(result, Value::Int(123));
    }

    #[test]
    fn test_push_and_pop_builtin() {
        let mut interpreter = Interpreter::new();
        let array = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let pushed = call_builtin(&mut interpreter, "push", vec![array.clone(), Value::Int(3)]);
        assert_eq!(
            pushed,
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );

        let popped = call_builtin(&mut interpreter, "pop", vec![pushed]);
        if let Value::Object(map) = popped {
            assert!(matches!(map.get("value"), Some(Value::Int(3))));
        } else {
            panic!("expected object from pop");
        }
    }

    static EVENT_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn event_handler(_: &mut Interpreter, args: &[Value]) -> Result<Value, RuntimeError> {
        EVENT_CALLS.fetch_add(1, Ordering::SeqCst);
        assert_eq!(args.len(), 1);
        assert_eq!(args[0], Value::Int(42));
        Ok(Value::Null)
    }

    #[test]
    fn test_event_triggering() {
        EVENT_CALLS.store(0, Ordering::SeqCst);
        let mut interpreter = Interpreter::new();
        let handler = Value::NativeFunction {
            name: "test_handler".into(),
            arity: NativeArity::Exact(1),
            func: event_handler,
        };
        call_builtin(
            &mut interpreter,
            "on_event",
            vec![Value::String("tick".into()), handler],
        );
        let triggered = call_builtin(
            &mut interpreter,
            "trigger_event",
            vec![Value::String("tick".into()), Value::Int(42)],
        );
        assert_eq!(triggered, Value::Int(1));
        assert_eq!(EVENT_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_file_roundtrip() {
        let mut interpreter = Interpreter::new();
        let mut path = PathBuf::from(env::temp_dir());
        path.push("novascript_test.txt");
        let path_value = Value::String(path.to_string_lossy().to_string());
        call_builtin(
            &mut interpreter,
            "write_file",
            vec![path_value.clone(), Value::String("hello".into())],
        );
        let read = call_builtin(&mut interpreter, "read_file", vec![path_value.clone()]);
        assert_eq!(read, Value::String("hello".into()));
        let _ = fs::remove_file(&path);
    }
}
