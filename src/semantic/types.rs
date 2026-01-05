//! Type system for Zyra

use crate::parser::ast;
use std::collections::HashMap;

/// Runtime type representation
#[derive(Debug, Clone, PartialEq)]
pub enum ZyraType {
    // Signed integers
    I8,
    I32,
    I64,

    // Unsigned integers
    U8,
    U32,
    U64,

    // Floats
    F32,
    F64,

    // Other primitives
    Bool,
    Char,
    String,

    // Collections
    Vec(Box<ZyraType>),
    Array {
        elem: Box<ZyraType>,
        size: usize,
    },

    // Legacy/Complex
    Object(HashMap<String, ZyraType>),
    Function {
        params: Vec<ZyraType>,
        return_type: Box<ZyraType>,
    },
    Reference {
        lifetime: Option<String>,
        mutable: bool,
        inner: Box<ZyraType>,
    },
    Option(Box<ZyraType>),
    Result {
        ok_type: Box<ZyraType>,
        err_type: Box<ZyraType>,
    },

    // Thread-safe types
    /// Atomic wrapper for lock-free shared state
    Atomic(Box<ZyraType>),
    /// Thread-safe reference-counted pointer
    Arc(Box<ZyraType>),
    /// Mutual exclusion lock for exclusive access
    Mutex(Box<ZyraType>),
    /// Read-write lock for concurrent reads
    RwLock(Box<ZyraType>),
    /// Thread-safe channel for message passing
    Channel(Box<ZyraType>),

    /// User-defined struct type
    Struct(String),
    /// User-defined enum type
    Enum(String),
    /// Closure type with captured variables
    Closure {
        params: Vec<ZyraType>,
        return_type: Box<ZyraType>,
    },

    Void,
    Never,
    Unknown,
}

impl ZyraType {
    pub fn from_ast_type(ast_type: &ast::Type) -> ZyraType {
        match ast_type {
            // Signed
            ast::Type::I8 => ZyraType::I8,
            ast::Type::I32 => ZyraType::I32,
            ast::Type::I64 => ZyraType::I64,
            ast::Type::Int => ZyraType::I32, // Alias Int -> i32 (32-bit default for memory efficiency)

            // Unsigned
            ast::Type::U8 => ZyraType::U8,
            ast::Type::U32 => ZyraType::U32,
            ast::Type::U64 => ZyraType::U64,

            // Floats
            ast::Type::F32 => ZyraType::F32,
            ast::Type::F64 => ZyraType::F64,
            ast::Type::Float => ZyraType::F32, // Alias Float -> f32 (32-bit default for memory efficiency)

            // Primitives
            ast::Type::Char => ZyraType::Char,
            ast::Type::Bool => ZyraType::Bool,
            ast::Type::String => ZyraType::String,

            // Collections
            ast::Type::Vec(inner) => ZyraType::Vec(Box::new(Self::from_ast_type(inner))),
            ast::Type::Array { elem, size } => ZyraType::Array {
                elem: Box::new(Self::from_ast_type(elem)),
                size: *size,
            },
            ast::Type::List(inner) => ZyraType::Vec(Box::new(Self::from_ast_type(inner))),

            ast::Type::Object => ZyraType::Object(HashMap::new()),

            // Centralized type resolution for Named types
            // This is the single source of truth for mapping type names to ZyraTypes
            ast::Type::Named(name) => Self::resolve_type_name(name),
            ast::Type::Reference {
                lifetime,
                mutable,
                inner,
            } => ZyraType::Reference {
                lifetime: lifetime.clone(),
                mutable: *mutable,
                inner: Box::new(Self::from_ast_type(inner)),
            },
            ast::Type::Inferred => ZyraType::Unknown,
            ast::Type::SelfType => ZyraType::Unknown, // Self refers to implementing type
            ast::Type::LifetimeAnnotated { lifetime: _, inner } => {
                // For now, just convert the inner type - lifetime is used for checking
                Self::from_ast_type(inner)
            }
        }
    }

