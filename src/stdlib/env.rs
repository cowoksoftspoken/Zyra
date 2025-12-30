//! Environment module for Zyra standard library
//!
//! Provides access to:
//! - Command line arguments
//! - Environment variables
//! - System information

use crate::compiler::bytecode::Value;

/// Get command line arguments
pub fn args() -> Value {
    let args: Vec<Value> = std::env::args().map(|s| Value::String(s)).collect();
    Value::Array(args)
}

/// Get number of command line arguments
pub fn args_count() -> i64 {
    std::env::args().count() as i64
}

/// Get a specific command line argument (index 0 is program name)
pub fn arg(index: i64) -> Value {
    std::env::args()
        .nth(index as usize)
        .map(|s| Value::String(s))
        .unwrap_or(Value::None)
}

/// Get an environment variable
pub fn env_var(name: &str) -> Value {
    match std::env::var(name) {
        Ok(value) => Value::String(value),
        Err(_) => Value::None,
    }
}

/// Set an environment variable (for current process)
pub fn set_env_var(name: &str, value: &str) {
    std::env::set_var(name, value);
}

/// Remove an environment variable
pub fn remove_env_var(name: &str) {
    std::env::remove_var(name);
}

/// Get all environment variables as array of [name, value] pairs
pub fn env_vars() -> Value {
    let vars: Vec<Value> = std::env::vars()
        .map(|(k, v)| Value::Array(vec![Value::String(k), Value::String(v)]))
        .collect();
    Value::Array(vars)
}

/// Check if an environment variable exists
pub fn env_var_exists(name: &str) -> bool {
    std::env::var(name).is_ok()
}

/// Get the home directory
pub fn home_dir() -> Value {
    #[allow(deprecated)]
    match std::env::home_dir() {
        Some(path) => Value::String(path.to_string_lossy().to_string()),
        None => Value::None,
    }
}

/// Get the temp directory
pub fn temp_dir() -> Value {
    let path = std::env::temp_dir();
    Value::String(path.to_string_lossy().to_string())
}

/// Get the executable path
pub fn exe_path() -> Value {
    match std::env::current_exe() {
        Ok(path) => Value::String(path.to_string_lossy().to_string()),
        Err(_) => Value::None,
    }
}

/// Get the OS name
pub fn os_name() -> Value {
    Value::String(std::env::consts::OS.to_string())
}

/// Get the OS architecture
pub fn os_arch() -> Value {
    Value::String(std::env::consts::ARCH.to_string())
}

/// Get the OS family (unix/windows)
pub fn os_family() -> Value {
    Value::String(std::env::consts::FAMILY.to_string())
}

/// Check if running on Windows
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Check if running on Linux
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Check if running on macOS
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}
