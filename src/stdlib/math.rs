//! Math module for Zyra standard library
//!
//! Provides mathematical operations:
//! - Basic: abs, min, max, clamp
//! - Rounding: floor, ceil, round
//! - Powers: sqrt, pow, exp, log
//! - Trig: sin, cos, tan, atan2
//! - Vectors: Vec2, Vec3 operations
//! - Interpolation: lerp, smoothstep

use crate::compiler::bytecode::Value;

// ===== Basic Math =====

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

/// Clamp a value between min and max
pub fn clamp(value: &Value, min_v: &Value, max_v: &Value) -> Value {
    match (value, min_v, max_v) {
        (Value::Int(v), Value::Int(mn), Value::Int(mx)) => Value::Int(*v.max(mn).min(mx)),
        (Value::Float(v), Value::Float(mn), Value::Float(mx)) => Value::Float(v.max(*mn).min(*mx)),
        _ => value.clone(),
    }
}

/// Sign of a value (-1, 0, or 1)
pub fn sign(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Int(if *n > 0 {
            1
        } else if *n < 0 {
            -1
        } else {
            0
        }),
        Value::Float(n) => Value::Float(if *n > 0.0 {
            1.0
        } else if *n < 0.0 {
            -1.0
        } else {
            0.0
        }),
        _ => Value::None,
    }
}

// ===== Rounding =====

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

/// Truncate towards zero
pub fn trunc(value: &Value) -> Value {
    match value {
        Value::Float(n) => Value::Int(n.trunc() as i64),
        Value::Int(n) => Value::Int(*n),
        _ => Value::None,
    }
}

/// Fractional part
pub fn fract(value: &Value) -> Value {
    match value {
        Value::Float(n) => Value::Float(n.fract()),
        Value::Int(_) => Value::Float(0.0),
        _ => Value::None,
    }
}

// ===== Powers & Roots =====

/// Square root
pub fn sqrt(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).sqrt()),
        Value::Float(n) => Value::Float(n.sqrt()),
        _ => Value::None,
    }
}

/// Cube root
pub fn cbrt(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).cbrt()),
        Value::Float(n) => Value::Float(n.cbrt()),
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

/// Natural exponent (e^x)
pub fn exp(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).exp()),
        Value::Float(n) => Value::Float(n.exp()),
        _ => Value::None,
    }
}

/// Natural logarithm
pub fn ln(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).ln()),
        Value::Float(n) => Value::Float(n.ln()),
        _ => Value::None,
    }
}

/// Base-10 logarithm
pub fn log10(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).log10()),
        Value::Float(n) => Value::Float(n.log10()),
        _ => Value::None,
    }
}

/// Base-2 logarithm
pub fn log2(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).log2()),
        Value::Float(n) => Value::Float(n.log2()),
        _ => Value::None,
    }
}

// ===== Trigonometry =====

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

/// Tangent
pub fn tan(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).tan()),
        Value::Float(n) => Value::Float(n.tan()),
        _ => Value::None,
    }
}

/// Arc sine
pub fn asin(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).asin()),
        Value::Float(n) => Value::Float(n.asin()),
        _ => Value::None,
    }
}

/// Arc cosine
pub fn acos(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).acos()),
        Value::Float(n) => Value::Float(n.acos()),
        _ => Value::None,
    }
}

/// Arc tangent
pub fn atan(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Float((*n as f64).atan()),
        Value::Float(n) => Value::Float(n.atan()),
        _ => Value::None,
    }
}

/// Arc tangent of y/x (handles quadrants correctly)
pub fn atan2(y: &Value, x: &Value) -> Value {
    let y_val = match y {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        _ => return Value::None,
    };
    let x_val = match x {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        _ => return Value::None,
    };
    Value::Float(y_val.atan2(x_val))
}

/// Convert degrees to radians
pub fn to_radians(degrees: &Value) -> Value {
    match degrees {
        Value::Int(n) => Value::Float((*n as f64).to_radians()),
        Value::Float(n) => Value::Float(n.to_radians()),
        _ => Value::None,
    }
}

