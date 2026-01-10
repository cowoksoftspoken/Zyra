//! String module for Zyra standard library
//!
//! Provides string operations with UTF-8 support:
//! - len, is_empty
//! - slice, char_at
//! - contains, starts_with, ends_with
//! - to_upper, to_lower
//! - trim, split, replace
//! - parse_int, parse_float

use crate::compiler::bytecode::Value;

/// Get the length of a string (character count, not bytes)
pub fn string_len(s: &str) -> i64 {
    s.chars().count() as i64
}

/// Get the byte length of a string
pub fn string_byte_len(s: &str) -> i64 {
    s.len() as i64
}

/// Check if a string is empty
pub fn string_is_empty(s: &str) -> bool {
    s.is_empty()
}

/// Get a character at index (returns None if out of bounds)
pub fn string_char_at(s: &str, index: i64) -> Value {
    if index >= 0 {
        if let Some(c) = s.chars().nth(index as usize) {
            return Value::String(c.to_string());
        }
    }
    Value::None
}

/// Get a substring (by character indices, not bytes)
pub fn string_slice(s: &str, start: i64, end: i64) -> String {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;
    let start = start.max(0) as usize;
    let end = end.min(len) as usize;

    if start <= end && start < chars.len() {
        chars[start..end].iter().collect()
    } else {
        String::new()
    }
}

/// Check if string contains a substring
pub fn string_contains(s: &str, substr: &str) -> bool {
    s.contains(substr)
}

/// Check if string starts with a prefix
pub fn string_starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Check if string ends with a suffix
pub fn string_ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// Convert string to uppercase
pub fn string_to_upper(s: &str) -> String {
    s.to_uppercase()
}

/// Convert string to lowercase
pub fn string_to_lower(s: &str) -> String {
    s.to_lowercase()
}

/// Trim whitespace from both ends
pub fn string_trim(s: &str) -> String {
    s.trim().to_string()
}

/// Trim whitespace from start
pub fn string_trim_start(s: &str) -> String {
    s.trim_start().to_string()
}

/// Trim whitespace from end
pub fn string_trim_end(s: &str) -> String {
    s.trim_end().to_string()
}

/// Split string by delimiter
pub fn string_split(s: &str, delimiter: &str) -> Value {
    let parts: Vec<Value> = s
        .split(delimiter)
        .map(|part| Value::String(part.to_string()))
        .collect();
    Value::Array(parts)
}

/// Split string into lines
pub fn string_lines(s: &str) -> Value {
    let lines: Vec<Value> = s
        .lines()
        .map(|line| Value::String(line.to_string()))
        .collect();
    Value::Array(lines)
}

/// Replace all occurrences
pub fn string_replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

/// Replace first occurrence
pub fn string_replace_first(s: &str, from: &str, to: &str) -> String {
    s.replacen(from, to, 1)
}

/// Repeat a string n times
pub fn string_repeat(s: &str, count: i64) -> String {
    if count <= 0 {
        String::new()
    } else {
        s.repeat(count as usize)
    }
}

/// Reverse a string
pub fn string_reverse(s: &str) -> String {
    s.chars().rev().collect()
}

/// Parse string to integer
pub fn string_parse_int(s: &str) -> Value {
    match s.trim().parse::<i64>() {
        Ok(n) => Value::Int(n),
        Err(_) => Value::None,
    }
}

/// Parse string to float
pub fn string_parse_float(s: &str) -> Value {
    match s.trim().parse::<f64>() {
        Ok(f) => Value::Float(f),
        Err(_) => Value::None,
    }
}

/// Parse string to boolean
pub fn string_parse_bool(s: &str) -> Value {
    match s.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" => Value::Bool(true),
        "false" | "0" | "no" => Value::Bool(false),
        _ => Value::None,
    }
}

/// Find index of substring (-1 if not found)
pub fn string_index_of(s: &str, substr: &str) -> i64 {
    match s.find(substr) {
        Some(pos) => {
            // Convert byte position to char position
            s[..pos].chars().count() as i64
        }
        None => -1,
    }
}

