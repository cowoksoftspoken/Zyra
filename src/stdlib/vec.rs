//! Vec module for Zyra standard library
//!
//! Provides dynamic array operations:
//! - push, pop, insert, remove
//! - len, is_empty, clear
//! - get, set, first, last
//! - contains, index_of
//! - reverse, sort

use crate::compiler::bytecode::Value;
use crate::error::{ZyraError, ZyraResult};

/// Create a new empty vector
pub fn vec_new() -> Value {
    Value::Array(Vec::new())
}

/// Create a vector with initial capacity
pub fn vec_with_capacity(capacity: i64) -> Value {
    Value::Array(Vec::with_capacity(capacity as usize))
}

/// Push a value to the end of the vector
pub fn vec_push(arr: &mut Value, value: Value) -> ZyraResult<()> {
    if let Value::Array(ref mut vec) = arr {
        vec.push(value);
        Ok(())
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Pop a value from the end of the vector
pub fn vec_pop(arr: &mut Value) -> ZyraResult<Value> {
    if let Value::Array(ref mut vec) = arr {
        Ok(vec.pop().unwrap_or(Value::None))
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Get the length of the vector
pub fn vec_len(arr: &Value) -> i64 {
    if let Value::Array(vec) = arr {
        vec.len() as i64
    } else {
        0
    }
}

/// Check if the vector is empty
pub fn vec_is_empty(arr: &Value) -> bool {
    if let Value::Array(vec) = arr {
        vec.is_empty()
    } else {
        true
    }
}

/// Clear all elements from the vector
pub fn vec_clear(arr: &mut Value) -> ZyraResult<()> {
    if let Value::Array(ref mut vec) = arr {
        vec.clear();
        Ok(())
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Get a value at index (returns None if out of bounds)
pub fn vec_get(arr: &Value, index: i64) -> Value {
    if let Value::Array(vec) = arr {
        if index >= 0 && (index as usize) < vec.len() {
            vec[index as usize].clone()
        } else {
            Value::None
        }
    } else {
        Value::None
    }
}

/// Set a value at index
pub fn vec_set(arr: &mut Value, index: i64, value: Value) -> ZyraResult<()> {
    if let Value::Array(ref mut vec) = arr {
        if index >= 0 && (index as usize) < vec.len() {
            vec[index as usize] = value;
            Ok(())
        } else {
            Err(ZyraError::new(
                "IndexError",
                &format!("Index {} out of bounds", index),
                None,
            ))
        }
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Insert a value at index (shifts elements right)
pub fn vec_insert(arr: &mut Value, index: i64, value: Value) -> ZyraResult<()> {
    if let Value::Array(ref mut vec) = arr {
        if index >= 0 && (index as usize) <= vec.len() {
            vec.insert(index as usize, value);
            Ok(())
        } else {
            Err(ZyraError::new(
                "IndexError",
                &format!("Index {} out of bounds", index),
                None,
            ))
        }
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Remove a value at index (shifts elements left)
pub fn vec_remove(arr: &mut Value, index: i64) -> ZyraResult<Value> {
    if let Value::Array(ref mut vec) = arr {
        if index >= 0 && (index as usize) < vec.len() {
            Ok(vec.remove(index as usize))
        } else {
            Err(ZyraError::new(
                "IndexError",
                &format!("Index {} out of bounds", index),
                None,
            ))
        }
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Get the first element (or None)
pub fn vec_first(arr: &Value) -> Value {
    if let Value::Array(vec) = arr {
        vec.first().cloned().unwrap_or(Value::None)
    } else {
        Value::None
    }
}

/// Get the last element (or None)
pub fn vec_last(arr: &Value) -> Value {
    if let Value::Array(vec) = arr {
        vec.last().cloned().unwrap_or(Value::None)
    } else {
        Value::None
    }
}

/// Check if vector contains a value
pub fn vec_contains(arr: &Value, value: &Value) -> bool {
    if let Value::Array(vec) = arr {
        for item in vec {
            if values_equal(item, value) {
                return true;
            }
        }
    }
    false
}

/// Find index of a value (returns -1 if not found)
pub fn vec_index_of(arr: &Value, value: &Value) -> i64 {
    if let Value::Array(vec) = arr {
        for (i, item) in vec.iter().enumerate() {
            if values_equal(item, value) {
                return i as i64;
            }
        }
    }
    -1
}

/// Reverse the vector in place
pub fn vec_reverse(arr: &mut Value) -> ZyraResult<()> {
    if let Value::Array(ref mut vec) = arr {
        vec.reverse();
        Ok(())
    } else {
        Err(ZyraError::new("TypeError", "Expected an Array", None))
    }
}

/// Clone a vector
pub fn vec_clone(arr: &Value) -> Value {
    if let Value::Array(vec) = arr {
        Value::Array(vec.clone())
    } else {
        Value::Array(Vec::new())
    }
}

/// Concatenate two vectors
pub fn vec_concat(arr1: &Value, arr2: &Value) -> Value {
    let mut result = Vec::new();
    if let Value::Array(vec1) = arr1 {
        result.extend(vec1.clone());
    }
    if let Value::Array(vec2) = arr2 {
        result.extend(vec2.clone());
    }
    Value::Array(result)
}

/// Slice a vector (returns new vector)
pub fn vec_slice(arr: &Value, start: i64, end: i64) -> Value {
    if let Value::Array(vec) = arr {
        let len = vec.len() as i64;
        let start = start.max(0) as usize;
        let end = end.min(len) as usize;
        if start <= end && start < vec.len() {
            Value::Array(vec[start..end].to_vec())
        } else {
            Value::Array(Vec::new())
        }
    } else {
        Value::Array(Vec::new())
    }
}

/// Join vector elements into a string
pub fn vec_join(arr: &Value, separator: &str) -> String {
    if let Value::Array(vec) = arr {
        let parts: Vec<String> = vec.iter().map(|v| value_to_string(v)).collect();
        parts.join(separator)
    } else {
        String::new()
    }
}

// Helper: compare two values for equality
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::None, Value::None) => true,
        _ => false,
    }
}

// Helper: convert value to string
fn value_to_string(v: &Value) -> String {
    match v {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::None => "None".to_string(),
        _ => "[complex]".to_string(),
    }
}
