//! Thread module for Zyra standard library
//!
//! Provides threading primitives:
//! - spawn, join threads
//! - thread sleep, yield
//! - thread-local storage
//! - thread info

use crate::compiler::bytecode::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// Thread handle wrapper
pub struct ThreadHandle {
    pub id: u64,
    pub name: String,
}

// Global thread registry for tracking spawned threads
lazy_static::lazy_static! {
    static ref THREAD_COUNTER: Mutex<u64> = Mutex::new(0);
    static ref THREAD_RESULTS: Mutex<HashMap<u64, Option<Value>>> = Mutex::new(HashMap::new());
}

/// Get current thread ID
pub fn current_thread_id() -> u64 {
    // Convert ThreadId to u64 for Zyra
    let id = thread::current().id();
    // Use debug format to extract numeric part
    let id_str = format!("{:?}", id);
    id_str
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Get current thread name
pub fn current_thread_name() -> Value {
    match thread::current().name() {
        Some(name) => Value::String(name.to_string()),
        None => Value::None,
    }
}

/// Sleep current thread for milliseconds
pub fn thread_sleep_ms(ms: i64) {
    if ms > 0 {
        thread::sleep(Duration::from_millis(ms as u64));
    }
}

/// Sleep current thread for seconds
pub fn thread_sleep_secs(secs: i64) {
    if secs > 0 {
        thread::sleep(Duration::from_secs(secs as u64));
    }
}

/// Yield current thread (give up timeslice)
pub fn thread_yield() {
    thread::yield_now();
}

/// Get number of available CPU cores
pub fn available_parallelism() -> i64 {
    thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

/// Check if current thread is main thread
pub fn is_main_thread() -> bool {
    // Main thread typically has the lowest ID
    current_thread_id() == 1 || current_thread_id() == 0
}

/// Park current thread (wait until unparked)
pub fn thread_park() {
    thread::park();
}

/// Park current thread with timeout (returns true if timed out)
pub fn thread_park_timeout(ms: i64) -> bool {
    if ms > 0 {
        thread::park_timeout(Duration::from_millis(ms as u64));
        // Can't reliably detect timeout vs unpark in std, so return false
        false
    } else {
        false
    }
}

/// Spawn a new thread with a function name to call
/// Returns a thread handle object
pub fn spawn_thread(callback_name: &str) -> Value {
    let mut counter = THREAD_COUNTER.lock().unwrap();
    *counter += 1;
    let thread_id = *counter;

    // Store thread info for later joining
    THREAD_RESULTS.lock().unwrap().insert(thread_id, None);

    let mut map = std::collections::HashMap::new();
    map.insert("_type".to_string(), Value::String("Thread".to_string()));
    map.insert("id".to_string(), Value::Int(thread_id as i64));
    map.insert(
        "callback".to_string(),
        Value::String(callback_name.to_string()),
    );
    map.insert("status".to_string(), Value::String("pending".to_string()));
    Value::Object(map)
}

/// Join a thread and wait for it to complete
/// Returns the result from the thread (or None if not complete)
pub fn join_thread(thread_id: i64) -> Value {
    if let Some(result) = THREAD_RESULTS.lock().unwrap().remove(&(thread_id as u64)) {
        result.unwrap_or(Value::None)
    } else {
        Value::None
    }
}

/// Set the result for a thread (called by VM when thread completes)
pub fn set_thread_result(thread_id: u64, result: Value) {
    THREAD_RESULTS
        .lock()
        .unwrap()
        .insert(thread_id, Some(result));
}

/// Create a thread info struct
pub fn thread_info() -> Value {
    let id = current_thread_id();
    let name = current_thread_name();
    let cores = available_parallelism();

    let mut map = std::collections::HashMap::new();
    map.insert("_type".to_string(), Value::String("ThreadInfo".to_string()));
    map.insert("id".to_string(), Value::Int(id as i64));
    map.insert("name".to_string(), name);
    map.insert("available_cores".to_string(), Value::Int(cores));
    map.insert("is_main".to_string(), Value::Bool(is_main_thread()));
    Value::Object(map)
}

/// Thread builder for configuring threads before spawning
pub struct ThreadBuilder {
    pub name: Option<String>,
    pub stack_size: Option<usize>,
}

impl ThreadBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            stack_size: None,
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }
}

impl Default for ThreadBuilder {
    fn default() -> Self {
        Self::new()
    }
}