/// Find last index of substring (-1 if not found)
pub fn string_last_index_of(s: &str, substr: &str) -> i64 {
    match s.rfind(substr) {
        Some(pos) => s[..pos].chars().count() as i64,
        None => -1,
    }
}

/// Concatenate two strings
pub fn string_concat(a: &str, b: &str) -> String {
    format!("{}{}", a, b)
}

/// Pad string on the left to reach target length
pub fn string_pad_start(s: &str, target_len: i64, pad_char: char) -> String {
    let current_len = s.chars().count() as i64;
    if current_len >= target_len {
        s.to_string()
    } else {
        let padding: String = std::iter::repeat(pad_char)
            .take((target_len - current_len) as usize)
            .collect();
        format!("{}{}", padding, s)
    }
}

/// Pad string on the right to reach target length
pub fn string_pad_end(s: &str, target_len: i64, pad_char: char) -> String {
    let current_len = s.chars().count() as i64;
    if current_len >= target_len {
        s.to_string()
    } else {
        let padding: String = std::iter::repeat(pad_char)
            .take((target_len - current_len) as usize)
            .collect();
        format!("{}{}", s, padding)
    }
}

/// Check if string is valid UTF-8 (always true for Rust strings, but useful for validation after external input)
pub fn string_is_valid_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

/// Convert string to character array
pub fn string_to_chars(s: &str) -> Value {
    let chars: Vec<Value> = s.chars().map(|c| Value::String(c.to_string())).collect();
    Value::Array(chars)
}

/// Convert string to byte array
pub fn string_to_bytes(s: &str) -> Value {
    let bytes: Vec<Value> = s.bytes().map(|b| Value::Int(b as i64)).collect();
    Value::Array(bytes)
}

/// Create string from character array
pub fn string_from_chars(arr: &Value) -> String {
    if let Value::Array(vec) = arr {
        vec.iter()
            .filter_map(|v| {
                if let Value::String(s) = v {
                    s.chars().next()
                } else {
                    None
                }
            })
            .collect()
    } else {
        String::new()
    }
}

// ===== String Validation Functions =====

/// Check if string contains only numeric characters (can be parsed as number)
pub fn string_is_numeric(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Allow optional leading sign and one decimal point
    let mut has_decimal = false;
    let mut chars = trimmed.chars().peekable();

    // Skip leading sign
    if matches!(chars.peek(), Some('+') | Some('-')) {
        chars.next();
    }

    if chars.peek().is_none() {
        return false; // Just a sign
    }

    for c in chars {
        if c == '.' {
            if has_decimal {
                return false; // Multiple decimal points
            }
            has_decimal = true;
        } else if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

/// Check if string contains only digit characters (0-9)
pub fn string_is_digit(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Check if string contains only alphabetic characters
pub fn string_is_alpha(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphabetic())
}

/// Check if string contains only alphanumeric characters
pub fn string_is_alphanumeric(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric())
}

// ===== Type Casting Functions =====

/// Convert string to i32
pub fn string_to_i32(s: &str) -> Value {
    match s.trim().parse::<i32>() {
        Ok(n) => Value::Int(n as i64),
        Err(_) => Value::None,
    }
}

/// Convert string to i64
pub fn string_to_i64(s: &str) -> Value {
    match s.trim().parse::<i64>() {
        Ok(n) => Value::Int(n),
        Err(_) => Value::None,
    }
}

/// Convert string to f32
pub fn string_to_f32(s: &str) -> Value {
    match s.trim().parse::<f32>() {
        Ok(f) => Value::Float(f as f64),
        Err(_) => Value::None,
    }
}

/// Convert string to f64
pub fn string_to_f64(s: &str) -> Value {
    match s.trim().parse::<f64>() {
        Ok(f) => Value::Float(f),
        Err(_) => Value::None,
    }
}
