//=============================================
// nova_script/interpreter.rs
//=============================================
// Author: NovaOS Contributors
// License: MIT (see LICENSE)
// Goal: NovaScript runtime interpreter implementation
// Objective: Execute parsed programs against NovaCore HAL and module system
// Formatting: Zobie.format (.novaformat)
//=============================================

//=============================================
// Section 1: Crate Attributes & Imports
//=============================================

#![allow(dead_code)]

use crate::ast::*;
use crate::modules::{ModuleArtifact, ModuleDescriptor, ModuleError, ModuleLoader};
use dirs::home_dir;
use chrono::{DateTime, Datelike, Timelike, Utc};
use rand::Rng;
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ureq::Agent;

use nova_core::backend;
use nova_core::integration::RuntimeHooks as NovaRuntimeHooks;
use nova_core::sys::hal::{
    DeviceCapability, DeviceInfo, DeviceKind, HardwareAbstractionLayer, SensorKind, SoftwareHal,
    StorageBus,
};

//=============================================/*
//  Collects crate attributes and imports required by the NovaScript interpreter runtime.
//============================================*/
//=============================================
// Section 2: Native Function Arity
//=============================================

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

//=============================================/*
//  Provides helper utilities to validate and describe acceptable native call arity.
//============================================*/
//=============================================
//            Section 3: Runtime Values
//=============================================

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
    //Function: is_truthy
    //Purpose: Evaluate NovaScript truthiness semantics for conditional flow
    //Inputs: &self
    //Returns: bool
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
    //Function: type_name
    //Purpose: Report human-readable name for the underlying runtime variant
    //Inputs: &self
    //Returns: &'static str
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
    //Function: to_number
    //Purpose: Attempt numeric coercion for arithmetic contexts
    //Inputs: &self
    //Returns: Option<f64>
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

    //Function: as_str
    //Purpose: Provide borrowed string slice for string-like values
    //Inputs: &self
    //Returns: Option<&str>
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

//=============================================/*
//  Encapsulates NovaScript runtime values and helper methods for type coercion.
//============================================*/
//=============================================
//            Section 4: Runtime Errors
//=============================================

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

impl From<ModuleError> for RuntimeError {
    fn from(value: ModuleError) -> Self {
        RuntimeError::Custom(value.to_string())
    }
}

//=============================================/*
//  Describes NovaScript runtime errors and conversion helpers from system layers.
//============================================*/
//=============================================
//            Section 5: Environments & Interpreter State
//=============================================

#[derive(Debug, Clone, PartialEq)]
pub struct VariableEntry {
    value: Value,
    mutable: bool,
}

type Environment = HashMap<String, VariableEntry>;

enum Resource {
    File(File),
}

type SharedModuleLoader = Rc<RefCell<ModuleLoader>>;

pub struct Interpreter {
    globals: Environment,
    locals: Vec<Environment>,
    call_stack: Vec<String>,
    max_call_depth: usize,
    events: HashMap<String, Vec<Value>>,
    resources: HashMap<u64, Resource>,
    next_handle_id: u64,
    http_agent: Agent,
    module_loader: SharedModuleLoader,
    module_path_stack: Vec<PathBuf>,
    hal: Arc<dyn HardwareAbstractionLayer>,
    dry_run: bool,
}

//=============================================/*
//  Defines environment storage, resource tracking, and core interpreter state.
//============================================*/
//=============================================
//            Section 6: Interpreter Construction & Builtins
//=============================================

impl Interpreter {
    //Function: new
    //Purpose: Construct interpreter with default module loader and HAL
    //Inputs: None
    //Returns: Self
    pub fn new() -> Self {
        let loader = Rc::new(RefCell::new(ModuleLoader::new()));
        let hooks = Arc::new(NovaRuntimeHooks::default());
        let hal_impl = SoftwareHal::new(backend::active_target(), hooks);
        let _ = hal_impl.register_builtin_devices();
        let hal: Arc<dyn HardwareAbstractionLayer> = Arc::new(hal_impl);
        Self::with_loader(loader, hal)
    }

    //Function: with_loader
    //Purpose: Build interpreter with injected module loader and HAL implementation
    //Inputs: loader: SharedModuleLoader, hal: Arc<dyn HardwareAbstractionLayer>
    //Returns: Self
    pub fn with_loader(loader: SharedModuleLoader, hal: Arc<dyn HardwareAbstractionLayer>) -> Self {
        let mut interpreter = Self {
            globals: HashMap::new(),
            locals: Vec::new(),
            call_stack: Vec::new(),
            max_call_depth: 1000,
            events: HashMap::new(),
            resources: HashMap::new(),
            next_handle_id: 1,
            http_agent: Agent::new(),
            module_loader: loader,
            module_path_stack: Vec::new(),
            hal,
            dry_run: false,
        };
        interpreter.init_builtins();
        interpreter
    }

    //Function: set_dry_run
    //Purpose: Toggle dry-run mode to skip side-effectful operations (spawns).
    //Inputs: &mut self, enabled: bool
    //Returns: ()
    pub fn set_dry_run(&mut self, enabled: bool) {
        self.dry_run = enabled;
    }

