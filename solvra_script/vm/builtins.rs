use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use solvra_core::{SolvraError, SolvraResult, Value};

type SyncBuiltin = fn(&[Value]) -> SolvraResult<Value>;
type AsyncBuiltin = fn(Vec<Value>) -> Pin<Box<dyn Future<Output = SolvraResult<Value>> + 'static>>;

#[derive(Clone)]
pub struct Builtins {
    sync: HashMap<String, SyncBuiltin>,
    #[allow(dead_code)]
    async_map: HashMap<String, AsyncBuiltin>,
}

impl Builtins {
    pub fn default() -> Self {
        let mut builtins = Self {
            sync: HashMap::new(),
            async_map: HashMap::new(),
        };
        builtins.register_sync("print", builtin_print);
        builtins.register_sync("println", builtin_println);
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

fn builtin_println(args: &[Value]) -> SolvraResult<Value> {
    if let Some(value) = args.first() {
        println!("{}", value_to_string(value));
    } else {
        println!();
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
