//! Thread-safe synchronization primitives for Zyra
//!
//! Provides:
//! - Atomic<T>: Lock-free atomic operations
//! - Arc<T>: Thread-safe reference counting
//! - Mutex<T>: Mutual exclusion locks
//! - RwLock<T>: Read-write locks
//! - Channel<T>: Message passing channels

use crate::compiler::bytecode::Value;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering},
    Arc, Mutex, RwLock,
};

/// Thread-safe atomic value wrapper
#[derive(Debug)]
pub enum AtomicValue {
    Bool(AtomicBool),
    I32(AtomicI32),
    I64(AtomicI64),
    U32(AtomicU32),
    U64(AtomicU64),
}

impl AtomicValue {
    /// Create a new atomic from a value
    pub fn new(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(b) => Some(AtomicValue::Bool(AtomicBool::new(*b))),
            Value::Int(i) => Some(AtomicValue::I64(AtomicI64::new(*i))),
            _ => None,
        }
    }

    /// Load the current value
    pub fn load(&self) -> Value {
        match self {
            AtomicValue::Bool(a) => Value::Bool(a.load(Ordering::SeqCst)),
            AtomicValue::I32(a) => Value::Int(a.load(Ordering::SeqCst) as i64),
            AtomicValue::I64(a) => Value::Int(a.load(Ordering::SeqCst)),
            AtomicValue::U32(a) => Value::Int(a.load(Ordering::SeqCst) as i64),
            AtomicValue::U64(a) => Value::Int(a.load(Ordering::SeqCst) as i64),
        }
    }

    /// Store a new value
    pub fn store(&self, value: &Value) {
        match (self, value) {
            (AtomicValue::Bool(a), Value::Bool(b)) => a.store(*b, Ordering::SeqCst),
            (AtomicValue::I64(a), Value::Int(i)) => a.store(*i, Ordering::SeqCst),
            _ => {}
        }
    }

    /// Compare and swap atomically
    pub fn compare_and_swap(&self, expected: &Value, new: &Value) -> bool {
        match (self, expected, new) {
            (AtomicValue::Bool(a), Value::Bool(exp), Value::Bool(n)) => a
                .compare_exchange(*exp, *n, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok(),
            (AtomicValue::I64(a), Value::Int(exp), Value::Int(n)) => a
                .compare_exchange(*exp, *n, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok(),
            _ => false,
        }
    }

    /// Atomically add to the value (returns old value)
    pub fn fetch_add(&self, delta: i64) -> Value {
        match self {
            AtomicValue::I32(a) => Value::Int(a.fetch_add(delta as i32, Ordering::SeqCst) as i64),
            AtomicValue::I64(a) => Value::Int(a.fetch_add(delta, Ordering::SeqCst)),
            AtomicValue::U32(a) => Value::Int(a.fetch_add(delta as u32, Ordering::SeqCst) as i64),
            AtomicValue::U64(a) => Value::Int(a.fetch_add(delta as u64, Ordering::SeqCst) as i64),
            AtomicValue::Bool(_) => self.load(),
        }
    }
}

/// Thread-safe shared value with Arc<Mutex<T>>
pub struct SharedValue {
    inner: Arc<Mutex<Value>>,
}

impl SharedValue {
    pub fn new(value: Value) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    pub fn get(&self) -> Value {
        self.inner.lock().unwrap().clone()
    }

    pub fn set(&self, value: Value) {
        *self.inner.lock().unwrap() = value;
    }

    pub fn clone_arc(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Clone for SharedValue {
    fn clone(&self) -> Self {
        self.clone_arc()
    }
}

/// Read-write locked value for concurrent reads
pub struct RwValue {
    inner: Arc<RwLock<Value>>,
}

impl RwValue {
    pub fn new(value: Value) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    /// Fast read access (multiple readers allowed)
    pub fn read(&self) -> Value {
        self.inner.read().unwrap().clone()
    }

    /// Exclusive write access
    pub fn write(&self, value: Value) {
        *self.inner.write().unwrap() = value;
    }

    pub fn clone_arc(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Clone for RwValue {
    fn clone(&self) -> Self {
        self.clone_arc()
    }
}

/// Message passing channel
pub struct Channel<T> {
    sender: std::sync::mpsc::Sender<T>,
    receiver: Arc<Mutex<std::sync::mpsc::Receiver<T>>>,
}

impl Channel<Value> {
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn send(&self, value: Value) -> bool {
        self.sender.send(value).is_ok()
    }

    pub fn recv(&self) -> Option<Value> {
        self.receiver.lock().unwrap().recv().ok()
    }

    pub fn try_recv(&self) -> Option<Value> {
        self.receiver.lock().unwrap().try_recv().ok()
    }
}

/// Register sync module functions
pub fn register_sync_functions() -> HashMap<String, fn(Vec<Value>) -> Value> {
    let mut funcs: HashMap<String, fn(Vec<Value>) -> Value> = HashMap::new();

    // Atomic operations
    funcs.insert("atomic_new".to_string(), |args| {
        if args.is_empty() {
            Value::None
        } else {
            args[0].clone()
        }
    });

    // Arc operations
    funcs.insert("arc_new".to_string(), |args| {
        if args.is_empty() {
            Value::None
        } else {
            args[0].clone()
        }
    });

    // Mutex operations
    funcs.insert("mutex_new".to_string(), |args| {
        if args.is_empty() {
            Value::None
        } else {
            args[0].clone()
        }
    });

    funcs
}
