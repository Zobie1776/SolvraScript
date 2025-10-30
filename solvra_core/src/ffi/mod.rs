//! Foreign function interface helpers.

/// Returns the generated C header for embedding SolvraRuntime.
pub fn c_header() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ffi/c_api.h"))
}

/// Returns the JSON description of the C API.
pub fn c_api_json() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ffi/c_api.json"))
}

#[cfg(feature = "ffi")]
mod c_api {
    use crate::{SolvraError, SolvraResult, SolvraRuntime, Value};
    use std::os::raw::{c_uchar, c_uint};
    use std::slice;

    macro_rules! unsafe_fn {
        ($(#[$meta:meta])* fn $name:ident($($arg:ident : $ty:ty),* $(,)?) -> $ret:ty $body:block) => {
            $(#[$meta])*
            pub unsafe extern "C" fn $name($($arg : $ty),*) -> $ret $body
        };
    }

    #[repr(C)]
    pub enum SolvraStatus {
        Ok = 0,
        Error = 1,
    }

    #[repr(C)]
    pub struct SolvraValue {
        pub tag: u32,
        pub int_value: i64,
        pub float_value: f64,
    }

    unsafe fn read_slice<'a>(ptr: *const c_uchar, len: c_uint) -> SolvraResult<&'a [u8]> {
        if ptr.is_null() {
            return Err(SolvraError::Internal("null pointer".into()));
        }
        Ok(slice::from_raw_parts(ptr, len as usize))
    }

    unsafe_fn! {
        /// Creates a new runtime instance and returns an opaque pointer.
        ///
        /// # Safety
        /// The returned pointer must eventually be released with [`solvra_runtime_free`] to avoid
        /// leaking memory.
        fn solvra_runtime_new() -> *mut SolvraRuntime {
            Box::into_raw(Box::new(SolvraRuntime::new()))
        }
    }

    unsafe_fn! {
        /// Releases a runtime previously created via [`solvra_runtime_new`].
        ///
        /// # Safety
        /// The caller must ensure that `ptr` either originates from [`solvra_runtime_new`] or is
        /// `NULL`. Passing any other pointer results in undefined behaviour.
        fn solvra_runtime_free(ptr: *mut SolvraRuntime) -> SolvraStatus {
            if ptr.is_null() {
                return SolvraStatus::Error;
            }
            drop(Box::from_raw(ptr));
            SolvraStatus::Ok
        }
    }

    unsafe_fn! {
        /// Executes bytecode and writes the resulting value to `out_value`.
        ///
        /// # Safety
        /// `runtime` must be a valid pointer produced by [`solvra_runtime_new`]. The `bytecode_ptr`
        /// and `out_value` pointers must reference memory valid for writes for the duration of the
        /// call.
        fn solvra_runtime_execute(
            runtime: *mut SolvraRuntime,
            bytecode_ptr: *const c_uchar,
            bytecode_len: c_uint,
            out_value: *mut SolvraValue,
        ) -> SolvraStatus {
            let Some(runtime) = runtime.as_mut() else {
                return SolvraStatus::Error;
            };
            let Ok(bytes) = read_slice(bytecode_ptr, bytecode_len) else {
                return SolvraStatus::Error;
            };
            match runtime.execute(bytes) {
                Ok(value) => {
                    if !out_value.is_null() {
                        (*out_value).tag = match value {
                            Value::Null => 0,
                            Value::Boolean(b) => {
                                (*out_value).int_value = i64::from(b);
                                1
                            }
                            Value::Integer(i) => {
                                (*out_value).int_value = i;
                                2
                            }
                            Value::Float(f) => {
                                (*out_value).float_value = f;
                                3
                            }
                            Value::String(_) => 4,
                            Value::Object(_) => 5,
                        };
                    }
                    SolvraStatus::Ok
                }
                Err(_) => SolvraStatus::Error,
            }
        }
    }
}

#[cfg(feature = "ffi")]
pub use c_api::*;