    //Function: add_module_search_path
    //Purpose: Allow callers to extend module resolution with additional directories.
    //Inputs: &mut self, path: Into<PathBuf>
    //Returns: ()
    pub fn add_module_search_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.module_loader.borrow_mut().add_script_path(path.into());
    }

    //Function: eval_expression
    //Purpose: Evaluate a single expression within the current interpreter context.
    //Inputs: &mut self, expr: &Expr
    //Returns: Result<Value, RuntimeError>
    pub fn eval_expression(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        self.eval_expr(expr)
    }

    //=============================================
    //            Section 7: Builtin Registration
    //=============================================
    fn init_builtins(&mut self) {
        self.register_builtin(
            "prt",
            NativeArity::Range { min: 0, max: None },
            Interpreter::builtin_prt,
        );
        // Backward-compatible alias.
        self.register_builtin(
            "print",
            NativeArity::Range { min: 0, max: None },
            Interpreter::builtin_prt,
        );
        self.register_builtin(
            "println",
            NativeArity::Range { min: 0, max: None },
            Interpreter::builtin_println,
        );
        self.register_builtin("div", NativeArity::Exact(2), Interpreter::builtin_div);
        // Legacy alias for scripts using the longer name.
        self.register_builtin("division", NativeArity::Exact(2), Interpreter::builtin_div);
        self.register_builtin("sbt", NativeArity::Exact(2), Interpreter::builtin_sbt);
        self.register_builtin("subtract", NativeArity::Exact(2), Interpreter::builtin_sbt);
        self.register_builtin("bool", NativeArity::Exact(1), Interpreter::builtin_bool);
        self.register_builtin("boolean", NativeArity::Exact(1), Interpreter::builtin_bool);
        self.register_builtin("endl", NativeArity::Exact(0), Interpreter::builtin_endl);
        self.register_builtin(
            "hal_devices",
            NativeArity::Range {
                min: 0,
                max: Some(1),
            },
            Interpreter::builtin_hal_devices,
        );
        self.register_builtin(
            "hal_read",
            NativeArity::Exact(2),
            Interpreter::builtin_hal_read,
        );
        self.register_builtin(
            "hal_write",
            NativeArity::Exact(3),
            Interpreter::builtin_hal_write,
        );
        self.register_builtin(
            "hal_interrupt",
            NativeArity::Range {
                min: 2,
                max: Some(3),
            },
            Interpreter::builtin_hal_interrupt,
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
            "process_run",
            NativeArity::Exact(1),
            Interpreter::builtin_process_run,
        );
        self.register_builtin(
            "process_spawn",
            NativeArity::Exact(1),
            Interpreter::builtin_process_spawn,
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
            "fs_exists",
            NativeArity::Exact(1),
            Interpreter::builtin_fs_exists,
        );
        self.register_builtin(
            "fs_ls",
            NativeArity::Exact(1),
            Interpreter::builtin_fs_ls,
        );
        self.register_builtin(
            "close_file",
            NativeArity::Exact(1),
            Interpreter::builtin_close_file,
        );
        self.register_builtin(
            "json_stringify",
            NativeArity::Exact(1),
            Interpreter::builtin_json_stringify,
        );
        self.register_builtin(
            "json_parse",
            NativeArity::Exact(1),
            Interpreter::builtin_json_parse,
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
        self.register_builtin(
            "path_join",
            NativeArity::Exact(2),
            Interpreter::builtin_path_join,
        );
        self.register_builtin(
            "path_home_dir",
            NativeArity::Exact(0),
            Interpreter::builtin_path_home_dir,
        );
        self.register_builtin(
            "io_stdout_writeln",
            NativeArity::Exact(1),
            Interpreter::builtin_io_stdout_writeln,
        );
        self.register_builtin(
            "io_stderr_writeln",
            NativeArity::Exact(1),
            Interpreter::builtin_io_stderr_writeln,
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
            VariableEntry {
                value: Value::NativeFunction {
                    name: name.to_string(),
                    arity,
                    func,
                },
                mutable: false,
            },
        );
    }

    fn builtin_prt(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        // Preserve the exact formatting supplied by the script, including escape sequences
        // that were decoded by the tokenizer. We intentionally avoid inserting separators or
        // implicit newlines so NovaScript authors have full control over stdout layout.
        write_values_to_stdout(args, false)?;
        Ok(Value::Null)
    }

    fn builtin_println(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        // `println` is a convenience wrapper over `prt` that appends a newline after the payload.
        write_values_to_stdout(args, true)?;
        Ok(Value::Null)
    }

    fn builtin_endl(&mut self, _: &[Value]) -> Result<Value, RuntimeError> {
        // Provides a semantic newline emitter so NovaScript code can mirror C++'s `std::endl`.
        // Flushing stdout here keeps interactive shells responsive when `endl()` is used alone.
        write_values_to_stdout(&[], true)?;
        Ok(Value::Null)
    }

    fn builtin_hal_devices(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let filter = if let Some(arg) = args.get(0) {
            Some(Self::parse_device_kind_arg(arg)?)
        } else {
            None
        };
        let devices = self.hal.list_devices(filter);
        let results = devices
            .into_iter()
            .map(|info| Self::device_info_to_value(&info))
            .collect();
        Ok(Value::Array(results))
    }

    fn builtin_hal_read(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let handle = Self::expect_string(&args[0], "hal_read handle")?;
        let register = expect_index(&args[1])?;
        let device = self.find_device(&handle)?;
        let value = self
            .hal
            .read_register(&device.handle, register)
            .map_err(|err| RuntimeError::Custom(err.to_string()))?;
        Ok(Value::Int(value as i64))
    }

    fn builtin_hal_write(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let handle = Self::expect_string(&args[0], "hal_write handle")?;
        let register = expect_index(&args[1])?;
        let value = Self::expect_u32(&args[2], "hal_write value")?;
        let device = self.find_device(&handle)?;
        self.hal
            .write_register(&device.handle, register, value)
            .map_err(|err| RuntimeError::Custom(err.to_string()))?;
        Ok(Value::Null)
    }

    fn builtin_hal_interrupt(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let handle = Self::expect_string(&args[0], "hal_interrupt handle")?;
        let irq = Self::expect_u32(&args[1], "hal_interrupt irq")?;
        let payload = if let Some(arg) = args.get(2) {
            Some(Self::expect_u32(arg, "hal_interrupt payload")?)
        } else {
            None
        };
        let device = self.find_device(&handle)?;
        self.hal
            .raise_interrupt(&device.handle, irq, payload)
            .map_err(|err| RuntimeError::Custom(err.to_string()))?;
        Ok(Value::Null)
    }

    fn builtin_div(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let numerator = expect_number(&args[0], "div")?;
        let denominator = expect_number(&args[1], "div")?;
        if denominator == 0.0 {
            return Err(RuntimeError::DivisionByZero);
        }

        Ok(Value::Float(numerator / denominator))
    }

    fn builtin_sbt(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let lhs_is_int = matches!(args[0], Value::Int(_)) && matches!(args[1], Value::Int(_));
        let left = expect_number(&args[0], "sbt")?;
        let right = expect_number(&args[1], "sbt")?;

        if lhs_is_int {
            Ok(Value::Int((left - right) as i64))
        } else {
            Ok(Value::Float(left - right))
        }
    }

    fn builtin_bool(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::Bool(args[0].is_truthy()))
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

    //=============================================/*
    //  Registers NovaScript builtins and exposes host integrations.
    //============================================*/
    //=============================================
    //            Section 8: Program Evaluation
    //=============================================

    //Function: eval_program
    //Purpose: Execute a NovaScript program without origin tracking
    //Inputs: &mut self, program: &Program
    //Returns: Result<Option<Value>, RuntimeError>
    pub fn eval_program(&mut self, program: &Program) -> Result<Option<Value>, RuntimeError> {
        self.eval_program_with_origin::<&Path>(program, None)
    }

    //Function: eval_program_with_origin
    //Purpose: Evaluate a program while recording filesystem origin for module resolution
    //Inputs: &mut self, program: &Program, origin: Option<P>
    //Returns: Result<Option<Value>, RuntimeError>
    pub fn eval_program_with_origin<P>(
        &mut self,
        program: &Program,
        origin: Option<P>,
    ) -> Result<Option<Value>, RuntimeError>
    where
        P: AsRef<Path>,
    {
        let maybe_dir = origin.as_ref().map(|origin_path| {
            origin_path
                .as_ref()
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        });

        if let Some(dir) = &maybe_dir {
            self.module_path_stack.push(dir.clone());
        }

        let result = self.eval_program_internal(program);

        if maybe_dir.is_some() {
            self.module_path_stack.pop();
        }

        result
    }

    fn eval_program_internal(&mut self, program: &Program) -> Result<Option<Value>, RuntimeError> {
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
            Stmt::ImportDecl { decl } => {
                self.execute_import(decl)?;
                Ok(None)
            }
            Stmt::VariableDecl { decl } => {
                let val = if let Some(expr) = &decl.initializer {
                    self.eval_expr(expr)?
                } else {
                    Value::Null
                };
                self.define_variable(decl.name.clone(), val, decl.is_mutable);
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
                            self.define_variable(variable.clone(), item, true);
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
                self.define_variable(decl.name.clone(), func, false);
                Ok(None)
            }

            other => Err(RuntimeError::NotImplemented(format!(
                "Statement: {:?}",
                other
            ))),
        }
    }

    fn execute_import(&mut self, decl: &ImportDecl) -> Result<(), RuntimeError> {
        let base_dir = self.module_path_stack.last().map(|path| path.as_path());
        let descriptor = {
            let mut loader = self.module_loader.borrow_mut();
            loader
                .prepare_module(&decl.source, base_dir)
                .map_err(RuntimeError::from)?
        };

        let exports = {
            let cached = {
                let loader = self.module_loader.borrow();
                loader.exports_cloned(&descriptor.id)
            };
            if let Some(exports) = cached {
                exports
            } else {
                let exports = self.evaluate_module_descriptor(&descriptor)?;
                self.module_loader
                    .borrow_mut()
                    .store_exports(&descriptor.id, exports.clone());
                exports
            }
        };

        self.bind_imports(decl, exports)
    }

    fn evaluate_module_descriptor(
        &mut self,
        descriptor: &ModuleDescriptor,
    ) -> Result<HashMap<String, Value>, RuntimeError> {
        match &descriptor.artifact {
            ModuleArtifact::Script { program, path } => {
                let loader = self.module_loader.clone();
                let mut module_interpreter = Interpreter::with_loader(loader, self.hal.clone());
                let baseline = module_interpreter.globals_snapshot();
                let origin: PathBuf = path.clone();
                let _ =
                    module_interpreter.eval_program_with_origin(program, Some(origin.as_path()))?;
                let updated = module_interpreter.globals_snapshot();
                let builtins: HashSet<String> = baseline.keys().cloned().collect();
                let exports = updated
                    .into_iter()
                    .filter(|(name, _)| !builtins.contains(name))
                    .collect::<HashMap<_, _>>();
                Ok(exports)
            }
            ModuleArtifact::Compiled { path, .. } => Err(RuntimeError::Custom(format!(
                "Compiled module '{}' is not yet supported",
                path.display()
            ))),
        }
    }

    fn bind_imports(
        &mut self,
        decl: &ImportDecl,
        exports: HashMap<String, Value>,
    ) -> Result<(), RuntimeError> {
        let namespace_value = Value::Object(exports.clone());
        if decl.items.is_empty() {
            let binding = decl
                .alias
                .clone()
                .unwrap_or_else(|| Self::default_module_alias(&decl.source));
            self.define_variable(binding, namespace_value, false);
        } else {
            for item in &decl.items {
                if let Some(value) = exports.get(item) {
                    self.define_variable(item.clone(), value.clone(), false);
                } else {
                    return Err(RuntimeError::Custom(format!(
                        "Module {} does not export '{}'; available: {:?}",
                        decl.source.display_name(),
                        item,
                        exports.keys().collect::<Vec<_>>()
                    )));
                }
            }
            if let Some(alias) = &decl.alias {
                self.define_variable(alias.clone(), namespace_value.clone(), false);
            }
        }
        Ok(())
    }

    fn find_device(&self, handle: &str) -> Result<DeviceInfo, RuntimeError> {
        self.hal
            .list_devices(None)
            .into_iter()
            .find(|info| info.handle.as_str() == handle)
            .ok_or_else(|| RuntimeError::ArgumentError(format!("Unknown device handle '{handle}'")))
    }

    fn device_info_to_value(info: &DeviceInfo) -> Value {
        let mut map = HashMap::new();
        map.insert(
            "handle".to_string(),
            Value::String(info.handle.as_str().to_string()),
        );
        map.insert(
            "name".to_string(),
            Value::String(info.descriptor.name.clone()),
        );
        map.insert(
            "kind".to_string(),
            Value::String(Self::device_kind_label(&info.descriptor.kind)),
        );
        map.insert(
            "registers".to_string(),
            Value::Int(info.descriptor.register_count as i64),
        );
        let capabilities = info
            .descriptor
            .capabilities
            .iter()
            .map(|cap| Value::String(Self::device_capability_label(cap)))
            .collect();
        map.insert("capabilities".to_string(), Value::Array(capabilities));
        Value::Object(map)
    }

    fn parse_device_kind_arg(value: &Value) -> Result<DeviceKind, RuntimeError> {
        match value {
            Value::String(name) => Self::parse_device_kind_string(name),
            other => Err(RuntimeError::TypeError(format!(
                "Device kind filter must be string, got {}",
                other.type_name()
            ))),
        }
    }

    fn parse_device_kind_string(name: &str) -> Result<DeviceKind, RuntimeError> {
        let lower = name.to_ascii_lowercase();
        let result = match lower.as_str() {
            "keyboard" => Some(DeviceKind::Keyboard),
            "mouse" => Some(DeviceKind::Mouse),
            "controller" | "gamepad" | "gamecontroller" => Some(DeviceKind::GameController),
            "speaker" | "speaker:external" => Some(DeviceKind::Speaker { external: true }),
            "speaker:internal" => Some(DeviceKind::Speaker { external: false }),
            "microphone" | "mic" => Some(DeviceKind::Microphone),
            "display" | "gpu" => Some(DeviceKind::Display),
            "network" | "net" => Some(DeviceKind::Network),
            "storage" => Some(DeviceKind::Storage(StorageBus::SdCard)),
            _ if lower.starts_with("storage:") => {
                let bus = match lower.split_once(':').map(|(_, b)| b) {
                    Some("sd") | Some("sdcard") => StorageBus::SdCard,
                    Some("nvme") => StorageBus::Nvme,
                    Some("sata") => StorageBus::Sata,
                    Some("usb") => StorageBus::Usb,
                    Some(other) => StorageBus::Custom(other.to_string()),
                    None => StorageBus::SdCard,
                };
                Some(DeviceKind::Storage(bus))
            }
            _ if lower.starts_with("sensor:") => {
                let sensor = match lower.split_once(':').map(|(_, s)| s) {
                    Some("temperature") => SensorKind::Temperature,
                    Some("motion") => SensorKind::Motion,
                    Some("proximity") => SensorKind::Proximity,
                    Some("light") => SensorKind::Light,
                    Some("humidity") => SensorKind::Humidity,
                    Some("pressure") => SensorKind::Pressure,
                    Some(other) => SensorKind::Custom(other.to_string()),
                    None => SensorKind::Temperature,
                };
                Some(DeviceKind::Sensor(sensor))
            }
            _ if lower.starts_with("custom:") => lower
                .split_once(':')
                .map(|(_, label)| DeviceKind::Custom(label.to_string())),
            _ => None,
        };

        result
            .ok_or_else(|| RuntimeError::ArgumentError(format!("Unknown device kind '{}'.", name)))
    }

    fn device_kind_label(kind: &DeviceKind) -> String {
        match kind {
            DeviceKind::Keyboard => "keyboard".into(),
            DeviceKind::Mouse => "mouse".into(),
            DeviceKind::GameController => "controller".into(),
            DeviceKind::Speaker { external } => {
                if *external {
                    "speaker-external".into()
                } else {
                    "speaker-internal".into()
                }
            }
            DeviceKind::Microphone => "microphone".into(),
            DeviceKind::Storage(bus) => format!("storage:{}", Self::storage_bus_label(bus)),
            DeviceKind::Sensor(kind) => format!("sensor:{}", Self::sensor_label(kind)),
            DeviceKind::Display => "display".into(),
            DeviceKind::Network => "network".into(),
            DeviceKind::Custom(label) => format!("custom:{}", label),
        }
    }

    fn storage_bus_label(bus: &StorageBus) -> &'static str {
        match bus {
            StorageBus::Sata => "sata",
            StorageBus::Nvme => "nvme",
            StorageBus::Usb => "usb",
            StorageBus::SdCard => "sdcard",
            StorageBus::MemoryMapped => "mmio",
            StorageBus::Custom(_) => "custom",
        }
    }

    fn sensor_label(kind: &SensorKind) -> &'static str {
        match kind {
            SensorKind::Temperature => "temperature",
            SensorKind::Motion => "motion",
            SensorKind::Proximity => "proximity",
            SensorKind::Light => "light",
            SensorKind::Humidity => "humidity",
            SensorKind::Pressure => "pressure",
            SensorKind::Custom(_) => "custom",
        }
    }

    fn device_capability_label(cap: &DeviceCapability) -> String {
        match cap {
            DeviceCapability::Input => "input".into(),
            DeviceCapability::Output => "output".into(),
            DeviceCapability::Audio => "audio".into(),
            DeviceCapability::Haptic => "haptic".into(),
            DeviceCapability::Storage => "storage".into(),
            DeviceCapability::SensorReading => "sensor".into(),
            DeviceCapability::Network => "network".into(),
            DeviceCapability::Power => "power".into(),
            DeviceCapability::Custom(label) => format!("custom:{}", label),
        }
    }

    fn expect_string(value: &Value, context: &str) -> Result<String, RuntimeError> {
        match value {
            Value::String(s) => Ok(s.clone()),
            other => Err(RuntimeError::TypeError(format!(
                "{} expects string, got {}",
                context,
                other.type_name()
            ))),
        }
    }

    fn expect_u32(value: &Value, context: &str) -> Result<u32, RuntimeError> {
        let number = expect_number(value, context)?;
        if number < 0.0 || number > u32::MAX as f64 {
            return Err(RuntimeError::ArgumentError(format!(
                "{} out of range for u32",
                context
            )));
        }
        Ok(number as u32)
    }

    fn globals_snapshot(&self) -> HashMap<String, Value> {
        self.globals
            .iter()
            .map(|(name, entry)| (name.clone(), entry.value.clone()))
            .collect()
    }

    fn default_module_alias(source: &ImportSource) -> String {
        match source {
            ImportSource::StandardModule(name) | ImportSource::BareModule(name) => name.clone(),
            ImportSource::ScriptPath(path) => Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(path.as_str())
                .to_string(),
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
                    self.assign_variable(name, val.clone())?;
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
            Expr::Lambda {
                params,
                body,
                position,
            } => {
                let closure = self.capture_environment();
                let body_stmt = Stmt::Return {
                    value: Some(*body.clone()),
                    position: position.clone(),
                };
                Ok(Value::Function {
                    name: format!("<lambda@{}:{}>", position.line, position.column),
                    params: params.clone(),
                    body: vec![body_stmt],
                    closure,
                })
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

    fn builtin_process_run(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let spec = args.first().ok_or_else(|| {
            RuntimeError::ArgumentError("process_run expects a command object".into())
        })?;
        let spec = expect_object(spec, "process_run spec")?;
        let program = spec
            .get("program")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RuntimeError::ArgumentError("process_run.program must be a string".into())
            })?;

        let mut command = Command::new(program);

        if let Some(Value::Array(arg_values)) = spec.get("args") {
            for arg in arg_values {
                command.arg(arg.to_string());
            }
        } else if let Some(other) = spec.get("args") {
            return Err(RuntimeError::TypeError(format!(
                "process_run.args must be an array, got {}",
                other.type_name()
            )));
        }

        if let Some(Value::String(dir)) = spec.get("cwd") {
            command.current_dir(dir);
        }

        if let Some(env_values) = spec.get("env") {
            let env_map = expect_object(env_values, "process_run.env")?;
            if matches!(spec.get("clear_env"), Some(Value::Bool(true))) {
                command.env_clear();
            }
            for (key, value) in env_map {
                command.env(key, value.to_string());
            }
        }

        if let Some(stdin) = parse_stdio_option(spec.get("stdin"), "process_run.stdin")? {
            command.stdin(stdin);
        }

        if self.dry_run {
            return Ok(Value::Object(process_status_map(true, Some(0), "", "")));
        }

        let output = command.output()?;
        let success = output.status.success();
        if matches!(spec.get("check"), Some(Value::Bool(true))) && !success {
            return Err(RuntimeError::Custom(format!(
                "process_run: command '{program}' failed (code {:?})",
                output.status.code()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(Value::Object(process_status_map(
            success,
            output.status.code(),
            &stdout,
            &stderr,
        )))
    }

    fn builtin_process_spawn(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let spec = args.first().ok_or_else(|| {
            RuntimeError::ArgumentError("process_spawn expects a command object".into())
        })?;
        let spec = expect_object(spec, "process_spawn spec")?;
        let program = spec
            .get("program")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RuntimeError::ArgumentError("process_spawn.program must be a string".into())
            })?;

        if self.dry_run {
            let mut map = HashMap::new();
            map.insert("ok".to_string(), Value::Bool(true));
            map.insert("pid".to_string(), Value::Int(0));
            return Ok(Value::Object(map));
        }

        let mut command = Command::new(program);

        if let Some(Value::Array(arg_values)) = spec.get("args") {
            for arg in arg_values {
                command.arg(arg.to_string());
            }
        } else if let Some(other) = spec.get("args") {
            return Err(RuntimeError::TypeError(format!(
                "process_spawn.args must be an array, got {}",
                other.type_name()
            )));
        }

        if let Some(Value::String(dir)) = spec.get("cwd") {
            command.current_dir(dir);
        }

        if let Some(env_values) = spec.get("env") {
            let env_map = expect_object(env_values, "process_spawn.env")?;
            if matches!(spec.get("clear_env"), Some(Value::Bool(true))) {
                command.env_clear();
            }
            for (key, value) in env_map {
                command.env(key, value.to_string());
            }
        }

        if let Some(stdin) = parse_stdio_option(spec.get("stdin"), "process_spawn.stdin")? {
            command.stdin(stdin);
        }
        if let Some(stdout) = parse_stdio_option(spec.get("stdout"), "process_spawn.stdout")? {
            command.stdout(stdout);
        }
        if let Some(stderr) = parse_stdio_option(spec.get("stderr"), "process_spawn.stderr")? {
            command.stderr(stderr);
        }

        let child = command.spawn()?;
        let mut map = HashMap::new();
        map.insert("ok".to_string(), Value::Bool(true));
        map.insert("pid".to_string(), Value::Int(child.id() as i64));
        Ok(Value::Object(map))
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

    fn builtin_fs_exists(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let path = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("fs_exists expects string path".into()))?;
        Ok(Value::Bool(Path::new(path).exists()))
    }

    fn builtin_fs_ls(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let path = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("fs_ls expects string path".into()))?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                entries.push(Value::String(name.to_string()));
            }
        }
        Ok(Value::Array(entries))
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

    fn builtin_json_stringify(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let json = value_to_json(&args[0]);
        Ok(Value::String(json.to_string()))
    }

    fn builtin_json_parse(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let text = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("json_parse expects string input".into()))?;
        let parsed: JsonValue = serde_json::from_str(text).map_err(|err| {
            RuntimeError::ArgumentError(format!("json_parse failed: {}", err))
        })?;
        Ok(json_to_value(&parsed))
    }

    fn builtin_path_join(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        let left = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("path_join expects string left".into()))?;
        let right = args[1]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("path_join expects string right".into()))?;
        let joined = PathBuf::from(left).join(right);
        Ok(Value::String(joined.to_string_lossy().to_string()))
    }

    fn builtin_path_home_dir(&mut self, _args: &[Value]) -> Result<Value, RuntimeError> {
        if let Some(home) = home_dir() {
            Ok(Value::String(home.to_string_lossy().to_string()))
        } else {
            Ok(Value::Null)
        }
    }

    fn builtin_io_stdout_writeln(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        writeln!(io::stdout(), "{}", args[0]).map_err(|err| RuntimeError::IoError(err.to_string()))?;
        Ok(Value::Null)
    }

    fn builtin_io_stderr_writeln(&mut self, args: &[Value]) -> Result<Value, RuntimeError> {
        writeln!(io::stderr(), "{}", args[0]).map_err(|err| RuntimeError::IoError(err.to_string()))?;
        Ok(Value::Null)
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
                closure,
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
                if closure.is_empty() {
                    self.push_scope();
                } else {
                    self.push_scope_with_closure(&closure);
                }

                // Bind parameters to arguments
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.define_variable(param.clone(), arg.clone(), true);
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

    //=============================================/*
    //  Executes NovaScript AST nodes, handling statements, expressions, and control flow.
    //============================================*/
    //=============================================
    //            Section 9: Environment Management
    //=============================================

    fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    fn push_scope_with_closure(&mut self, closure: &Environment) {
        self.locals.push(closure.clone());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    fn define_variable(&mut self, name: String, value: Value, mutable: bool) {
        let entry = VariableEntry { value, mutable };
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name, entry);
        } else {
            self.globals.insert(name, entry);
        }
    }

    fn assign_variable(&mut self, name: &str, value: Value) -> Result<Value, RuntimeError> {
        for scope in self.locals.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                if !entry.mutable {
                    return Err(RuntimeError::TypeError(format!(
                        "Cannot assign to immutable variable '{}'",
                        name
                    )));
                }
                entry.value = value.clone();
                return Ok(value);
            }
        }

        if let Some(entry) = self.globals.get_mut(name) {
            if !entry.mutable {
                return Err(RuntimeError::TypeError(format!(
                    "Cannot assign to immutable variable '{}'",
                    name
                )));
            }
            entry.value = value.clone();
            return Ok(value);
        }

        Err(RuntimeError::VariableNotFound(name.to_string()))
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        for scope in self.locals.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry.value.clone());
            }
        }
        self.globals.get(name).map(|entry| entry.value.clone())
    }

    fn capture_environment(&self) -> Environment {
        let mut snapshot = HashMap::new();
        for scope in &self.locals {
            for (name, entry) in scope {
                snapshot.insert(name.clone(), entry.clone());
            }
        }
        for (name, entry) in &self.globals {
            snapshot
                .entry(name.clone())
                .or_insert_with(|| entry.clone());
        }
        snapshot
    }
}

