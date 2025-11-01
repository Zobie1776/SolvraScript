use crate::core_bridge::CoreBridge;
use crate::interpreter::{Interpreter, NativeArity, RuntimeError, Value};

pub fn register_vm_builtins(interpreter: &mut Interpreter) {
    interpreter.register_builtin("core_vm_execute", NativeArity::Exact(1), |_interp, args| {
        let path = args[0]
            .as_str()
            .ok_or_else(|| RuntimeError::TypeError("core_vm_execute expects string path".into()))?;
        CoreBridge::global()
            .execute_module(path)
            .map_err(|err| RuntimeError::Custom(format!("VM exec failed: {err}")))?;
        Ok(Value::Null)
    });
}
