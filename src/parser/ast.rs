//! Abstract Syntax Tree definitions for Zyra

use crate::lexer::Span;

/// A complete Zyra program
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// Statement types
#[derive(Debug, Clone)]
pub enum Statement {
    /// Variable declaration: let [mut] name [: Type] = expr;
    Let {
        name: String,
        mutable: bool,
        type_annotation: Option<Type>,
        value: Expression,
        span: Span,
    },

    /// Function declaration: func name<'a>(params) -> Type { body }
    Function {
        name: String,
        lifetimes: Vec<String>,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Block,
        span: Span,
    },

    /// Expression statement: expr;
    Expression { expr: Expression, span: Span },

    /// Import statement: example import std::game::{Graphics, Window}
    Import {
        path: Vec<String>,  // ["std", "game"]
        items: Vec<String>, // ["Graphics", "Window"] or empty for all
        span: Span,
    },

    /// Return statement: return expr;
    Return {
        value: Option<Expression>,
        span: Span,
    },

    /// If statement: if condition { } else { }
    If {
        condition: Expression,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },

    /// While loop: while condition { }
    While {
        condition: Expression,
        body: Block,
        span: Span,
    },

    /// For loop: for name in start..end { } or for name in start..=end { }
    For {
        variable: String,
        start: Expression,
        end: Expression,
        inclusive: bool,
        body: Block,
        span: Span,
    },

    /// Block of statements
    Block(Block),

    /// Struct definition: struct Name { field: Type, ... }
    Struct {
        name: String,
        fields: Vec<StructField>,
        span: Span,
    },

    /// Enum definition: enum Name { Variant1, Variant2(Type), ... }
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        span: Span,
    },

    /// Impl block: impl Name { methods... }
    Impl {
        target_type: String,
        trait_name: Option<String>, // Some for trait impl, None for inherent impl
        methods: Vec<Box<Statement>>,
        span: Span,
    },

    /// Trait definition: trait Name { method signatures... }
    Trait {
        name: String,
        methods: Vec<TraitMethod>,
        span: Span,
    },
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
    pub span: Span,
}

/// Block of statements with optional trailing expression
#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub expression: Option<Box<Expression>>,
    pub span: Span,
}

/// Struct field definition
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
    pub span: Span,
}

/// Enum variant definition
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<Vec<Type>>, // None = unit variant, Some([]) = tuple variant
    pub span: Span,
}

/// Trait method signature
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub default_impl: Option<Block>, // Some = has default implementation
    pub span: Span,
}

/// Expression types
#[derive(Debug, Clone)]
pub enum Expression {
    /// Integer literal
    Int { value: i64, span: Span },

    /// Float literal
    Float { value: f64, span: Span },

    /// Boolean literal
    Bool { value: bool, span: Span },

    /// Character literal
    Char { value: char, span: Span },

    /// String literal
    String { value: String, span: Span },

    /// Variable reference
    Identifier { name: String, span: Span },

    /// Binary operation: a + b, a && b, etc.
    Binary {
        left: Box<Expression>,
        operator: BinaryOp,
        right: Box<Expression>,
        span: Span,
    },

    /// Unary operation: -a, !a
    Unary {
        operator: UnaryOp,
        operand: Box<Expression>,
        span: Span,
    },

