use crate::interpreter::{Interpreter, NativeArity, RuntimeError};

pub fn register_vm_builtins(interpreter: &mut Interpreter) {
    interpreter.register_builtin(
        "core_vm_execute",
        NativeArity::Exact(1),
        |_interp, _args| {
            Err(RuntimeError::NotImplemented(
                "core_vm_execute is not implemented yet".into(),
            ))
        },
    );
    interpreter.register_builtin("core_vm_spawn", NativeArity::Exact(1), |_interp, _args| {
        Err(RuntimeError::NotImplemented(
            "core_vm_spawn is not implemented yet".into(),
        ))
    });
    interpreter.register_builtin("core_task_info", NativeArity::Exact(1), |_interp, _args| {
        Err(RuntimeError::NotImplemented(
            "core_task_info is not implemented yet".into(),
        ))
    });
}
