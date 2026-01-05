//! Runtime values for Zyra VM

pub use crate::compiler::bytecode::{Value, WindowState};

impl Value {
    /// Perform addition
    pub fn add(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            // Legacy Int (i64)
            (Value::Int(a), Value::Int(b)) => Some(Value::Int(a + b)),
            (Value::Int(a), Value::Float(b)) => Some(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Some(Value::Float(a + *b as f64)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Float(a + b)),

            // I32 (Standard Int)
            (Value::I32(a), Value::I32(b)) => Some(Value::I32(a.wrapping_add(*b))),
            (Value::I32(a), Value::F32(b)) => Some(Value::F32(*a as f32 + b)),
            (Value::F32(a), Value::I32(b)) => Some(Value::F32(a + *b as f32)),

            // F32
            (Value::F32(a), Value::F32(b)) => Some(Value::F32(a + b)),

            // I64
            (Value::I64(a), Value::I64(b)) => Some(Value::I64(a.wrapping_add(*b))),
            (Value::I64(a), Value::F64(b)) => Some(Value::F64(*a as f64 + b)),
            (Value::F64(a), Value::I64(b)) => Some(Value::F64(a + *b as f64)),

            // F64
            (Value::F64(a), Value::F64(b)) => Some(Value::F64(a + b)),

            // Mixed standard types (promotion)
            (Value::I32(a), Value::I64(b)) => Some(Value::I64(*a as i64 + b)),
            (Value::I64(a), Value::I32(b)) => Some(Value::I64(a + *b as i64)),
            (Value::I32(a), Value::F64(b)) => Some(Value::F64(*a as f64 + b)),
            (Value::F64(a), Value::I32(b)) => Some(Value::F64(a + *b as f64)),

            // String concatenation
            (Value::String(a), Value::String(b)) => Some(Value::String(format!("{}{}", a, b))),
            (Value::String(a), b) => Some(Value::String(format!("{}{}", a, b))),
            (a, Value::String(b)) => Some(Value::String(format!("{}{}", a, b))),

            // I64 and Int cross-compatibility (Int is legacy i64)
            (Value::I64(a), Value::Int(b)) => Some(Value::I64(a.wrapping_add(*b))),
            (Value::Int(a), Value::I64(b)) => Some(Value::I64(a.wrapping_add(*b))),

            // F64 and Float cross-compatibility (Float is legacy f64)
            (Value::F64(a), Value::Float(b)) => Some(Value::F64(a + b)),
            (Value::Float(a), Value::F64(b)) => Some(Value::Float(a + b)),

            _ => None,
        }
    }

    /// Perform subtraction
    pub fn sub(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            // Legacy
            (Value::Int(a), Value::Int(b)) => Some(Value::Int(a - b)),
            (Value::Int(a), Value::Float(b)) => Some(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Some(Value::Float(a - *b as f64)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Float(a - b)),

            // I32
            (Value::I32(a), Value::I32(b)) => Some(Value::I32(a.wrapping_sub(*b))),
            (Value::I32(a), Value::F32(b)) => Some(Value::F32(*a as f32 - b)),
            (Value::F32(a), Value::I32(b)) => Some(Value::F32(a - *b as f32)),

            // F32
            (Value::F32(a), Value::F32(b)) => Some(Value::F32(a - b)),

            // I64
            (Value::I64(a), Value::I64(b)) => Some(Value::I64(a.wrapping_sub(*b))),

            // Mixed
            (Value::I32(a), Value::I64(b)) => Some(Value::I64((*a as i64).wrapping_sub(*b))),
            (Value::I64(a), Value::I32(b)) => Some(Value::I64(a.wrapping_sub(*b as i64))),

            _ => None,
        }
    }

    /// Perform multiplication
    pub fn mul(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Float(a * b)),

            (Value::I32(a), Value::I32(b)) => Some(Value::I32(a.wrapping_mul(*b))),
            (Value::F32(a), Value::F32(b)) => Some(Value::F32(a * b)),
            (Value::I64(a), Value::I64(b)) => Some(Value::I64(a.wrapping_mul(*b))),

            // Mixed int/float
            (Value::I32(a), Value::F32(b)) => Some(Value::F32(*a as f32 * b)),
            (Value::F32(a), Value::I32(b)) => Some(Value::F32(a * *b as f32)),

