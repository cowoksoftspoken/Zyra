//! Zyra Standard Library
//!
//! Built-in functions exposed to Zyra programs

pub mod game;
pub mod io;
pub mod math;
pub mod sync;
pub mod time;

use crate::compiler::bytecode::Value;
use crate::error::ZyraResult;
// VM is no longer needed here - stdlib functions use global state

/// Standard library dispatcher
pub struct StdLib {
    // Reserved for future state
}

impl StdLib {
    pub fn new() -> Self {
        Self {}
    }

    /// Call a standard library function
    pub fn call(&self, name: &str, args: &[Value]) -> ZyraResult<Option<Value>> {
        match name {
            // IO functions
            "print" => {
                if let Some(value) = args.first() {
                    io::print(value);
                }
                Ok(Some(Value::None))
            }
            "println" => {
                if let Some(value) = args.first() {
                    io::println(value);
                } else {
                    println!();
                }
                Ok(Some(Value::None))
            }
            "input" => Ok(Some(io::input())),

            // Math functions
            "abs" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::abs(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "min" => {
                if args.len() >= 2 {
                    Ok(Some(math::min(&args[0], &args[1])))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "max" => {
                if args.len() >= 2 {
                    Ok(Some(math::max(&args[0], &args[1])))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "sqrt" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::sqrt(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "pow" => {
                if args.len() >= 2 {
                    Ok(Some(math::pow(&args[0], &args[1])))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "floor" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::floor(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "ceil" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::ceil(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "round" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::round(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "random" => {
                let min = args
                    .get(0)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                let max = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(100);
                Ok(Some(math::random(min, max)))
            }
            "sin" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::sin(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "cos" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::cos(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "pi" => Ok(Some(math::pi())),
            "clamp" => {
                if args.len() >= 3 {
                    Ok(Some(math::clamp(&args[0], &args[1], &args[2])))
                } else {
                    Ok(Some(Value::None))
                }
            }

            // Time functions
            "now" => Ok(Some(time::now())),
            "sleep" => {
                if let Some(Value::Int(ms)) = args.first() {
                    time::sleep(*ms);
                }
                Ok(Some(Value::None))
            }

            // Game functions - Window constructor
            "Window" => {
                let width = args
                    .get(0)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(800);
                let height = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(600);
                let title = args
                    .get(2)
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "Zyra Window".to_string());
                Ok(Some(game::create_window(width, height, &title)))
            }

            // Window methods (called on window objects)
            "win.is_open" | "is_open" => Ok(Some(Value::Bool(game::window_is_open()))),
            "win.clear" | "clear" => {
                game::clear();
                Ok(Some(Value::None))
            }
            "win.display" | "display" => {
                game::display();
                Ok(Some(Value::None))
            }

            // Input
            "input.key" | "key_pressed" => {
                if let Some(Value::String(key)) = args.first() {
                    Ok(Some(Value::Bool(game::key_pressed(key))))
                } else {
                    Ok(Some(Value::Bool(false)))
                }
            }

            // Drawing
            "draw.rect" | "draw_rect" => {
                let x = args
                    .get(0)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                let y = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                let w = args
                    .get(2)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(10);
                let h = args
                    .get(3)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(10);
                game::draw_rect(x, y, w, h);
                Ok(Some(Value::None))
            }
            "draw.rect_color" => {
                let x = args
                    .get(0)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                let y = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                let w = args
                    .get(2)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(10);
                let h = args
                    .get(3)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(10);
                let color = args
                    .get(4)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n as u32),
                        _ => None,
                    })
                    .unwrap_or(0xFFFFFF);
                game::draw_rect_color(x, y, w, h, color);
                Ok(Some(Value::None))
            }

            // String/List methods
            "len" | "length" => {
                if let Some(value) = args.first() {
                    match value {
                        Value::String(s) => Ok(Some(Value::Int(s.len() as i64))),
                        Value::List(items) => Ok(Some(Value::Int(items.len() as i64))),
                        _ => Ok(Some(Value::Int(0))),
                    }
                } else {
                    Ok(Some(Value::Int(0)))
                }
            }

            // Handle method calls: check if name ends with known methods
            _ if name.ends_with(".len") || name.ends_with(".length") => {
                if let Some(value) = args.first() {
                    match value {
                        Value::String(s) => Ok(Some(Value::Int(s.len() as i64))),
                        Value::List(items) => Ok(Some(Value::Int(items.len() as i64))),
                        _ => Ok(Some(Value::Int(0))),
                    }
                } else {
                    Ok(Some(Value::Int(0)))
                }
            }

            // Unknown function
            _ => Ok(None),
        }
    }
}

impl Default for StdLib {
    fn default() -> Self {
        Self::new()
    }
}
