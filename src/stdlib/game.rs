//! Game module for Zyra standard library
//!
//! Provides window creation, drawing, and input handling for 2D games

use crate::compiler::bytecode::{Value, WindowState};
use minifb::{Key, Window, WindowOptions};
use std::collections::HashMap;

/// Global game state
pub struct GameState {
    pub window: Option<Window>,
    pub buffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub keys_pressed: HashMap<String, bool>,
    pub running: bool,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            window: None,
            buffer: Vec::new(),
            width: 0,
            height: 0,
            keys_pressed: HashMap::new(),
            running: false,
        }
    }

    /// Create a new window
    pub fn create_window(&mut self, width: usize, height: usize, title: &str) -> bool {
        let options = WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        };

        match Window::new(title, width, height, options) {
            Ok(win) => {
                self.window = Some(win);
                self.buffer = vec![0; width * height];
                self.width = width;
                self.height = height;
                self.running = true;
                true
            }
            Err(_) => false,
        }
    }

    /// Check if window is still open
    pub fn is_open(&mut self) -> bool {
        if let Some(ref window) = self.window {
            window.is_open() && !window.is_key_down(Key::Escape)
        } else {
            false
        }
    }

    /// Update key states
    pub fn update_keys(&mut self) {
        if let Some(ref window) = self.window {
            // Clear previous state
            self.keys_pressed.clear();

            // Check common game keys
            let key_mappings = [
                ("W", Key::W),
                ("w", Key::W),
                ("A", Key::A),
                ("a", Key::A),
                ("S", Key::S),
                ("s", Key::S),
                ("D", Key::D),
                ("d", Key::D),
                ("Up", Key::Up),
                ("Down", Key::Down),
                ("Left", Key::Left),
                ("Right", Key::Right),
                ("Space", Key::Space),
                ("Enter", Key::Enter),
                ("Escape", Key::Escape),
            ];

            for (name, key) in key_mappings.iter() {
                if window.is_key_down(*key) {
                    self.keys_pressed.insert(name.to_string(), true);
                }
            }
        }
    }

    /// Check if a key is pressed
    pub fn is_key_pressed(&self, key: &str) -> bool {
        self.keys_pressed.get(key).copied().unwrap_or(false)
    }

    /// Clear the screen to black
    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }

    /// Clear to a specific color
    pub fn clear_color(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    /// Draw a filled rectangle
    pub fn draw_rect(&mut self, x: i64, y: i64, w: i64, h: i64, color: u32) {
        let x = x.max(0) as usize;
        let y = y.max(0) as usize;
        let w = w.max(0) as usize;
        let h = h.max(0) as usize;

        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px < self.width && py < self.height {
                    self.buffer[py * self.width + px] = color;
                }
            }
        }
    }

    /// Display the buffer to the window
    pub fn display(&mut self) {
        if let Some(ref mut window) = self.window {
            window
                .update_with_buffer(&self.buffer, self.width, self.height)
                .ok();
            // Update key states after display
            self.update_keys();
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-local game state (Window is not Send/Sync so we use thread_local instead of lazy_static)
thread_local! {
    pub static GAME_STATE: std::cell::RefCell<GameState> = std::cell::RefCell::new(GameState::new());
}

/// Create a window and return a Window value
pub fn create_window(width: i64, height: i64, title: &str) -> Value {
    let title_owned = title.to_string();
    let w = width as usize;
    let h = height as usize;
    GAME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.create_window(w, h, &title_owned) {
            Value::Window(WindowState {
                width: w,
                height: h,
                title: title_owned.clone(),
                buffer: Vec::new(),
                is_open: true,
            })
        } else {
            Value::None
        }
    })
}

/// Check if window is open
pub fn window_is_open() -> bool {
    GAME_STATE.with(|state| state.borrow_mut().is_open())
}

/// Check if a key is pressed
pub fn key_pressed(key: &str) -> bool {
    GAME_STATE.with(|state| state.borrow().is_key_pressed(key))
}

/// Clear the screen
pub fn clear() {
    GAME_STATE.with(|state| {
        state.borrow_mut().clear();
    })
}

/// Draw a rectangle (default white color)
pub fn draw_rect(x: i64, y: i64, w: i64, h: i64) {
    GAME_STATE.with(|state| {
        state.borrow_mut().draw_rect(x, y, w, h, 0xFFFFFF); // White
    })
}

/// Draw a rectangle with specific color
pub fn draw_rect_color(x: i64, y: i64, w: i64, h: i64, color: u32) {
    GAME_STATE.with(|state| {
        state.borrow_mut().draw_rect(x, y, w, h, color);
    })
}

/// Display the frame
pub fn display() {
    GAME_STATE.with(|state| {
        state.borrow_mut().display();
    })
}
