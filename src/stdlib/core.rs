//! Core module for Zyra standard library
//!
//! Provides foundational types and functions:
//! - assert() / panic() for runtime checks
//! - Option<T> / Result<T,E> type helpers
//! - Type introspection utilities

use crate::compiler::bytecode::Value;
use crate::error::{ZyraError, ZyraResult};

/// Assert that a condition is true, panic with message if false
pub fn assert_true(condition: bool, message: &str) -> ZyraResult<Value> {
    if condition {
        Ok(Value::None)
    } else {
        Err(ZyraError::new(
            "AssertionError",
            &format!("Assertion failed: {}", message),
            None,
        ))
    }
}

/// Panic with a message - halts execution
pub fn panic(message: &str) -> ZyraResult<Value> {
    Err(ZyraError::new(
        "PanicError",
        &format!("Panic: {}", message),
        None,
    ))
}

/// Check if a value is None (Option::None)
pub fn is_none(value: &Value) -> bool {
    matches!(value, Value::None)
}

/// Check if a value is Some (not None)
pub fn is_some(value: &Value) -> bool {
    matches!(value, Value::Some(_))
}

/// Unwrap an Option value, panic if None
pub fn unwrap(value: Value) -> ZyraResult<Value> {
    match value {
        Value::Some(inner) => Ok(*inner),
        Value::None => Err(ZyraError::new(
            "UnwrapError",
            "Called unwrap() on a None value",
            None,
        )),
        other => Ok(other), // Non-Option values pass through
    }
}

/// Unwrap an Option value, return default if None
pub fn unwrap_or(value: Value, default: Value) -> Value {
    match value {
        Value::Some(inner) => *inner,
        Value::None => default,
        other => other,
    }
}

/// Get the type name of a value
pub fn type_of(value: &Value) -> String {
    match value {
        Value::Int(_) | Value::I64(_) => "Int".to_string(),
        Value::Float(_) | Value::F64(_) => "Float".to_string(),
        Value::Bool(_) => "Bool".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Array(_) | Value::Vec(_) | Value::List(_) => "Array".to_string(),
        Value::None => "None".to_string(),
        Value::Some(_) => "Some".to_string(),
        Value::Ok(_) => "Ok".to_string(),
        Value::Err(_) => "Err".to_string(),
        Value::Object(_) => "Object".to_string(),
        Value::Reference { .. } => "Reference".to_string(),
        Value::Function { .. } => "Function".to_string(),
        Value::Window(_) => "Window".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Compare two values for equality
pub fn equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::None, Value::None) => true,
        _ => false,
    }
}

/// Compare two values (returns -1, 0, or 1)
pub fn compare(a: &Value, b: &Value) -> i64 {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => {
            if x < y {
                -1
            } else if x > y {
                1
            } else {
                0
            }
        }
        (Value::Float(x), Value::Float(y)) => {
            if x < y {
                -1
            } else if x > y {
                1
            } else {
                0
            }
        }
        (Value::String(x), Value::String(y)) => {
            if x < y {
                -1
            } else if x > y {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Create a Some value (wraps any value as Option::Some)
pub fn some(value: Value) -> Value {
    Value::Some(Box::new(value))
}

/// Create a None value
pub fn none() -> Value {
    Value::None
}

/// Create an Ok result (for Result<T,E>)
pub fn ok(value: Value) -> Value {
    Value::Ok(Box::new(value))
}

/// Create an Err result
pub fn err(message: String) -> Value {
    Value::Err(Box::new(Value::String(message)))
}

/// Check if a Result is Ok
pub fn is_ok(result: &Value) -> bool {
    matches!(result, Value::Ok(_))
}

/// Check if a Result is Err
pub fn is_err(result: &Value) -> bool {
    matches!(result, Value::Err(_))
}

/// Unwrap a Result, panic if Err
pub fn result_unwrap(result: Value) -> ZyraResult<Value> {
    match result {
        Value::Ok(inner) => Ok(*inner),
        Value::Err(err_val) => {
            let msg = match *err_val {
                Value::String(s) => s,
                _ => "Unknown error".to_string(),
            };
            Err(ZyraError::new(
                "ResultError",
                &format!("Called unwrap() on an Err result: {}", msg),
                None,
            ))
        }
        _ => Err(ZyraError::new("TypeError", "Expected a Result type", None)),
    }
}

/// Get the error message from a Result
pub fn result_error(result: &Value) -> Option<String> {
    if let Value::Err(err_val) = result {
        if let Value::String(s) = err_val.as_ref() {
            return Some(s.clone());
        }
    }
    None
}

/// Unwrap a Result, return default if Err
pub fn result_unwrap_or(result: Value, default: Value) -> Value {
    match result {
        Value::Ok(inner) => *inner,
        _ => default,
    }
}

/// Map over Option: apply function to Some value
pub fn option_map<F>(value: Value, f: F) -> Value
where
    F: FnOnce(Value) -> Value,
{
    match value {
        Value::Some(inner) => Value::Some(Box::new(f(*inner))),
        Value::None => Value::None,
        other => f(other),
    }
}
