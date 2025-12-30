//! File System module for Zyra standard library
//!
//! Provides file and directory operations:
//! - read_file, write_file, append_file
//! - file_exists, is_file, is_dir
//! - create_dir, remove_file, remove_dir
//! - list_dir, current_dir
//! - path operations

use crate::compiler::bytecode::Value;
use crate::error::{ZyraError, ZyraResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Read entire file contents as string
pub fn read_file(path: &str) -> ZyraResult<Value> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Value::String(contents)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to read file '{}': {}", path, e),
            None,
        )),
    }
}

/// Read file as bytes (returns array of integers)
pub fn read_file_bytes(path: &str) -> ZyraResult<Value> {
    match fs::read(path) {
        Ok(bytes) => {
            let values: Vec<Value> = bytes.iter().map(|b| Value::Int(*b as i64)).collect();
            Ok(Value::Array(values))
        }
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to read file '{}': {}", path, e),
            None,
        )),
    }
}

/// Write string to file (creates or overwrites)
pub fn write_file(path: &str, contents: &str) -> ZyraResult<Value> {
    match fs::write(path, contents) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to write file '{}': {}", path, e),
            None,
        )),
    }
}

/// Write bytes to file
pub fn write_file_bytes(path: &str, bytes: &Value) -> ZyraResult<Value> {
    if let Value::Array(arr) = bytes {
        let byte_vec: Vec<u8> = arr
            .iter()
            .filter_map(|v| {
                if let Value::Int(n) = v {
                    Some(*n as u8)
                } else {
                    None
                }
            })
            .collect();

        match fs::write(path, byte_vec) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(e) => Err(ZyraError::new(
                "FileError",
                &format!("Failed to write file '{}': {}", path, e),
                None,
            )),
        }
    } else {
        Err(ZyraError::new(
            "TypeError",
            "Expected an array of bytes",
            None,
        ))
    }
}

/// Append string to file
pub fn append_file(path: &str, contents: &str) -> ZyraResult<Value> {
    use std::fs::OpenOptions;
    use std::io::Write;

    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => match file.write_all(contents.as_bytes()) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(e) => Err(ZyraError::new(
                "FileError",
                &format!("Failed to append to file '{}': {}", path, e),
                None,
            )),
        },
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to open file '{}': {}", path, e),
            None,
        )),
    }
}

/// Check if a path exists
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Check if path is a file
pub fn is_file(path: &str) -> bool {
    Path::new(path).is_file()
}

/// Check if path is a directory
pub fn is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Create a directory (and parents if needed)
pub fn create_dir(path: &str) -> ZyraResult<Value> {
    match fs::create_dir_all(path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to create directory '{}': {}", path, e),
            None,
        )),
    }
}

/// Remove a file
pub fn remove_file(path: &str) -> ZyraResult<Value> {
    match fs::remove_file(path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to remove file '{}': {}", path, e),
            None,
        )),
    }
}

/// Remove a directory (must be empty)
pub fn remove_dir(path: &str) -> ZyraResult<Value> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to remove directory '{}': {}", path, e),
            None,
        )),
    }
}

/// Remove a directory and all its contents
pub fn remove_dir_all(path: &str) -> ZyraResult<Value> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to remove directory '{}': {}", path, e),
            None,
        )),
    }
}

/// List directory contents
pub fn list_dir(path: &str) -> ZyraResult<Value> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut items = Vec::new();
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    items.push(Value::String(name.to_string()));
                }
            }
            Ok(Value::Array(items))
        }
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to list directory '{}': {}", path, e),
            None,
        )),
    }
}

/// Get current working directory
pub fn current_dir() -> ZyraResult<Value> {
    match std::env::current_dir() {
        Ok(path) => Ok(Value::String(path.to_string_lossy().to_string())),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to get current directory: {}", e),
            None,
        )),
    }
}

/// Change current working directory
pub fn set_current_dir(path: &str) -> ZyraResult<Value> {
    match std::env::set_current_dir(path) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to change directory to '{}': {}", path, e),
            None,
        )),
    }
}

/// Copy a file
pub fn copy_file(from: &str, to: &str) -> ZyraResult<Value> {
    match fs::copy(from, to) {
        Ok(bytes) => Ok(Value::Int(bytes as i64)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to copy '{}' to '{}': {}", from, to, e),
            None,
        )),
    }
}

/// Rename/move a file or directory
pub fn rename(from: &str, to: &str) -> ZyraResult<Value> {
    match fs::rename(from, to) {
        Ok(()) => Ok(Value::Bool(true)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to rename '{}' to '{}': {}", from, to, e),
            None,
        )),
    }
}

/// Get file size in bytes
pub fn file_size(path: &str) -> ZyraResult<Value> {
    match fs::metadata(path) {
        Ok(meta) => Ok(Value::Int(meta.len() as i64)),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to get size of '{}': {}", path, e),
            None,
        )),
    }
}

// Path utilities

/// Join path components
pub fn path_join(base: &str, component: &str) -> String {
    let path = PathBuf::from(base).join(component);
    path.to_string_lossy().to_string()
}

/// Get parent directory
pub fn path_parent(path: &str) -> Value {
    match Path::new(path).parent() {
        Some(parent) => Value::String(parent.to_string_lossy().to_string()),
        None => Value::None,
    }
}

/// Get file name from path
pub fn path_file_name(path: &str) -> Value {
    match Path::new(path).file_name() {
        Some(name) => Value::String(name.to_string_lossy().to_string()),
        None => Value::None,
    }
}

/// Get file extension
pub fn path_extension(path: &str) -> Value {
    match Path::new(path).extension() {
        Some(ext) => Value::String(ext.to_string_lossy().to_string()),
        None => Value::None,
    }
}

/// Get file stem (name without extension)
pub fn path_stem(path: &str) -> Value {
    match Path::new(path).file_stem() {
        Some(stem) => Value::String(stem.to_string_lossy().to_string()),
        None => Value::None,
    }
}

/// Check if path is absolute
pub fn path_is_absolute(path: &str) -> bool {
    Path::new(path).is_absolute()
}

/// Get absolute path
pub fn path_absolute(path: &str) -> ZyraResult<Value> {
    match fs::canonicalize(path) {
        Ok(abs) => Ok(Value::String(abs.to_string_lossy().to_string())),
        Err(e) => Err(ZyraError::new(
            "FileError",
            &format!("Failed to get absolute path for '{}': {}", path, e),
            None,
        )),
    }
}
