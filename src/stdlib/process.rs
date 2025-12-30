//! Process module for Zyra standard library
//!
//! Provides process control:
//! - exit with code
//! - spawn child processes
//! - execute commands

use crate::compiler::bytecode::Value;
use crate::error::{ZyraError, ZyraResult};
use std::collections::HashMap;
use std::process::{Command, Stdio};

/// Exit the program with a status code
pub fn exit(code: i64) -> ! {
    std::process::exit(code as i32)
}

/// Abort the program immediately
pub fn abort() -> ! {
    std::process::abort()
}

/// Get current process ID
pub fn pid() -> i64 {
    std::process::id() as i64
}

/// Execute a command and wait for it to finish
/// Returns object { success: bool, code: int, stdout: string, stderr: string }
pub fn exec(command: &str, args: &[String]) -> ZyraResult<Value> {
    match Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1) as i64;
            let success = output.status.success();

            let mut map = HashMap::new();
            map.insert(
                "_type".to_string(),
                Value::String("ProcessResult".to_string()),
            );
            map.insert("success".to_string(), Value::Bool(success));
            map.insert("code".to_string(), Value::Int(code));
            map.insert("stdout".to_string(), Value::String(stdout));
            map.insert("stderr".to_string(), Value::String(stderr));
            Ok(Value::Object(map))
        }
        Err(e) => Err(ZyraError::new(
            "ProcessError",
            &format!("Failed to execute '{}': {}", command, e),
            None,
        )),
    }
}

/// Execute a shell command (platform-specific)
pub fn shell(command: &str) -> ZyraResult<Value> {
    let (shell_cmd, shell_arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    match Command::new(shell_cmd)
        .arg(shell_arg)
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1) as i64;
            let success = output.status.success();

            let mut map = HashMap::new();
            map.insert(
                "_type".to_string(),
                Value::String("ProcessResult".to_string()),
            );
            map.insert("success".to_string(), Value::Bool(success));
            map.insert("code".to_string(), Value::Int(code));
            map.insert("stdout".to_string(), Value::String(stdout));
            map.insert("stderr".to_string(), Value::String(stderr));
            Ok(Value::Object(map))
        }
        Err(e) => Err(ZyraError::new(
            "ProcessError",
            &format!("Failed to execute shell command: {}", e),
            None,
        )),
    }
}

/// Spawn a child process without waiting (returns process handle)
pub fn spawn(command: &str, args: &[String]) -> ZyraResult<Value> {
    match Command::new(command).args(args).spawn() {
        Ok(child) => {
            let mut map = HashMap::new();
            map.insert("_type".to_string(), Value::String("Process".to_string()));
            map.insert("id".to_string(), Value::Int(child.id() as i64));
            map.insert("command".to_string(), Value::String(command.to_string()));
            Ok(Value::Object(map))
        }
        Err(e) => Err(ZyraError::new(
            "ProcessError",
            &format!("Failed to spawn '{}': {}", command, e),
            None,
        )),
    }
}