//=============================================/*
//  Manages lexical scopes, assignment rules, and captured closures for functions.
//============================================*/
impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

//=============================================
//            Section 10: Utility Helpers
//=============================================

/// Write NovaScript values to stdout while preserving their exact textual representation.
///
/// The tokenizer already resolved escape sequences inside string literals, so printing here
/// simply delegates to `Display` and flushes the output stream. An optional newline mirrors
/// the behaviour of `println` and the new `endl()` helper without hard-coding separators.
fn write_values_to_stdout(args: &[Value], newline: bool) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    for value in args {
        write!(stdout, "{value}")?;
    }
    if newline {
        writeln!(stdout)?;
    }
    stdout.flush()
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

fn expect_object<'a>(
    value: &'a Value,
    context: &str,
) -> Result<&'a HashMap<String, Value>, RuntimeError> {
    match value {
        Value::Object(map) => Ok(map),
        other => Err(RuntimeError::TypeError(format!(
            "{context} expects object, got {}",
            other.type_name()
        ))),
    }
}

fn parse_stdio_option(
    value: Option<&Value>,
    context: &str,
) -> Result<Option<Stdio>, RuntimeError> {
    let Some(mode_value) = value else {
        return Ok(None);
    };
    let mode = mode_value
        .as_str()
        .ok_or_else(|| RuntimeError::TypeError(format!("{context} expects string mode")))?;
    match mode {
        "null" => Ok(Some(Stdio::null())),
        "inherit" => Ok(Some(Stdio::inherit())),
        other => Err(RuntimeError::ArgumentError(format!(
            "{context} must be \"null\" or \"inherit\", got '{other}'"
        ))),
    }
}

