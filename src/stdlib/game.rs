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

/// Draw a single digit (0-9) using 5x7 pixel font
/// Returns the width drawn (6 pixels including spacing)
pub fn draw_digit(x: i64, y: i64, digit: i64, color: u32) {
    // 5x7 bitmap font for digits 0-9
    // Each digit is 5 pixels wide, 7 pixels tall
    let patterns: [[u8; 7]; 10] = [
        // 0
        [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        // 1
        [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        // 2
        [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        // 3
        [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        // 4
        [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        // 5
        [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        // 6
        [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        // 7
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        // 8
        [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        // 9
        [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
    ];

    let d = (digit % 10) as usize;
    let pattern = patterns[d];

    GAME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        for (row, &bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 1 {
                    let px = x + col as i64;
                    let py = y + row as i64;
                    state.draw_rect(px, py, 1, 1, color);
                }
            }
        }
    });
}

/// Draw a number (multiple digits) at position
/// Scale: 1 = 5x7 pixels per digit, 2 = 10x14, etc.
pub fn draw_number(x: i64, y: i64, num: i64, color: u32, scale: i64) {
    let num_str = num.abs().to_string();
    let mut offset = 0i64;

    // Handle negative
    if num < 0 {
        // Draw minus sign
        GAME_STATE.with(|state| {
            state
                .borrow_mut()
                .draw_rect(x, y + 3 * scale, 4 * scale, scale, color);
        });
        offset += 6 * scale;
    }

    for ch in num_str.chars() {
        if let Some(digit) = ch.to_digit(10) {
            draw_digit_scaled(x + offset, y, digit as i64, color, scale);
            offset += 6 * scale;
        }
    }
}

/// Draw a scaled digit
fn draw_digit_scaled(x: i64, y: i64, digit: i64, color: u32, scale: i64) {
    let patterns: [[u8; 7]; 10] = [
        [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
    ];

    let d = (digit % 10) as usize;
    let pattern = patterns[d];

    GAME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        for (row, &bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 1 {
                    let px = x + col as i64 * scale;
                    let py = y + row as i64 * scale;
                    state.draw_rect(px, py, scale, scale, color);
                }
            }
        }
    });
}

/// Draw text "WIN" at position (for victory screen)
pub fn draw_text_win(x: i64, y: i64, color: u32, scale: i64) {
    // W pattern
    let w_pattern: [u8; 7] = [
        0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
    ];
    // I pattern
    let i_pattern: [u8; 7] = [
        0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ];
    // N pattern
    let n_pattern: [u8; 7] = [
        0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
    ];

    draw_char_pattern(x, y, &w_pattern, color, scale);
    draw_char_pattern(x + 6 * scale, y, &i_pattern, color, scale);
    draw_char_pattern(x + 12 * scale, y, &n_pattern, color, scale);
}

/// Draw text "LOSE" at position
pub fn draw_text_lose(x: i64, y: i64, color: u32, scale: i64) {
    // L pattern
    let l_pattern: [u8; 7] = [
        0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
    ];
    // O pattern
    let o_pattern: [u8; 7] = [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ];
    // S pattern
    let s_pattern: [u8; 7] = [
        0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
    ];
    // E pattern
    let e_pattern: [u8; 7] = [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
    ];

    draw_char_pattern(x, y, &l_pattern, color, scale);
    draw_char_pattern(x + 6 * scale, y, &o_pattern, color, scale);
    draw_char_pattern(x + 12 * scale, y, &s_pattern, color, scale);
    draw_char_pattern(x + 18 * scale, y, &e_pattern, color, scale);
}

/// Helper to draw a character pattern
fn draw_char_pattern(x: i64, y: i64, pattern: &[u8; 7], color: u32, scale: i64) {
    GAME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        for (row, &bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 1 {
                    let px = x + col as i64 * scale;
                    let py = y + row as i64 * scale;
                    state.draw_rect(px, py, scale, scale, color);
                }
            }
        }
    });
}
