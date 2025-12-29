//! Zyra Programming Language
//!
//! A modern, safe, and deterministic programming language built in Rust.
//! Designed for students, beginner programmers, and indie game developers.

pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod semantic;
pub mod stdlib;
pub mod vm;

pub use error::{ZyraError, ZyraResult};
