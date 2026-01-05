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
    /// Method call: method name, arg count (receiver is pushed first, then args)
    MethodCall(String, usize),
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
    MakeList(usize),   // Array (fixed size): element count
    MakeVec(usize),    // Vec (dynamic): element count
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

    // Stack operations for pattern matching
    /// Duplicate the top of stack
    Dup,
    /// Check if string contains substring: stack [string, substr] => bool
    StrContains,

    // Halt execution
    Halt,

    // Type casting
    /// Cast top of stack to target type (type name as string)
    Cast(String),

    // Closures
    /// Create a closure: MakeClosure(function_name, param_count)
    MakeClosure {
        func_name: String,
        param_count: usize,
    },
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

    /// Reference to heap-allocated object (Struct, Enum, Vec, String)
    /// The usize is the HeapId for lookup in the VM's heap
    Ref(usize),

    /// Closure value with function name and captured environment
    Closure {
        func_name: String,
        param_count: usize,
    },
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
            Value::Ref(_) => "Ref",
            Value::Closure { .. } => "Closure",
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
            Value::Ref(id) => write!(f, "<Ref#{}>", id),
            Value::Closure {
                func_name,
                param_count,
            } => {
                write!(f, "<Closure {} ({} params)>", func_name, param_count)
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

    /// Serialize bytecode to bytes for .zyc file format
    pub fn serialize(&self) -> Vec<u8> {
        let mut output = Vec::new();

        // Magic number "ZYRA"
        output.extend_from_slice(b"ZYRA");

        // Version (2 bytes)
        output.push(0);
        output.push(1);

        // Instruction count (4 bytes, little-endian)
        let count = self.instructions.len() as u32;
        output.extend_from_slice(&count.to_le_bytes());

        // Function count (4 bytes, little-endian)
        let func_count = self.functions.len() as u32;
        output.extend_from_slice(&func_count.to_le_bytes());

        // Serialize each function definition
        for (name, func_def) in &self.functions {
            Self::serialize_string(&mut output, name);
            output.extend_from_slice(&(func_def.params.len() as u32).to_le_bytes());
            for param in &func_def.params {
                Self::serialize_string(&mut output, param);
            }
            output.extend_from_slice(&(func_def.start_address as u32).to_le_bytes());
            output.extend_from_slice(&(func_def.end_address as u32).to_le_bytes());
        }

        // Serialize each instruction
        for instr in &self.instructions {
            Self::serialize_instruction(&mut output, instr);
        }

        output
    }

    fn serialize_instruction(output: &mut Vec<u8>, instr: &Instruction) {
        match instr {
            Instruction::LoadConst(value) => {
                output.push(0x01);
                Self::serialize_value(output, value);
            }
            Instruction::LoadVar(name) => {
                output.push(0x02);
                Self::serialize_string(output, name);
            }
            Instruction::StoreVar(name) => {
                output.push(0x03);
                Self::serialize_string(output, name);
            }
            Instruction::Pop => output.push(0x04),
            Instruction::Add => output.push(0x10),
            Instruction::Sub => output.push(0x11),
            Instruction::Mul => output.push(0x12),
            Instruction::Div => output.push(0x13),
            Instruction::Mod => output.push(0x14),
            Instruction::Neg => output.push(0x15),
            Instruction::Eq => output.push(0x20),
            Instruction::Neq => output.push(0x21),
            Instruction::Lt => output.push(0x22),
            Instruction::Lte => output.push(0x23),
            Instruction::Gt => output.push(0x24),
            Instruction::Gte => output.push(0x25),
            Instruction::And => output.push(0x30),
            Instruction::Or => output.push(0x31),
            Instruction::Not => output.push(0x32),
            Instruction::Jump(addr) => {
                output.push(0x40);
                output.extend_from_slice(&(*addr as u32).to_le_bytes());
            }
            Instruction::JumpIfFalse(addr) => {
                output.push(0x41);
                output.extend_from_slice(&(*addr as u32).to_le_bytes());
            }
            Instruction::Call(name, argc) => {
                output.push(0x50);
                Self::serialize_string(output, name);
                output.extend_from_slice(&(*argc as u32).to_le_bytes());
            }
            Instruction::MethodCall(method_name, argc) => {
                output.push(0x52);
                Self::serialize_string(output, method_name);
                output.extend_from_slice(&(*argc as u32).to_le_bytes());
            }
            Instruction::Return => output.push(0x51),
            Instruction::Alloc => output.push(0x60),
            Instruction::Move(from, to) => {
                output.push(0x61);
                Self::serialize_string(output, from);
                Self::serialize_string(output, to);
            }
            Instruction::BorrowShared(name) => {
                output.push(0x62);
                Self::serialize_string(output, name);
            }
            Instruction::BorrowMut(name) => {
                output.push(0x63);
                Self::serialize_string(output, name);
            }
            Instruction::Drop(name) => {
                output.push(0x64);
                Self::serialize_string(output, name);
            }
            Instruction::EndBorrow(name) => {
                output.push(0x65);
                Self::serialize_string(output, name);
            }
            Instruction::MakeList(count) => {
                output.push(0x70);
                output.extend_from_slice(&(*count as u32).to_le_bytes());
            }
            Instruction::MakeVec(count) => {
                output.push(0x76);
                output.extend_from_slice(&(*count as u32).to_le_bytes());
            }
            Instruction::MakeObject(count) => {
                output.push(0x71);
                output.extend_from_slice(&(*count as u32).to_le_bytes());
            }
            Instruction::GetField(name) => {
                output.push(0x72);
                Self::serialize_string(output, name);
            }
            Instruction::SetField(name) => {
                output.push(0x73);
                Self::serialize_string(output, name);
            }
            Instruction::GetIndex => output.push(0x74),
            Instruction::SetIndex => output.push(0x75),
            Instruction::EnterScope => output.push(0x80),
            Instruction::ExitScope => output.push(0x81),
            Instruction::Print => output.push(0x90),
            Instruction::Nop => output.push(0xFE),
            Instruction::Dup => output.push(0xA0),
            Instruction::StrContains => output.push(0xA1),
            Instruction::Halt => output.push(0xFF),
            Instruction::Cast(type_name) => {
                output.push(0xA2);
                Self::serialize_string(output, type_name);
            }
            Instruction::MakeClosure {
                func_name,
                param_count,
            } => {
                output.push(0xA3);
                Self::serialize_string(output, func_name);
                output.extend_from_slice(&(*param_count as u32).to_le_bytes());
            }
        }
    }

    fn serialize_value(output: &mut Vec<u8>, value: &Value) {
        match value {
            Value::None => output.push(0x00),
            Value::Bool(b) => {
                output.push(0x01);
                output.push(if *b { 1 } else { 0 });
            }
            Value::Int(n) | Value::I64(n) => {
                output.push(0x02);
                output.extend_from_slice(&n.to_le_bytes());
            }
            Value::I32(n) => {
                output.push(0x03);
                output.extend_from_slice(&n.to_le_bytes());
            }
            Value::Float(f) | Value::F64(f) => {
                output.push(0x04);
                output.extend_from_slice(&f.to_le_bytes());
            }
            Value::F32(f) => {
                output.push(0x05);
                output.extend_from_slice(&f.to_le_bytes());
            }
            Value::String(s) => {
                output.push(0x06);
                Self::serialize_string(output, s);
            }
            Value::Char(c) => {
                output.push(0x07);
                output.extend_from_slice(&(*c as u32).to_le_bytes());
            }
            Value::Vec(items) | Value::List(items) | Value::Array(items) => {
                output.push(0x08);
                output.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    Self::serialize_value(output, item);
                }
            }
            Value::Function {
                name,
                params,
                address,
            } => {
                output.push(0x10);
                Self::serialize_string(output, name);
                output.extend_from_slice(&(params.len() as u32).to_le_bytes());
                for param in params {
                    Self::serialize_string(output, param);
                }
                output.extend_from_slice(&(*address as u32).to_le_bytes());
            }
            // Complex types - serialize as None for now
            _ => output.push(0x00),
        }
    }

    fn serialize_string(output: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(bytes);
    }

    /// Deserialize bytecode from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < 14 {
            return Err("Invalid bytecode: too short".to_string());
        }

        // Check magic
        if &data[0..4] != b"ZYRA" {
            return Err("Invalid bytecode: bad magic number".to_string());
        }

        // Check version
        let version = (data[4] as u16) << 8 | data[5] as u16;
        if version != 1 {
            return Err(format!("Unsupported bytecode version: {}", version));
        }

        // Read instruction count
        let instr_count = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;

        // Read function count
        let func_count = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;

        let mut bytecode = Bytecode::new();
        let mut pos = 14;

        // Read function definitions
        for _ in 0..func_count {
            let (name, new_pos) = Self::deserialize_string(data, pos)?;
            pos = new_pos;

            if pos + 4 > data.len() {
                return Err("Unexpected end".to_string());
            }
            let param_count =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            let mut params = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                let (param, new_pos) = Self::deserialize_string(data, pos)?;
                params.push(param);
                pos = new_pos;
            }

            if pos + 8 > data.len() {
                return Err("Unexpected end".to_string());
            }
            let start_address =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;
            let end_address =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            bytecode.functions.insert(
                name.clone(),
                FunctionDef {
                    name,
                    params,
                    start_address,
                    end_address,
                },
            );
        }

        // Read instructions
        for _ in 0..instr_count {
            if pos >= data.len() {
                return Err("Unexpected end of bytecode".to_string());
            }
            let (instr, new_pos) = Self::deserialize_instruction(data, pos)?;
            bytecode.emit(instr);
            pos = new_pos;
        }

        Ok(bytecode)
    }

    fn deserialize_instruction(data: &[u8], pos: usize) -> Result<(Instruction, usize), String> {
        if pos >= data.len() {
            return Err("Unexpected end of bytecode".to_string());
        }

        let opcode = data[pos];
        let mut pos = pos + 1;

        let instr = match opcode {
            0x01 => {
                let (value, new_pos) = Self::deserialize_value(data, pos)?;
                pos = new_pos;
                Instruction::LoadConst(value)
            }
            0x02 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::LoadVar(name)
            }
            0x03 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::StoreVar(name)
            }
            0x04 => Instruction::Pop,
            0x10 => Instruction::Add,
            0x11 => Instruction::Sub,
            0x12 => Instruction::Mul,
            0x13 => Instruction::Div,
            0x14 => Instruction::Mod,
            0x15 => Instruction::Neg,
            0x20 => Instruction::Eq,
            0x21 => Instruction::Neq,
            0x22 => Instruction::Lt,
            0x23 => Instruction::Lte,
            0x24 => Instruction::Gt,
            0x25 => Instruction::Gte,
            0x30 => Instruction::And,
            0x31 => Instruction::Or,
            0x32 => Instruction::Not,
            0x40 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let addr =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::Jump(addr)
            }
            0x41 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let addr =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::JumpIfFalse(addr)
            }
            0x50 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let argc =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::Call(name, argc)
            }
            0x51 => Instruction::Return,
            0x52 => {
                let (method, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let arg_count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::MethodCall(method, arg_count)
            }
            0x60 => Instruction::Alloc,
            0x61 => {
                let (from, new_pos) = Self::deserialize_string(data, pos)?;
                let (to, new_pos) = Self::deserialize_string(data, new_pos)?;
                pos = new_pos;
                Instruction::Move(from, to)
            }
            0x62 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::BorrowShared(name)
            }
            0x63 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::BorrowMut(name)
            }
            0x64 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::Drop(name)
            }
            0x65 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::EndBorrow(name)
            }
            0x70 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::MakeList(count)
            }
            0x71 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::MakeObject(count)
            }
            0x72 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::GetField(name)
            }
            0x73 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::SetField(name)
            }
            0x74 => Instruction::GetIndex,
            0x75 => Instruction::SetIndex,
            0x76 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::MakeVec(count)
            }
            0x80 => Instruction::EnterScope,
            0x81 => Instruction::ExitScope,
            0x90 => Instruction::Print,
            0xFE => Instruction::Nop,
            0xFF => Instruction::Halt,
            0xA0 => Instruction::Dup,
            0xA1 => Instruction::StrContains,
            0xA2 => {
                let (type_name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Instruction::Cast(type_name)
            }
            0xA3 => {
                let (func_name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                let param_count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Instruction::MakeClosure {
                    func_name,
                    param_count,
                }
            }
            _ => return Err(format!("Unknown opcode: 0x{:02X}", opcode)),
        };

        Ok((instr, pos))
    }

    fn deserialize_value(data: &[u8], pos: usize) -> Result<(Value, usize), String> {
        if pos >= data.len() {
            return Err("Unexpected end of bytecode".to_string());
        }

        let tag = data[pos];
        let mut pos = pos + 1;

        let value = match tag {
            0x00 => Value::None,
            0x01 => {
                if pos >= data.len() {
                    return Err("Unexpected end".to_string());
                }
                let b = data[pos] != 0;
                pos += 1;
                Value::Bool(b)
            }
            0x02 => {
                if pos + 8 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let n = i64::from_le_bytes([
                    data[pos],
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                pos += 8;
                Value::Int(n)
            }
            0x03 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let n =
                    i32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;
                Value::I32(n)
            }
            0x04 => {
                if pos + 8 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let f = f64::from_le_bytes([
                    data[pos],
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                pos += 8;
                Value::Float(f)
            }
            0x05 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let f =
                    f32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;
                Value::F32(f)
            }
            0x06 => {
                let (s, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                Value::String(s)
            }
            0x07 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let code =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;
                Value::Char(char::from_u32(code).unwrap_or('\0'))
            }
            0x08 => {
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    let (item, new_pos) = Self::deserialize_value(data, pos)?;
                    items.push(item);
                    pos = new_pos;
                }
                Value::Vec(items)
            }
            0x10 => {
                let (name, new_pos) = Self::deserialize_string(data, pos)?;
                pos = new_pos;
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let param_count =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                let mut params = Vec::with_capacity(param_count);
                for _ in 0..param_count {
                    let (param, new_pos) = Self::deserialize_string(data, pos)?;
                    params.push(param);
                    pos = new_pos;
                }
                if pos + 4 > data.len() {
                    return Err("Unexpected end".to_string());
                }
                let address =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;
                Value::Function {
                    name,
                    params,
                    address,
                }
            }
            _ => return Err(format!("Unknown value tag: 0x{:02X}", tag)),
        };

        Ok((value, pos))
    }

    fn deserialize_string(data: &[u8], pos: usize) -> Result<(String, usize), String> {
        if pos + 4 > data.len() {
            return Err("Unexpected end of bytecode".to_string());
        }
        let len =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let pos = pos + 4;
        if pos + len > data.len() {
            return Err("Unexpected end of bytecode".to_string());
        }
        let s = String::from_utf8(data[pos..pos + len].to_vec())
            .map_err(|_| "Invalid UTF-8 in string".to_string())?;
        Ok((s, pos + len))
    }
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}
