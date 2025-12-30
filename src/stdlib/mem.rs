//! Memory module for Zyra standard library
//!
//! Provides memory and ownership utilities:
//! - size/alignment info
//! - explicit drop
//! - memory statistics
//! - reference counting helpers

use crate::compiler::bytecode::Value;
use std::collections::HashMap;
use std::mem;

/// Get the size of a type in bytes (approximate for Zyra values)
pub fn size_of_value(value: &Value) -> i64 {
    match value {
        Value::None => 0,
        Value::Bool(_) => 1,
        Value::Int(_) | Value::I64(_) => 8,
        Value::Float(_) | Value::F64(_) => 8,
        Value::String(s) => s.len() as i64,
        Value::Array(arr) | Value::Vec(arr) | Value::List(arr) => {
            let base = mem::size_of::<Vec<Value>>() as i64;
            let elements: i64 = arr.iter().map(|v| size_of_value(v)).sum();
            base + elements
        }
        Value::Object(map) => {
            let base = mem::size_of::<HashMap<String, Value>>() as i64;
            let field_size: i64 = map
                .iter()
                .map(|(k, v)| k.len() as i64 + size_of_value(v))
                .sum();
            base + field_size
        }
        Value::Reference { .. } => mem::size_of::<usize>() as i64,
        Value::Function { .. } => 48, // approximate
        Value::Window(_) => 64,       // approximate
        _ => 8,                       // default
    }
}

/// Get type name as string
pub fn type_name(value: &Value) -> String {
    match value {
        Value::None => "None".to_string(),
        Value::Bool(_) => "Bool".to_string(),
        Value::Int(_) | Value::I64(_) => "Int".to_string(),
        Value::Float(_) | Value::F64(_) => "Float".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Array(_) | Value::Vec(_) | Value::List(_) => "Array".to_string(),
        Value::Object(_) => "Object".to_string(),
        Value::Some(_) => "Some".to_string(),
        Value::Ok(_) => "Ok".to_string(),
        Value::Err(_) => "Err".to_string(),
        Value::Reference { .. } => "Reference".to_string(),
        Value::Function { name, .. } => format!("Function<{}>", name),
        Value::Window(_) => "Window".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Check if a value is a reference type
pub fn is_reference(value: &Value) -> bool {
    matches!(value, Value::Reference { .. })
}

/// Check if a value is a primitive type
pub fn is_primitive(value: &Value) -> bool {
    matches!(
        value,
        Value::None | Value::Bool(_) | Value::Int(_) | Value::Float(_)
    )
}

/// Check if a value is a collection type
pub fn is_collection(value: &Value) -> bool {
    matches!(value, Value::Array(_) | Value::Vec(_) | Value::Object(_))
}

/// Clone a value (deep copy)
pub fn deep_clone(value: &Value) -> Value {
    match value {
        Value::Array(arr) => Value::Array(arr.iter().map(|v| deep_clone(v)).collect()),
        Value::Vec(arr) => Value::Vec(arr.iter().map(|v| deep_clone(v)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), deep_clone(v)))
                .collect(),
        ),
        Value::Some(inner) => Value::Some(Box::new(deep_clone(inner))),
        Value::Ok(inner) => Value::Ok(Box::new(deep_clone(inner))),
        Value::Err(inner) => Value::Err(Box::new(deep_clone(inner))),
        _ => value.clone(),
    }
}

/// Memory statistics struct
pub struct MemoryStats {
    pub heap_used: usize,
    pub stack_depth: usize,
    pub value_count: usize,
}

/// Get current memory usage (approximate)
pub fn memory_usage() -> Value {
    // These are approximations since Rust doesn't expose exact heap usage easily
    let heap_estimate = 0i64; // Would need custom allocator to track

    let mut map = HashMap::new();
    map.insert(
        "_type".to_string(),
        Value::String("MemoryStats".to_string()),
    );
    map.insert("heap_used".to_string(), Value::Int(heap_estimate));
    map.insert(
        "platform".to_string(),
        Value::String(std::env::consts::OS.to_string()),
    );
    map.insert(
        "pointer_size".to_string(),
        Value::Int(mem::size_of::<usize>() as i64),
    );
    Value::Object(map)
}

/// Explicit drop (for documentation clarity, actual drop is automatic)
pub fn drop_value(_value: Value) {
    // Value is moved in and dropped at end of function scope
    // This is mainly for semantic clarity in Zyra code
}

/// Swap two values (requires mutable references in actual use)
pub fn swap(a: &mut Value, b: &mut Value) {
    std::mem::swap(a, b);
}

/// Take value and replace with None
pub fn take(value: &mut Value) -> Value {
    std::mem::replace(value, Value::None)
}

/// Replace value and return old value
pub fn replace(dest: &mut Value, src: Value) -> Value {
    std::mem::replace(dest, src)
}

/// Check if two values point to the same memory (for references)
pub fn ptr_eq(a: &Value, b: &Value) -> bool {
    std::ptr::eq(a, b)
}

/// Get raw pointer address (for debugging)
pub fn addr_of(value: &Value) -> i64 {
    value as *const Value as i64
}