    /// Resolve a type name string to a ZyraType.
    /// This is the SINGLE SOURCE OF TRUTH for all type name resolution.
    /// All paths (parser Named types, identifier lookups, etc.) should use this.
    pub fn resolve_type_name(name: &str) -> ZyraType {
        match name {
            // Signed integers (lowercase and explicit)
            "int" | "Int" => ZyraType::I32, // Int is 32-bit by default (memory efficient)
            "i8" | "I8" => ZyraType::I8,
            "i32" | "I32" => ZyraType::I32,
            "i64" | "I64" => ZyraType::I64,

            // Unsigned integers
            "u8" | "U8" => ZyraType::U8,
            "u32" | "U32" => ZyraType::U32,
            "u64" | "U64" => ZyraType::U64,

            // Floating point (lowercase and explicit)
            "float" | "Float" => ZyraType::F32, // Float is 32-bit by default (memory efficient)
            "f32" | "F32" => ZyraType::F32,
            "f64" | "F64" => ZyraType::F64,

            // Other primitives
            "bool" | "Bool" => ZyraType::Bool,
            "char" | "Char" => ZyraType::Char,
            "string" | "String" => ZyraType::String,

            // Special types
            "object" | "Object" => ZyraType::Object(HashMap::new()),
            "void" | "Void" | "()" => ZyraType::Void,
            "never" | "Never" | "!" => ZyraType::Never,

            // User-defined types: treat as Struct
            // The type registry (SemanticAnalyzer.types) validates if they exist
            // No uppercase/lowercase heuristic - proper validation is in semantic phase
            other => ZyraType::Struct(other.to_string()),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            ZyraType::I8
                | ZyraType::I32
                | ZyraType::I64
                | ZyraType::U8
                | ZyraType::U32
                | ZyraType::U64
                | ZyraType::F32
                | ZyraType::F64
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            ZyraType::I8
                | ZyraType::I32
                | ZyraType::I64
                | ZyraType::U8
                | ZyraType::U32
                | ZyraType::U64
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, ZyraType::F32 | ZyraType::F64)
    }

    /// Returns true if this type is a Copy type (stack-only, never refcounted).
    /// Copy types are passed by value and can be used multiple times without move.
    pub fn is_copy_type(&self) -> bool {
        matches!(
            self,
            ZyraType::I8
                | ZyraType::I32
                | ZyraType::I64
                | ZyraType::U8
                | ZyraType::U32
                | ZyraType::U64
                | ZyraType::F32
                | ZyraType::F64
                | ZyraType::Bool
                | ZyraType::Char
                | ZyraType::Void
                | ZyraType::Never
        )
    }

    /// Returns true if this type is a Reference type (heap-only, always refcounted).
    /// Reference types are heap-allocated and tracked by reference counting.
    pub fn is_reference_type(&self) -> bool {
        matches!(
            self,
            ZyraType::String
                | ZyraType::Vec(_)
                | ZyraType::Array { .. }
                | ZyraType::Object(_)
                | ZyraType::Struct(_)
                | ZyraType::Enum(_)
                | ZyraType::Option(_)
                | ZyraType::Result { .. }
                | ZyraType::Arc(_)
                | ZyraType::Mutex(_)
                | ZyraType::RwLock(_)
                | ZyraType::Channel(_)
        )
    }