/// Convert radians to degrees
pub fn to_degrees(radians: &Value) -> Value {
    match radians {
        Value::Int(n) => Value::Float((*n as f64).to_degrees()),
        Value::Float(n) => Value::Float(n.to_degrees()),
        _ => Value::None,
    }
}

// ===== Constants =====

/// PI constant
pub fn pi() -> Value {
    Value::Float(std::f64::consts::PI)
}

/// E constant
pub fn e() -> Value {
    Value::Float(std::f64::consts::E)
}

/// TAU (2*PI)
pub fn tau() -> Value {
    Value::Float(std::f64::consts::TAU)
}

// ===== Random =====

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

/// Generate a random float between 0 and 1
pub fn random_float() -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    Value::Float((seed as f64) / (u32::MAX as f64))
}

// ===== Interpolation =====

/// Linear interpolation between a and b by t
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Linear interpolation (Value version)
pub fn lerp_value(a: &Value, b: &Value, t: &Value) -> Value {
    let a_val = match a {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        _ => return Value::None,
    };
    let b_val = match b {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        _ => return Value::None,
    };
    let t_val = match t {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        _ => return Value::None,
    };
    Value::Float(lerp(a_val, b_val, t_val))
}

/// Inverse lerp: get t value for a given value between a and b
pub fn inverse_lerp(a: f64, b: f64, value: f64) -> f64 {
    if (b - a).abs() < f64::EPSILON {
        0.0
    } else {
        (value - a) / (b - a)
    }
}