            _ => None,
        }
    }

    /// Perform division
    pub fn div(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) if *b != 0 => Some(Value::Int(a / b)),
            (Value::Float(a), Value::Float(b)) if *b != 0.0 => Some(Value::Float(a / b)),

            (Value::I32(a), Value::I32(b)) if *b != 0 => Some(Value::I32(a / b)),
            (Value::F32(a), Value::F32(b)) if *b != 0.0 => Some(Value::F32(a / b)),
            (Value::I64(a), Value::I64(b)) if *b != 0 => Some(Value::I64(a / b)),

            (Value::I32(a), Value::F32(b)) if *b != 0.0 => Some(Value::F32(*a as f32 / b)),
            (Value::F32(a), Value::I32(b)) if *b != 0 => Some(Value::F32(a / *b as f32)),

            _ => None,
        }
    }

    /// Perform modulo
    pub fn modulo(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) if *b != 0 => Some(Value::Int(a % b)),
            (Value::I32(a), Value::I32(b)) if *b != 0 => Some(Value::I32(a % b)),
            (Value::I64(a), Value::I64(b)) if *b != 0 => Some(Value::I64(a % b)),
            _ => None,
        }
    }

    /// Perform negation
    pub fn neg(&self) -> Option<Value> {
        match self {
            Value::Int(a) => Some(Value::Int(-a)),
            Value::Float(a) => Some(Value::Float(-a)),

            Value::I32(a) => Some(Value::I32(-a)),
            Value::I64(a) => Some(Value::I64(-a)),
            Value::F32(a) => Some(Value::F32(-a)),
            Value::F64(a) => Some(Value::F64(-a)),
            _ => None,
        }
    }

    /// Perform logical not
    pub fn not(&self) -> Value {
        Value::Bool(!self.is_truthy())
    }

    /// Equality comparison
    pub fn eq(&self, other: &Value) -> Value {
        Value::Bool(match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,

            (Value::I32(a), Value::I32(b)) => a == b,
            (Value::I64(a), Value::I64(b)) => a == b,
            (Value::F32(a), Value::F32(b)) => (a - b).abs() < f32::EPSILON,

            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::None, Value::None) => true,
            _ => false,
        })
    }

    /// Less than comparison
    pub fn lt(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(Value::Bool(a < b)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Bool(a < b)),

            (Value::I32(a), Value::I32(b)) => Some(Value::Bool(a < b)),
            (Value::I64(a), Value::I64(b)) => Some(Value::Bool(a < b)),
            (Value::F32(a), Value::F32(b)) => Some(Value::Bool(a < b)),
            (Value::String(a), Value::String(b)) => Some(Value::Bool(a < b)),

            // None comparisons: treat None as 0 for numeric comparisons
            (Value::None, Value::Int(b)) => Some(Value::Bool(&0 < b)),
            (Value::Int(a), Value::None) => Some(Value::Bool(a < &0)),
            (Value::None, Value::None) => Some(Value::Bool(false)),
            _ => None,
        }
    }

    /// Less than or equal comparison
    pub fn lte(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(Value::Bool(a <= b)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Bool(a <= b)),

            (Value::I32(a), Value::I32(b)) => Some(Value::Bool(a <= b)),
            (Value::I64(a), Value::I64(b)) => Some(Value::Bool(a <= b)),
            (Value::F32(a), Value::F32(b)) => Some(Value::Bool(a <= b)),

            // None comparisons
            (Value::None, Value::Int(b)) => Some(Value::Bool(&0 <= b)),
            (Value::Int(a), Value::None) => Some(Value::Bool(a <= &0)),
            (Value::None, Value::None) => Some(Value::Bool(true)),
            _ => None,
        }
    }

    /// Greater than comparison
    pub fn gt(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(Value::Bool(a > b)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Bool(a > b)),

            (Value::I32(a), Value::I32(b)) => Some(Value::Bool(a > b)),
            (Value::I64(a), Value::I64(b)) => Some(Value::Bool(a > b)),
            (Value::F32(a), Value::F32(b)) => Some(Value::Bool(a > b)),

            // None comparisons: treat None as 0 for numeric comparisons
            (Value::None, Value::Int(b)) => Some(Value::Bool(&0 > b)),
            (Value::Int(a), Value::None) => Some(Value::Bool(a > &0)),
            (Value::None, Value::None) => Some(Value::Bool(false)),
            _ => None,
        }
    }

    /// Greater than or equal comparison
    pub fn gte(&self, other: &Value) -> Option<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(Value::Bool(a >= b)),
            (Value::Float(a), Value::Float(b)) => Some(Value::Bool(a >= b)),

            (Value::I32(a), Value::I32(b)) => Some(Value::Bool(a >= b)),
            (Value::I64(a), Value::I64(b)) => Some(Value::Bool(a >= b)),
            (Value::F32(a), Value::F32(b)) => Some(Value::Bool(a >= b)),

            // None comparisons
            (Value::None, Value::Int(b)) => Some(Value::Bool(&0 >= b)),
            (Value::Int(a), Value::None) => Some(Value::Bool(a >= &0)),
            (Value::None, Value::None) => Some(Value::Bool(true)),
            _ => None,
        }
    }
}