fn process_status_map(
    success: bool,
    code: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> HashMap<String, Value> {
    let mut status = HashMap::new();
    status.insert("success".to_string(), Value::Bool(success));
    status.insert(
        "code".to_string(),
        match code {
            Some(code) => Value::Int(code as i64),
            None => Value::Null,
        },
    );

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Object(status));
    result.insert("stdout".to_string(), Value::String(stdout.to_string()));
    result.insert("stderr".to_string(), Value::String(stderr.to_string()));
    result
}

pub(crate) fn value_to_json(value: &Value) -> JsonValue {
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

pub(crate) fn json_to_value(json: &JsonValue) -> Value {
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

//=============================================/*
//  Provides IO helpers, numeric coercions, and conversions between NovaScript values and JSON.
//============================================*/
//=============================================
//            Section 11: Tests
//=============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::Parser, tokenizer::Tokenizer};
    use std::fs;
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

    #[test]
    fn test_prt_aliases_exist() {
        let interpreter = Interpreter::new();
        assert!(interpreter.globals.contains_key("prt"));
        assert!(interpreter.globals.contains_key("print"));
    }

    #[test]
    fn test_division_aliases() {
        let mut interpreter = Interpreter::new();
        let div_result = call_builtin(&mut interpreter, "div", vec![Value::Int(9), Value::Int(3)]);
        assert_eq!(div_result, Value::Float(3.0));

        let alias_result = call_builtin(
            &mut interpreter,
            "division",
            vec![Value::Float(7.5), Value::Float(2.5)],
        );
        assert_eq!(alias_result, Value::Float(3.0));
    }

    #[test]
    fn test_subtract_aliases() {
        let mut interpreter = Interpreter::new();
        let sbt_result = call_builtin(&mut interpreter, "sbt", vec![Value::Int(10), Value::Int(4)]);
        assert_eq!(sbt_result, Value::Int(6));

        let alias_result = call_builtin(
            &mut interpreter,
            "subtract",
            vec![Value::Float(5.5), Value::Float(2.0)],
        );
        assert_eq!(alias_result, Value::Float(3.5));
    }

    #[test]
    fn test_bool_aliases() {
        let mut interpreter = Interpreter::new();
        let bool_result = call_builtin(&mut interpreter, "bool", vec![Value::Int(1)]);
        assert_eq!(bool_result, Value::Bool(true));

        let alias_result = call_builtin(&mut interpreter, "boolean", vec![Value::Null]);
        assert_eq!(alias_result, Value::Bool(false));
    }

    #[test]
    fn test_hal_devices_builtin() {
        let mut interpreter = Interpreter::new();
        let devices = call_builtin(&mut interpreter, "hal_devices", vec![]);
        let Value::Array(list) = devices else {
            panic!("expected array from hal_devices");
        };
        assert!(!list.is_empty(), "expected at least one device");
        let Value::Object(device) = &list[0] else {
            panic!("expected object entry");
        };
        assert!(device.contains_key("handle"));
    }

    #[test]
    fn test_hal_read_write_cycle() {
        let mut interpreter = Interpreter::new();
        let devices = call_builtin(&mut interpreter, "hal_devices", vec![]);
        let Value::Array(list) = devices else {
            panic!("expected array");
        };
        let Value::Object(first) = &list[0] else {
            panic!("expected object");
        };
        let handle = match first.get("handle") {
            Some(Value::String(s)) => s.clone(),
            other => panic!("unexpected handle value: {other:?}"),
        };

        call_builtin(
            &mut interpreter,
            "hal_write",
            vec![
                Value::String(handle.clone()),
                Value::Int(0),
                Value::Int(123),
            ],
        );

        let read = call_builtin(
            &mut interpreter,
            "hal_read",
            vec![Value::String(handle), Value::Int(0)],
        );
        assert_eq!(read, Value::Int(123));
    }

    #[test]
    fn test_hal_sensor_write_blocked() {
        let mut interpreter = Interpreter::new();
        let devices = call_builtin(
            &mut interpreter,
            "hal_devices",
            vec![Value::String("sensor:temperature".into())],
        );
        let Value::Array(list) = devices else {
            panic!("expected array");
        };
        let Value::Object(first) = &list[0] else {
            panic!("expected object");
        };
        let handle = match first.get("handle") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("missing handle"),
        };
        let err = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            call_builtin(
                &mut interpreter,
                "hal_write",
                vec![Value::String(handle), Value::Int(0), Value::Int(1)],
            );
        }));
        assert!(
            err.is_err(),
            "sensor writes should panic with runtime error"
        );
    }

    fn call_builtin(interpreter: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
        let func = interpreter
            .globals
            .get(name)
            .cloned()
            .expect("builtin not found")
            .value;
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
    fn test_const_assignment_not_allowed() {
        let source = "const LIMIT = 10; LIMIT = 5;";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().expect("tokenize const script");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse const script");
        let mut interpreter = Interpreter::new();
        let err = interpreter
            .eval_program(&program)
            .expect_err("expected assignment error");
        match err {
            RuntimeError::TypeError(message) => {
                assert!(message.contains("immutable variable 'LIMIT'"));
            }
            other => panic!("expected type error, got {other:?}"),
        }
    }

    #[test]
    fn test_lambda_execution() {
        let source = r#"
            let offset = 3;
            let calc = lambda |x| -> x + offset;
            calc(4);
        "#;
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().expect("tokenize lambda script");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse lambda script");
        let mut interpreter = Interpreter::new();
        let result = interpreter
            .eval_program(&program)
            .expect("evaluate lambda script")
            .expect("lambda script should return value");
        assert_eq!(result, Value::Int(7));
    }

    #[test]
    fn test_file_roundtrip() {
        let mut interpreter = Interpreter::new();
        let mut path = env::temp_dir();
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

//=============================================/*
//  Supplies regression coverage for interpreter value semantics and host integrations.
//============================================*/
//=============================================
// End Of nova_script/interpreter.rs
//=============================================
// Notes:
// -[@ZNOTE] Keep interpreter builtins synchronized with NovaCore capability surface.
//=============================================