    pub fn is_compatible(&self, other: &ZyraType) -> bool {
        match (self, other) {
            (ZyraType::Unknown, _) | (_, ZyraType::Unknown) => true,

            // Exact matches for primitives
            (ZyraType::I8, ZyraType::I8) => true,
            (ZyraType::I32, ZyraType::I32) => true,
            (ZyraType::I64, ZyraType::I64) => true,
            (ZyraType::U8, ZyraType::U8) => true,
            (ZyraType::U32, ZyraType::U32) => true,
            (ZyraType::U64, ZyraType::U64) => true,
            (ZyraType::F32, ZyraType::F32) => true,
            (ZyraType::F64, ZyraType::F64) => true,

            // Implicit widening: smaller types can coerce to larger types
            // Integer widening: i8 → i32 → i64, u8 → u32 → u64
            (ZyraType::I8, ZyraType::I32) => true,
            (ZyraType::I8, ZyraType::I64) => true,
            (ZyraType::I32, ZyraType::I64) => true,
            (ZyraType::U8, ZyraType::U32) => true,
            (ZyraType::U8, ZyraType::U64) => true,
            (ZyraType::U32, ZyraType::U64) => true,

            // Float widening: f32 → f64
            (ZyraType::F32, ZyraType::F64) => true,

            // Implicit narrowing: larger types can coerce to smaller (may lose precision)
            // Integer narrowing: i64 → i32 → i8, u64 → u32 → u8
            (ZyraType::I64, ZyraType::I32) => true,
            (ZyraType::I64, ZyraType::I8) => true,
            (ZyraType::I32, ZyraType::I8) => true,
            (ZyraType::U64, ZyraType::U32) => true,
            (ZyraType::U64, ZyraType::U8) => true,
            (ZyraType::U32, ZyraType::U8) => true,

            // Float narrowing: f64 → f32
            (ZyraType::F64, ZyraType::F32) => true,

            (ZyraType::Char, ZyraType::Char) => true,
            (ZyraType::Bool, ZyraType::Bool) => true,
            (ZyraType::String, ZyraType::String) => true,
            (ZyraType::Void, ZyraType::Void) => true,

            // Allow Void compatibility (legacy logic)
            (ZyraType::Void, _) => true,

            // Collections
            (ZyraType::Vec(a), ZyraType::Vec(b)) => a.is_compatible(b),
            (ZyraType::Array { elem: a, size: sa }, ZyraType::Array { elem: b, size: sb }) => {
                sa == sb && a.is_compatible(b)
            }

            // References
            (ZyraType::Reference { inner: a, .. }, ZyraType::Reference { inner: b, .. }) => {
                a.is_compatible(b)
            }
            // Auto-deref / borrowing compatibility checks can be added here if strictness allows

            // Option/Result
            (ZyraType::Option(a), ZyraType::Option(b)) => a.is_compatible(b),
            (
                ZyraType::Result {
                    ok_type: a,
                    err_type: e1,
                },
                ZyraType::Result {
                    ok_type: b,
                    err_type: e2,
                },
            ) => a.is_compatible(b) && e1.is_compatible(e2),

            // User-defined nominal types: compatible if names match (nominal equality)
            (ZyraType::Struct(a), ZyraType::Struct(b)) => a == b,
            (ZyraType::Enum(a), ZyraType::Enum(b)) => a == b,

            // Cross-compatibility: user-defined types may be declared as Struct but return Enum
            // This handles the case where `-> Result` (Struct) is compatible with `Result::Ok(...)` (Enum)
            (ZyraType::Struct(a), ZyraType::Enum(b)) => a == b,
            (ZyraType::Enum(a), ZyraType::Struct(b)) => a == b,

            // Object types: structural compatibility
            (ZyraType::Object(fields_a), ZyraType::Object(fields_b)) => {
                // Empty objects are compatible with any object (placeholder semantics)
                if fields_a.is_empty() || fields_b.is_empty() {
                    return true;
                }
                // Structural compatibility: all fields in b must be compatible with fields in a
                fields_b.iter().all(|(key, type_b)| {
                    fields_a
                        .get(key)
                        .map(|type_a| type_a.is_compatible(type_b))
                        .unwrap_or(false)
                })
            }

            // Concurrency types
            (ZyraType::Mutex(a), ZyraType::Mutex(b)) => a.is_compatible(b),
            (ZyraType::RwLock(a), ZyraType::RwLock(b)) => a.is_compatible(b),
            (ZyraType::Channel(a), ZyraType::Channel(b)) => a.is_compatible(b),

            _ => false,
        }
    }

