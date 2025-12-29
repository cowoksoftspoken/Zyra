//! Time module for Zyra standard library

use crate::compiler::bytecode::Value;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Get current time in milliseconds since epoch
pub fn now() -> Value {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    Value::Int(duration.as_millis() as i64)
}

/// Sleep for a number of milliseconds
pub fn sleep(ms: i64) {
    if ms > 0 {
        thread::sleep(Duration::from_millis(ms as u64));
    }
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
    pub fn fps(&self) -> f64 {
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
