//! Math module for Zyra standard library

use crate::compiler::bytecode::Value;

/// Absolute value
pub fn abs(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Int(n.abs()),
        Value::Float(n) => Value::Float(n.abs()),
        _ => Value::None,
    }
}

/// Minimum of two values
pub fn min(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Value::Int(*x.min(y)),
        (Value::Float(x), Value::Float(y)) => Value::Float(x.min(*y)),
        (Value::Int(x), Value::Float(y)) => Value::Float((*x as f64).min(*y)),
        (Value::Float(x), Value::Int(y)) => Value::Float(x.min(*y as f64)),
        _ => Value::None,
    }
}

/// Maximum of two values
pub fn max(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Value::Int(*x.max(y)),
        (Value::Float(x), Value::Float(y)) => Value::Float(x.max(*y)),
        (Value::Int(x), Value::Float(y)) => Value::Float((*x as f64).max(*y)),
        (Value::Float(x), Value::Int(y)) => Value::Float(x.max(*y as f64)),
        _ => Value::None,
    }
}

/// Square root
pub fn sqrt(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).sqrt()),
        Value::Float(n) => Value::Float(n.sqrt()),
        _ => Value::None,
    }
}

/// Power
pub fn pow(base: &Value, exp: &Value) -> Value {
    match (base, exp) {
        (Value::Int(b), Value::Int(e)) => {
            if *e >= 0 {
                Value::Int(b.pow(*e as u32))
            } else {
                Value::Float((*b as f64).powf(*e as f64))
            }
        }
        (Value::Float(b), Value::Int(e)) => Value::Float(b.powi(*e as i32)),
        (Value::Float(b), Value::Float(e)) => Value::Float(b.powf(*e)),
        (Value::Int(b), Value::Float(e)) => Value::Float((*b as f64).powf(*e)),
        _ => Value::None,
    }
}

/// Floor
pub fn floor(value: &Value) -> Value {
    match value {
        Value::Float(n) => Value::Int(n.floor() as i64),
        Value::Int(n) => Value::Int(*n),
        _ => Value::None,
    }
}

/// Ceiling
pub fn ceil(value: &Value) -> Value {
    match value {
        Value::Float(n) => Value::Int(n.ceil() as i64),
        Value::Int(n) => Value::Int(*n),
        _ => Value::None,
    }
}

/// Round
pub fn round(value: &Value) -> Value {
    match value {
        Value::Float(n) => Value::Int(n.round() as i64),
        Value::Int(n) => Value::Int(*n),
        _ => Value::None,
    }
}

/// Generate a random integer between min and max (inclusive)
pub fn random(min_val: i64, max_val: i64) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as i64;

    let range = max_val - min_val + 1;
    Value::Int(min_val + (seed.abs() % range))
}

/// Clamp a value between min and max
pub fn clamp(value: &Value, min_v: &Value, max_v: &Value) -> Value {
    match (value, min_v, max_v) {
        (Value::Int(v), Value::Int(mn), Value::Int(mx)) => Value::Int(*v.max(mn).min(mx)),
        (Value::Float(v), Value::Float(mn), Value::Float(mx)) => Value::Float(v.max(*mn).min(*mx)),
        _ => value.clone(),
    }
}

/// Sine
pub fn sin(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).sin()),
        Value::Float(n) => Value::Float(n.sin()),
        _ => Value::None,
    }
}

/// Cosine
pub fn cos(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).cos()),
        Value::Float(n) => Value::Float(n.cos()),
        _ => Value::None,
    }
}

/// PI constant
pub fn pi() -> Value {
    Value::Float(std::f64::consts::PI)
}
