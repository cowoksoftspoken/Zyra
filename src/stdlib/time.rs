//! Time module for Zyra standard library
//!
//! Provides time utilities:
//! - Instant type for monotonic timing
//! - Duration helpers
//! - sleep, now
//! - Frame timing for games

use crate::compiler::bytecode::Value;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Global instant storage for monotonic timing
lazy_static::lazy_static! {
    static ref INSTANTS: Mutex<Vec<Instant>> = Mutex::new(Vec::new());
    static ref START_TIME: Instant = Instant::now();
    static ref LAST_FRAME_TIME: Mutex<Instant> = Mutex::new(Instant::now());
}

/// Get current time in milliseconds since epoch
pub fn now() -> Value {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    Value::Int(duration.as_millis() as i64)
}

/// Get current time in seconds since epoch (float)
pub fn now_secs() -> Value {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    Value::Float(duration.as_secs_f64())
}

/// Get monotonic time in milliseconds since program start
pub fn monotonic_ms() -> i64 {
    START_TIME.elapsed().as_millis() as i64
}

/// Get monotonic time in seconds since program start
pub fn monotonic_secs() -> f64 {
    START_TIME.elapsed().as_secs_f64()
}

/// Create a new Instant (returns handle ID)
pub fn instant_now() -> Value {
    let mut instants = INSTANTS.lock().unwrap();
    let id = instants.len();
    instants.push(Instant::now());
    Value::Int(id as i64)
}

/// Get elapsed time since an Instant in milliseconds
pub fn instant_elapsed_ms(instant_id: i64) -> Value {
    let instants = INSTANTS.lock().unwrap();
    if let Some(instant) = instants.get(instant_id as usize) {
        Value::Int(instant.elapsed().as_millis() as i64)
    } else {
        Value::None
    }
}

/// Get elapsed time since an Instant in seconds (float)
pub fn instant_elapsed_secs(instant_id: i64) -> Value {
    let instants = INSTANTS.lock().unwrap();
    if let Some(instant) = instants.get(instant_id as usize) {
        Value::Float(instant.elapsed().as_secs_f64())
    } else {
        Value::None
    }
}

/// Sleep for a number of milliseconds
pub fn sleep(ms: i64) {
    if ms > 0 {
        thread::sleep(Duration::from_millis(ms as u64));
    }
}

/// Sleep for a number of seconds
pub fn sleep_secs(secs: f64) {
    if secs > 0.0 {
        thread::sleep(Duration::from_secs_f64(secs));
    }
}

/// Get delta time since last frame (in seconds)
pub fn delta_time() -> f64 {
    let mut last = LAST_FRAME_TIME.lock().unwrap();
    let now = Instant::now();
    let delta = now.duration_since(*last).as_secs_f64();
    *last = now;
    delta
}

/// Mark frame start (for delta time tracking)
pub fn frame_start() {
    let mut last = LAST_FRAME_TIME.lock().unwrap();
    *last = Instant::now();
}

/// Get frames per second (based on last frame time)
pub fn fps() -> f64 {
    let last = LAST_FRAME_TIME.lock().unwrap();
    let elapsed = last.elapsed().as_secs_f64();
    if elapsed > 0.0 {
        1.0 / elapsed
    } else {
        60.0 // default
    }
}

// Duration utilities

/// Create a Duration from milliseconds
pub fn duration_from_ms(ms: i64) -> Value {
    let mut map = std::collections::HashMap::new();
    map.insert("_type".to_string(), Value::String("Duration".to_string()));
    map.insert("ms".to_string(), Value::Int(ms));
    map.insert("secs".to_string(), Value::Float(ms as f64 / 1000.0));
    Value::Object(map)
}

/// Create a Duration from seconds
pub fn duration_from_secs(secs: f64) -> Value {
    let mut map = std::collections::HashMap::new();
    map.insert("_type".to_string(), Value::String("Duration".to_string()));
    map.insert("ms".to_string(), Value::Int((secs * 1000.0) as i64));
    map.insert("secs".to_string(), Value::Float(secs));
    Value::Object(map)
}

/// Frame timer for games
pub struct FrameTimer {
    last_frame: Instant,
    target_fps: u32,
    frame_duration: Duration,
}

impl FrameTimer {
    pub fn new(target_fps: u32) -> Self {
        Self {
            last_frame: Instant::now(),
            target_fps,
            frame_duration: Duration::from_secs_f64(1.0 / target_fps as f64),
        }
    }

    /// Get delta time since last frame in seconds
    pub fn delta(&mut self) -> f64 {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        delta.as_secs_f64()
    }

    /// Wait to maintain target FPS
    pub fn wait(&mut self) {
        let elapsed = self.last_frame.elapsed();
        if elapsed < self.frame_duration {
            thread::sleep(self.frame_duration - elapsed);
        }
    }

    /// Get current FPS
    pub fn get_fps(&self) -> f64 {
        let elapsed = self.last_frame.elapsed();
        if elapsed.as_secs_f64() > 0.0 {
            1.0 / elapsed.as_secs_f64()
        } else {
            self.target_fps as f64
        }
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new(60)
    }
}

/// Measure execution time of a closure (for benchmarking)
pub fn measure<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    (result, start.elapsed())
}

/// Format duration as human-readable string
pub fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else if ms < 3600000 {
        let mins = ms / 60000;
        let secs = (ms % 60000) / 1000;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = ms / 3600000;
        let mins = (ms % 3600000) / 60000;
        format!("{}h {}m", hours, mins)
    }
}
