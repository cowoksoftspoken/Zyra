//! Zyra Programming Language
//!
//! A modern, statically typed and deterministic programming language built in Rust.
//! Zyra combines a custom compiler and lightweight virtual machine with
//! compile-time memory safety via ownership, borrowing, and lifetime checking.
//! This design enables fast, predictable, and garbage-collection-free execution.

pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod semantic;
pub mod stdlib;
pub mod vm;

pub use error::{ZyraError, ZyraResult};
