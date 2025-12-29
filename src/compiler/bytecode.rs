//! Bytecode definitions for Zyra VM

use std::fmt;

/// Bytecode instruction set
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Load a constant value onto the stack
    LoadConst(Value),

    /// Load a variable's value onto the stack
    LoadVar(String),

    /// Store the top of stack into a variable
    StoreVar(String),

    /// Pop the top value from the stack
    Pop,

    // Arithmetic operations
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,

    // Comparison operations
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,

    // Logical operations
    And,
    Or,
    Not,

    // Control flow
    Jump(usize),
    JumpIfFalse(usize),

    // Function operations
    Call(String, usize), // function name, arg count
    Return,

    // Memory management instructions (ownership & borrowing)
    /// Allocate heap memory for a value, returns heap ID
    Alloc,
    /// Move ownership from one variable to another
    Move(String, String), // from, to
    /// Create an immutable borrow (reference)
    BorrowShared(String), // source variable
    /// Create a mutable borrow (reference)
    BorrowMut(String), // source variable
    /// Explicitly drop/free a variable
    Drop(String),
    /// End a borrow (reference goes out of scope)
    EndBorrow(String),

    // Data structures
    MakeList(usize),   // element count
    MakeObject(usize), // field count
    GetField(String),
    SetField(String),
    GetIndex,
    SetIndex,

    // Scope management
    EnterScope,
    ExitScope,

    // Built-in operations
    Print,

    // No operation
    Nop,

    // Halt execution
    Halt,
}

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Signed integers
    I8(i8),
    I32(i32),
    I64(i64),
    /// Legacy alias for I64, will be deprecated
    Int(i64),

    // Unsigned integers
    U8(u8),
    U32(u32),
    U64(u64),

    // Floats
    F32(f32),
    F64(f64),
    /// Legacy alias for F64, will be deprecated
    Float(f64),

    // Primitives
    Bool(bool),
    Char(char),
    String(String),

    // Collections
    // Vec is alias for List basically, but let's be explicitly Rust-like
    Vec(Vec<Value>),
    List(Vec<Value>),  // Legacy
    Array(Vec<Value>), // Fixed size (runtime representation same as Vec)

    Object(std::collections::HashMap<String, Value>),
    Function {
        name: String,
        params: Vec<String>,
        address: usize,
    },
    // None replaces Null - represents absence of a value
    None,
    // Option type: Some wraps a value
    Some(Box<Value>),
    // Result type: Ok wraps a success value
    Ok(Box<Value>),
    // Result type: Err wraps an error value
    Err(Box<Value>),
    // Special values for VM
    Reference {
        name: String,
        mutable: bool,
    },
    Window(WindowState),
}

/// Window state for game module
#[derive(Debug, Clone, PartialEq)]
pub struct WindowState {
    pub width: usize,
    pub height: usize,
    pub title: String,
    pub buffer: Vec<u32>,
    pub is_open: bool,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::I8(_) => "i8",
            Value::I32(_) => "i32",
            Value::I64(_) => "i64",
            Value::Int(_) => "Int",
            Value::U8(_) => "u8",
            Value::U32(_) => "u32",
            Value::U64(_) => "u64",
            Value::F32(_) => "f32",
            Value::F64(_) => "f64",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Char(_) => "char",
            Value::String(_) => "String",
            Value::Vec(_) => "Vec",
            Value::List(_) => "List",
            Value::Array(_) => "Array",
            Value::Object(_) => "Object",
            Value::Function { .. } => "Function",
            Value::None => "None",
            Value::Some(_) => "Some",
            Value::Ok(_) => "Ok",
            Value::Err(_) => "Err",
            Value::Reference { .. } => "Reference",
            Value::Window(_) => "Window",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::I8(n) => *n != 0,
            Value::I32(n) => *n != 0,
            Value::I64(n) => *n != 0,
            Value::U8(n) => *n != 0,
            Value::U32(n) => *n != 0,
            Value::U64(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::F32(n) => *n != 0.0,
            Value::F64(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Vec(l) => !l.is_empty(),
            Value::Array(l) => !l.is_empty(),
            Value::None => false,
            _ => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I8(n) => write!(f, "{}", n),
            Value::I32(n) => write!(f, "{}", n),
            Value::I64(n) => write!(f, "{}", n),
            Value::Int(n) => write!(f, "{}", n),
            Value::U8(n) => write!(f, "{}", n),
            Value::U32(n) => write!(f, "{}", n),
            Value::U64(n) => write!(f, "{}", n),
            Value::F32(n) => write!(f, "{}", n),
            Value::F64(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "{}", c),
            Value::String(s) => write!(f, "{}", s),
            Value::Vec(items) | Value::List(items) | Value::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Object(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function { name, .. } => write!(f, "<function {}>", name),
            Value::None => write!(f, "None"),
            Value::Some(inner) => write!(f, "Some({})", inner),
            Value::Ok(inner) => write!(f, "Ok({})", inner),
            Value::Err(inner) => write!(f, "Err({})", inner),
            Value::Reference { name, mutable } => {
                write!(f, "&{}{}", if *mutable { "mut " } else { "" }, name)
            }
            Value::Window(state) => {
                write!(
                    f,
                    "<Window {}x{} '{}'>",
                    state.width, state.height, state.title
                )
            }
        }
    }
}

/// Compiled bytecode program
#[derive(Debug, Clone)]
pub struct Bytecode {
    pub instructions: Vec<Instruction>,
    pub functions: std::collections::HashMap<String, FunctionDef>,
}

/// Function definition in bytecode
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub start_address: usize,
    pub end_address: usize,
}

impl Bytecode {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            functions: std::collections::HashMap::new(),
        }
    }

    pub fn emit(&mut self, instruction: Instruction) -> usize {
        let addr = self.instructions.len();
        self.instructions.push(instruction);
        addr
    }

    pub fn current_address(&self) -> usize {
        self.instructions.len()
    }

    pub fn patch_jump(&mut self, addr: usize, target: usize) {
        match &mut self.instructions[addr] {
            Instruction::Jump(dest) => *dest = target,
            Instruction::JumpIfFalse(dest) => *dest = target,
            _ => panic!("Tried to patch non-jump instruction"),
        }
    }

    /// Serialize bytecode to bytes
    pub fn serialize(&self) -> Vec<u8> {
        // Simple serialization for build command
        // In production, use a proper binary format
        let mut output = Vec::new();

        // Magic number "ZYRA"
        output.extend_from_slice(b"ZYRA");

        // Version
        output.push(0);
        output.push(1);

        // Instruction count
        let count = self.instructions.len() as u32;
        output.extend_from_slice(&count.to_le_bytes());

        // For now, just store the instruction count
        // Full serialization would encode each instruction

        output
    }
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}