    /// Assignment: a = b
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
        span: Span,
    },

    /// Function call: func(args)
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
        span: Span,
    },

    /// Field access: obj.field
    FieldAccess {
        object: Box<Expression>,
        field: String,
        span: Span,
    },

    /// Index access: arr[index]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },

    /// List literal: [a, b, c]
    List {
        elements: Vec<Expression>,
        span: Span,
    },

    /// Object literal: { field: value }
    Object {
        fields: Vec<(String, Expression)>,
        span: Span,
    },

    /// Reference: &expr, &mut expr
    Reference {
        mutable: bool,
        value: Box<Expression>,
        span: Span,
    },

    /// Dereference: *expr
    Dereference { value: Box<Expression>, span: Span },

    /// Range: start..end
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        span: Span,
    },

    /// Grouped expression: (expr)
    Grouped { inner: Box<Expression>, span: Span },

    /// If expression: if condition { } else { } (used as trailing expression)
    If {
        condition: Box<Expression>,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },

    /// Struct instantiation: StructName { field: value, ... }
    StructInit {
        name: String,
        fields: Vec<(String, Expression)>,
        span: Span,
    },

    /// Enum variant: EnumName::Variant or EnumName::Variant(data)
    EnumVariant {
        enum_name: String,
        variant: String,
        data: Option<Box<Expression>>,
        span: Span,
    },
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Int { span, .. } => *span,
            Expression::Float { span, .. } => *span,
            Expression::Bool { span, .. } => *span,
            Expression::Char { span, .. } => *span,
            Expression::String { span, .. } => *span,
            Expression::Identifier { span, .. } => *span,
            Expression::Binary { span, .. } => *span,
            Expression::Unary { span, .. } => *span,
            Expression::Assignment { span, .. } => *span,
            Expression::Call { span, .. } => *span,
            Expression::FieldAccess { span, .. } => *span,
            Expression::Index { span, .. } => *span,
            Expression::List { span, .. } => *span,
            Expression::Object { span, .. } => *span,
            Expression::Reference { span, .. } => *span,
            Expression::Dereference { span, .. } => *span,
            Expression::Range { span, .. } => *span,
            Expression::Grouped { span, .. } => *span,
            Expression::If { span, .. } => *span,
            Expression::StructInit { span, .. } => *span,
            Expression::EnumVariant { span, .. } => *span,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
}

impl BinaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Modulo => "%",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Negate,
    Not,
}

/// Type annotations
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Signed integers
    I8,
    I32,
    I64,
    /// Int is alias for I32
    Int,

    // Unsigned integers
    U8,
    U32,
    U64,

    // Floating point
    F32,
    F64,
    /// Float is alias for F64
    Float,

    // Character
    Char,

    // Boolean
    Bool,

    // String
    String,

    // Collections
    /// Vec<T> - dynamic vector
    Vec(Box<Type>),
    /// [T; N] - fixed-size array
    Array {
        elem: Box<Type>,
        size: usize,
    },
    /// List<T> - legacy alias for Vec
    List(Box<Type>),

    /// Object type (structural)
    Object,
    /// User-defined type
    Named(String),
    /// Reference type: &T, &'a T
    Reference {
        lifetime: Option<String>,
        mutable: bool,
        inner: Box<Type>,
    },
    /// Self type (for &self, &mut self)
    SelfType,
    /// Lifetime-annotated type: 'a Type
    LifetimeAnnotated {
        lifetime: String,
        inner: Box<Type>,
    },
    /// Inferred type (placeholder)
    Inferred,
}

impl Type {
    pub fn as_str(&self) -> String {
        match self {
            // Signed integers
            Type::I8 => "i8".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::Int => "Int".to_string(),

            // Unsigned integers
            Type::U8 => "u8".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),

            // Floats
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Float => "Float".to_string(),

            // Other primitives
            Type::Char => "char".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::String => "String".to_string(),

            // Collections
            Type::Vec(inner) => format!("Vec<{}>", inner.as_str()),
            Type::Array { elem, size } => format!("[{}; {}]", elem.as_str(), size),
            Type::List(inner) => format!("List<{}>", inner.as_str()),

            Type::Object => "Object".to_string(),
            Type::Named(name) => name.clone(),
            Type::Reference {
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
                s.push_str(&inner.as_str());
                s
            }
            Type::SelfType => "Self".to_string(),
            Type::LifetimeAnnotated { lifetime, inner } => {
                format!("'{} {}", lifetime, inner.as_str())
            }
            Type::Inferred => "_".to_string(),
        }
    }
}
