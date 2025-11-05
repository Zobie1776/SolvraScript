//==================================================
// File: vm/legacy_builtins.rs
//==================================================
// Author: ZobieLabs
// License: Apache License 2.0
// Goal: Provide transitional wrappers for legacy SolvraScript builtins
// Objective: Allow stdlib modules and classic calls to share identical logic
//==================================================

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use solvra_core::{SolvraError, SolvraResult, Value};

use super::builtins::Builtins;

//==================================================
// Section 1.0 - IO bridge
//==================================================
// @TODO[StdlibPhase1]: add buffered writer once async console lands.
// @ZNOTE[Bridge]: Functions are reused by legacy builtins and new stdlib modules.

pub(crate) fn io_print(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    if let Some(value) = args.first() {
        print!("{}", value_to_string(value));
    }
    Ok(Value::Null)
}

pub(crate) fn io_println(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    if let Some(value) = args.first() {
        println!("{}", value_to_string(value));
    } else {
        println!();
    }
    Ok(Value::Null)
}

pub(crate) fn io_input(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let prompt = args
        .get(0)
        .map(value_to_string)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "> ".to_string());

    print!("{prompt}");
    let _ = io::stdout().flush();

    let mut buffer = String::new();
    if io::stdin().read_line(&mut buffer).is_err() {
        return Ok(Value::String(String::new()));
    }

    while buffer.ends_with(['\n', '\r']) {
        buffer.pop();
    }

    Ok(Value::String(buffer))
}

pub(crate) fn io_inp(builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    io_input(builtins, args)
}

pub(crate) fn io_sleep(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let millis = args.first().and_then(value_to_integer).unwrap_or(0);
    if millis > 0 {
        thread::sleep(Duration::from_millis(millis as u64));
    }
    Ok(Value::Null)
}

//==================================================
// Section 2.0 - String utilities
//==================================================

pub(crate) fn string_len(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let subject = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("len() expects a value to inspect".into()))?;
    match subject {
        Value::String(text) => Ok(Value::Integer(text.chars().count() as i64)),
        Value::Null => Ok(Value::Integer(0)),
        other => Err(SolvraError::Internal(format!(
            "len() only supports strings/null for now, got {}",
            other.type_name()
        ))),
    }
}

pub(crate) fn string_to_string(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let value = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("to_string() expects value".into()))?;
    Ok(Value::String(value_to_string(value)))
}

pub(crate) fn string_parse_int(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let text = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("parse_int() expects text".into()))?;
    let base = args.get(1);
    let radix = match base {
        None | Some(Value::Null) => 10,
        Some(Value::Integer(int)) => {
            if (2..=36).contains(int) {
                *int as u32
            } else {
                return Err(SolvraError::Internal(
                    "parse_int base must be between 2 and 36".into(),
                ));
            }
        }
        Some(other) => {
            return Err(SolvraError::Internal(format!(
                "parse_int base must be an integer, got {}",
                other.type_name()
            )));
        }
    };

    let cleaned = value_to_string(text);
    match i64::from_str_radix(cleaned.trim(), radix) {
        Ok(number) => Ok(Value::Integer(number)),
        Err(_) => Err(SolvraError::Internal(format!(
            "Unable to parse '{}' as base {radix} integer",
            cleaned.trim()
        ))),
    }
}

pub(crate) fn string_parse_float(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let text = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("parse_float() expects text".into()))?;
    let payload = value_to_string(text);
    match payload.trim().parse::<f64>() {
        Ok(number) => Ok(Value::Float(number)),
        Err(_) => Err(SolvraError::Internal(format!(
            "Unable to parse '{}' as float",
            payload.trim()
        ))),
    }
}

//==================================================
// Section 3.0 - Math utilities
//==================================================

pub(crate) fn math_sqrt(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let number = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("sqrt() expects a number".into()))?;
    let value = value_to_f64(number, "sqrt")?;
    if value < 0.0 {
        return Err(SolvraError::Internal(
            "sqrt() cannot operate on negative numbers".into(),
        ));
    }
    Ok(Value::Float(value.sqrt()))
}

pub(crate) fn math_pow(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let base = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("pow() expects a base value".into()))?;
    let exponent = args
        .get(1)
        .ok_or_else(|| SolvraError::Internal("pow() expects an exponent".into()))?;
    let lhs = value_to_f64(base, "pow")?;
    let rhs = value_to_f64(exponent, "pow")?;
    Ok(Value::Float(lhs.powf(rhs)))
}

pub(crate) fn math_sin(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let value = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("sin() expects a number".into()))?;
    Ok(Value::Float(value_to_f64(value, "sin")?.sin()))
}

pub(crate) fn math_cos(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let value = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("cos() expects a number".into()))?;
    Ok(Value::Float(value_to_f64(value, "cos")?.cos()))
}

pub(crate) fn math_abs(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let value = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("abs() expects a number".into()))?;
    match value {
        Value::Integer(int) => Ok(Value::Integer(int.abs())),
        Value::Float(float) => Ok(Value::Float(float.abs())),
        Value::Boolean(flag) => {
            if *flag {
                Ok(Value::Integer(1))
            } else {
                Ok(Value::Integer(0))
            }
        }
        Value::Null => Ok(Value::Integer(0)),
        other => Err(SolvraError::Internal(format!(
            "abs() expects numeric input, got {}",
            other.type_name()
        ))),
    }
}

pub(crate) fn math_min(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let left = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("min() expects left operand".into()))?;
    let right = args
        .get(1)
        .ok_or_else(|| SolvraError::Internal("min() expects right operand".into()))?;
    let (lhs, rhs) = (value_to_f64(left, "min")?, value_to_f64(right, "min")?);
    if matches!(left, Value::Integer(_)) && matches!(right, Value::Integer(_)) {
        Ok(Value::Integer(lhs.min(rhs) as i64))
    } else {
        Ok(Value::Float(lhs.min(rhs)))
    }
}

pub(crate) fn math_max(_builtins: &Builtins, args: &[Value]) -> SolvraResult<Value> {
    let left = args
        .get(0)
        .ok_or_else(|| SolvraError::Internal("max() expects left operand".into()))?;
    let right = args
        .get(1)
        .ok_or_else(|| SolvraError::Internal("max() expects right operand".into()))?;
    let (lhs, rhs) = (value_to_f64(left, "max")?, value_to_f64(right, "max")?);
    if matches!(left, Value::Integer(_)) && matches!(right, Value::Integer(_)) {
        Ok(Value::Integer(lhs.max(rhs) as i64))
    } else {
        Ok(Value::Float(lhs.max(rhs)))
    }
}

//==================================================
// Section 4.0 - Helpers
//==================================================

pub(crate) fn value_to_string(value: &Value) -> String {
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

fn value_to_integer(value: &Value) -> Option<i64> {
    match value {
        Value::Integer(int) => Some(*int),
        Value::Float(float) => Some(*float as i64),
        Value::Boolean(flag) => Some(if *flag { 1 } else { 0 }),
        _ => None,
    }
}

fn value_to_f64(value: &Value, name: &str) -> SolvraResult<f64> {
    match value {
        Value::Integer(int) => Ok(*int as f64),
        Value::Float(float) => Ok(*float),
        Value::Boolean(flag) => Ok(if *flag { 1.0 } else { 0.0 }),
        Value::Null => Ok(0.0),
        other => Err(SolvraError::Internal(format!(
            "{name}() expects numeric input, got {}",
            other.type_name()
        ))),
    }
}