    /// Check if this type can be explicitly cast to another type using `as`
    /// This is more permissive than is_compatible - allows widening, narrowing, float/int conversion
    pub fn is_castable(&self, target: &ZyraType) -> bool {
        // Same type - always castable
        if self == target {
            return true;
        }

        // Unknown is always castable (for backwards compat)
        if matches!(self, ZyraType::Unknown) || matches!(target, ZyraType::Unknown) {
            return true;
        }

        match (self, target) {
            // All numeric types can be cast to each other
            (from, to) if from.is_numeric() && to.is_numeric() => true,

            // Numeric to String (for display) - future: use ToString trait
            // For now, don't allow - use format!() instead

            // Char to Int types and vice versa
            (ZyraType::Char, to) if to.is_numeric() => true,
            (from, ZyraType::Char) if from.is_numeric() => true,

            // Bool to Int (true=1, false=0)
            (ZyraType::Bool, to) if to.is_numeric() => true,

            // User-defined types - same name means castable
            (ZyraType::Struct(a), ZyraType::Struct(b)) => a == b,
            (ZyraType::Enum(a), ZyraType::Enum(b)) => a == b,
            (ZyraType::Struct(a), ZyraType::Enum(b)) => a == b,
            (ZyraType::Enum(a), ZyraType::Struct(b)) => a == b,

            // All other casts are not allowed
            _ => false,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            ZyraType::I8 => "i8".to_string(),
            ZyraType::I32 => "i32".to_string(),
            ZyraType::I64 => "i64".to_string(),
            ZyraType::U8 => "u8".to_string(),
            ZyraType::U32 => "u32".to_string(),
            ZyraType::U64 => "u64".to_string(),
            ZyraType::F32 => "f32".to_string(),
            ZyraType::F64 => "f64".to_string(),
            ZyraType::Char => "char".to_string(),
            ZyraType::Bool => "Bool".to_string(),
            ZyraType::String => "String".to_string(),

            ZyraType::Vec(inner) => format!("Vec<{}>", inner.display_name()),
            ZyraType::Array { elem, size } => format!("[{}; {}]", elem.display_name(), size),

            ZyraType::Object(_) => "Object".to_string(),
            ZyraType::Function {
                params,
                return_type,
            } => {
                let params_str: Vec<_> = params.iter().map(|p| p.display_name()).collect();
                format!(
                    "func({}) -> {}",
                    params_str.join(", "),
                    return_type.display_name()
                )
            }
            ZyraType::Reference {
                lifetime,
                mutable,
                inner,
            } => {
                let mut s = String::from("&");
                if let Some(lt) = lifetime {
                    s.push('\'');
                    s.push_str(lt);
                    s.push(' ');
                }
                if *mutable {
                    s.push_str("mut ");
                }
                s.push_str(&inner.display_name());
                s
            }
            ZyraType::Option(inner) => format!("Option<{}>", inner.display_name()),
            ZyraType::Result { ok_type, err_type } => {
                format!(
                    "Result<{}, {}>",
                    ok_type.display_name(),
                    err_type.display_name()
                )
            }
            ZyraType::Atomic(inner) => format!("Atomic<{}>", inner.display_name()),
            ZyraType::Arc(inner) => format!("Arc<{}>", inner.display_name()),
            ZyraType::Mutex(inner) => format!("Mutex<{}>", inner.display_name()),
            ZyraType::RwLock(inner) => format!("RwLock<{}>", inner.display_name()),
            ZyraType::Channel(inner) => format!("Channel<{}>", inner.display_name()),
            ZyraType::Struct(name) => name.clone(),
            ZyraType::Enum(name) => name.clone(),
            ZyraType::Closure {
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|p| p.display_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("|{}| -> {}", params_str, return_type.display_name())
            }
            ZyraType::Void => "Void".to_string(),
            ZyraType::Never => "Never".to_string(),
            ZyraType::Unknown => "Unknown".to_string(),
        }
    }