/// Smoothstep interpolation
pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smootherstep interpolation (Ken Perlin's version)
pub fn smootherstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

// ===== Vec2 Operations =====

/// Create a Vec2 struct
pub fn vec2_new(x: f64, y: f64) -> Value {
    let mut map = std::collections::HashMap::new();
    map.insert("_type".to_string(), Value::String("Vec2".to_string()));
    map.insert("x".to_string(), Value::Float(x));
    map.insert("y".to_string(), Value::Float(y));
    Value::Object(map)
}

/// Add two Vec2
pub fn vec2_add(a: &Value, b: &Value) -> Value {
    let (ax, ay) = extract_vec2(a);
    let (bx, by) = extract_vec2(b);
    vec2_new(ax + bx, ay + by)
}

/// Subtract two Vec2
pub fn vec2_sub(a: &Value, b: &Value) -> Value {
    let (ax, ay) = extract_vec2(a);
    let (bx, by) = extract_vec2(b);
    vec2_new(ax - bx, ay - by)
}

/// Scale Vec2 by scalar
pub fn vec2_scale(v: &Value, s: f64) -> Value {
    let (x, y) = extract_vec2(v);
    vec2_new(x * s, y * s)
}

/// Vec2 dot product
pub fn vec2_dot(a: &Value, b: &Value) -> f64 {
    let (ax, ay) = extract_vec2(a);
    let (bx, by) = extract_vec2(b);
    ax * bx + ay * by
}

/// Vec2 length
pub fn vec2_len(v: &Value) -> f64 {
    let (x, y) = extract_vec2(v);
    (x * x + y * y).sqrt()
}

/// Vec2 normalize
pub fn vec2_normalize(v: &Value) -> Value {
    let len = vec2_len(v);
    if len < f64::EPSILON {
        vec2_new(0.0, 0.0)
    } else {
        vec2_scale(v, 1.0 / len)
    }
}

/// Vec2 distance between two points
pub fn vec2_distance(a: &Value, b: &Value) -> f64 {
    vec2_len(&vec2_sub(a, b))
}

/// Vec2 lerp
pub fn vec2_lerp(a: &Value, b: &Value, t: f64) -> Value {
    let (ax, ay) = extract_vec2(a);
    let (bx, by) = extract_vec2(b);
    vec2_new(lerp(ax, bx, t), lerp(ay, by, t))
}

/// Vec2 angle in radians
pub fn vec2_angle(v: &Value) -> f64 {
    let (x, y) = extract_vec2(v);
    y.atan2(x)
}

/// Vec2 from angle and length
pub fn vec2_from_angle(angle: f64, length: f64) -> Value {
    vec2_new(angle.cos() * length, angle.sin() * length)
}

fn extract_vec2(v: &Value) -> (f64, f64) {
    if let Value::Object(map) = v {
        let x = map.get("x").map(|v| extract_float(v)).unwrap_or(0.0);
        let y = map.get("y").map(|v| extract_float(v)).unwrap_or(0.0);
        return (x, y);
    }
    (0.0, 0.0)
}

// ===== Vec3 Operations =====

/// Create a Vec3 struct
pub fn vec3_new(x: f64, y: f64, z: f64) -> Value {
    let mut map = std::collections::HashMap::new();
    map.insert("_type".to_string(), Value::String("Vec3".to_string()));
    map.insert("x".to_string(), Value::Float(x));
    map.insert("y".to_string(), Value::Float(y));
    map.insert("z".to_string(), Value::Float(z));
    Value::Object(map)
}

/// Add two Vec3
pub fn vec3_add(a: &Value, b: &Value) -> Value {
    let (ax, ay, az) = extract_vec3(a);
    let (bx, by, bz) = extract_vec3(b);
    vec3_new(ax + bx, ay + by, az + bz)
}

/// Subtract two Vec3
pub fn vec3_sub(a: &Value, b: &Value) -> Value {
    let (ax, ay, az) = extract_vec3(a);
    let (bx, by, bz) = extract_vec3(b);
    vec3_new(ax - bx, ay - by, az - bz)
}

/// Scale Vec3 by scalar
pub fn vec3_scale(v: &Value, s: f64) -> Value {
    let (x, y, z) = extract_vec3(v);
    vec3_new(x * s, y * s, z * s)
}

/// Vec3 dot product
pub fn vec3_dot(a: &Value, b: &Value) -> f64 {
    let (ax, ay, az) = extract_vec3(a);
    let (bx, by, bz) = extract_vec3(b);
    ax * bx + ay * by + az * bz
}

/// Vec3 cross product
pub fn vec3_cross(a: &Value, b: &Value) -> Value {
    let (ax, ay, az) = extract_vec3(a);
    let (bx, by, bz) = extract_vec3(b);
    vec3_new(ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx)
}

/// Vec3 length
pub fn vec3_len(v: &Value) -> f64 {
    let (x, y, z) = extract_vec3(v);
    (x * x + y * y + z * z).sqrt()
}

/// Vec3 normalize
pub fn vec3_normalize(v: &Value) -> Value {
    let len = vec3_len(v);
    if len < f64::EPSILON {
        vec3_new(0.0, 0.0, 0.0)
    } else {
        vec3_scale(v, 1.0 / len)
    }
}

/// Vec3 distance
pub fn vec3_distance(a: &Value, b: &Value) -> f64 {
    vec3_len(&vec3_sub(a, b))
}

/// Vec3 lerp
pub fn vec3_lerp(a: &Value, b: &Value, t: f64) -> Value {
    let (ax, ay, az) = extract_vec3(a);
    let (bx, by, bz) = extract_vec3(b);
    vec3_new(lerp(ax, bx, t), lerp(ay, by, t), lerp(az, bz, t))
}

fn extract_vec3(v: &Value) -> (f64, f64, f64) {
    if let Value::Object(map) = v {
        let x = map.get("x").map(|v| extract_float(v)).unwrap_or(0.0);
        let y = map.get("y").map(|v| extract_float(v)).unwrap_or(0.0);
        let z = map.get("z").map(|v| extract_float(v)).unwrap_or(0.0);
        return (x, y, z);
    }
    (0.0, 0.0, 0.0)
}

fn extract_float(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(i) => *i as f64,
        _ => 0.0,
    }
}
