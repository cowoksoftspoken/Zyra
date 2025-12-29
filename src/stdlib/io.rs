//! IO module for Zyra standard library

use crate::compiler::bytecode::Value;
use std::io::{self, Write};

/// Print a value to stdout
pub fn print(value: &Value) {
    print!("{}", value);
    io::stdout().flush().ok();
}

/// Print a value to stdout with newline
pub fn println(value: &Value) {
    println!("{}", value);
}

/// Read a line from stdin
pub fn input() -> Value {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).ok();
    Value::String(buffer.trim().to_string())
}

/// Read a line with a prompt
pub fn input_prompt(prompt: &str) -> Value {
    print!("{}", prompt);
    io::stdout().flush().ok();
    input()
}