    /// Strict type compatibility check
    /// - Unknown types are NOT compatible in strict mode
    /// - References must match mutability and lifetime
    /// - Arrays must match exact size
    pub fn is_compatible_strict(&self, other: &ZyraType) -> bool {
        match (self, other) {
            // Unknown is NOT compatible in strict mode
            (ZyraType::Unknown, _) | (_, ZyraType::Unknown) => false,

            // Exact matches for primitives
            (ZyraType::I8, ZyraType::I8) => true,
            (ZyraType::I32, ZyraType::I32) => true,
            (ZyraType::I64, ZyraType::I64) => true,
            (ZyraType::U8, ZyraType::U8) => true,
            (ZyraType::U32, ZyraType::U32) => true,
            (ZyraType::U64, ZyraType::U64) => true,
            (ZyraType::F32, ZyraType::F32) => true,
            (ZyraType::F64, ZyraType::F64) => true,
            (ZyraType::Char, ZyraType::Char) => true,
            (ZyraType::Bool, ZyraType::Bool) => true,
            (ZyraType::String, ZyraType::String) => true,
            (ZyraType::Void, ZyraType::Void) => true,

            // Collections: strict element type AND size matching
            (ZyraType::Vec(a), ZyraType::Vec(b)) => a.is_compatible_strict(b),
            (ZyraType::Array { elem: a, size: sa }, ZyraType::Array { elem: b, size: sb }) => {
                sa == sb && a.is_compatible_strict(b)
            }

            // References: check mutability and lifetime
            (
                ZyraType::Reference {
                    lifetime: l1,
                    mutable: m1,
                    inner: i1,
                },
                ZyraType::Reference {
                    lifetime: l2,
                    mutable: m2,
                    inner: i2,
                },
            ) => {
                // Cannot assign &T to &mut T
                let mut_ok = *m1 == *m2 || (!*m2 && *m1); // &mut can become &, not vice versa
                                                          // Lifetimes must match if both specified
                let lt_ok = l1 == l2 || l1.is_none() || l2.is_none();
                mut_ok && lt_ok && i1.is_compatible_strict(i2)
            }

            // Option/Result
            (ZyraType::Option(a), ZyraType::Option(b)) => a.is_compatible_strict(b),
            (
                ZyraType::Result {
                    ok_type: a,
                    err_type: e1,
                },
                ZyraType::Result {
                    ok_type: b,
                    err_type: e2,
                },
            ) => a.is_compatible_strict(b) && e1.is_compatible_strict(e2),

            _ => false,
        }
    }

    /// Check reference compatibility with mutability and lifetime rules
    /// Returns (compatible, error_reason)
    pub fn is_reference_compatible(&self, other: &ZyraType) -> (bool, Option<String>) {
        match (self, other) {
            (
                ZyraType::Reference {
                    lifetime: l1,
                    mutable: m1,
                    inner: i1,
                },
                ZyraType::Reference {
                    lifetime: l2,
                    mutable: m2,
                    inner: i2,
                },
            ) => {
                // Rule 1: Cannot assign &T to &mut T
                if *m2 && !*m1 {
                    return (
                        false,
                        Some("cannot assign immutable reference to mutable reference".to_string()),
                    );
                }

                // Rule 2: Lifetime must be compatible (source must outlive target)
                if let (Some(lt1), Some(lt2)) = (l1, l2) {
                    if lt1 != lt2 {
                        return (
                            false,
                            Some(format!("lifetime '{}' does not match '{}'", lt1, lt2)),
                        );
                    }
                }

                // Rule 3: Inner types must be compatible
                if !i1.is_compatible_strict(i2) {
                    return (
                        false,
                        Some(format!(
                            "inner type {} is not compatible with {}",
                            i1.display_name(),
                            i2.display_name()
                        )),
                    );
                }

                (true, None)
            }
            _ => (false, Some("not a reference type".to_string())),
        }
    }

    /// Check if this type is Unknown
    pub fn is_unknown(&self) -> bool {
        matches!(self, ZyraType::Unknown)
    }
}
