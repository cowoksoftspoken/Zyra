//! Zyra Standard Library
//!
//! Built-in functions exposed to Zyra programs

pub mod core;
pub mod env;
pub mod fs;
pub mod game;
pub mod io;
pub mod math;
pub mod mem;
pub mod process;
pub mod string;
pub mod sync;
pub mod thread;
pub mod time;
pub mod vec;

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

    /// Extract i64 from any integer Value type (I8, I32, I64, Int, U8, U32, U64)
    fn to_i64(v: &Value) -> Option<i64> {
        match v {
            Value::Int(n) | Value::I64(n) => Some(*n),
            Value::I32(n) => Some(*n as i64),
            Value::I8(n) => Some(*n as i64),
            Value::U8(n) => Some(*n as i64),
            Value::U32(n) => Some(*n as i64),
            Value::U64(n) => Some(*n as i64),
            _ => None,
        }
    }

    /// Call a standard library function
    pub fn call(&self, name: &str, args: &[Value]) -> ZyraResult<Option<Value>> {
        // Handle qualified names by using the leaf name (e.g. std::math::abs -> abs)
        // This relies on the semantic analyzer to ensure correct module usage
        let func_name = name.split("::").last().unwrap_or(name);

        match func_name {
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
                let ms = match args.first() {
                    Some(Value::Int(n)) => *n,
                    Some(Value::I64(n)) => *n,
                    Some(Value::I32(n)) => *n as i64,
                    _ => 0,
                };
                time::sleep(ms);
                Ok(Some(Value::None))
            }

            // Game functions - Window constructor
            "Window" => {
                let width = args.get(0).and_then(Self::to_i64).unwrap_or(800);
                let height = args.get(1).and_then(Self::to_i64).unwrap_or(600);
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
                let x = args.get(0).and_then(Self::to_i64).unwrap_or(0);
                let y = args.get(1).and_then(Self::to_i64).unwrap_or(0);
                let w = args.get(2).and_then(Self::to_i64).unwrap_or(10);
                let h = args.get(3).and_then(Self::to_i64).unwrap_or(10);
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

            // Draw a number at position with scale
            "draw_number" | "draw.number" => {
                let x = args
                    .first()
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let y = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let num = args
                    .get(2)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let scale = args
                    .get(3)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(2);
                game::draw_number(x, y, num, 0xFFFFFF, scale);
                Ok(Some(Value::None))
            }

            // Draw WIN text
            "draw_win" | "draw.win" => {
                let x = args
                    .first()
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let y = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let scale = args
                    .get(2)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(4);
                game::draw_text_win(x, y, 0x00FF00, scale); // Green
                Ok(Some(Value::None))
            }

            // Draw LOSE text
            "draw_lose" | "draw.lose" => {
                let x = args
                    .first()
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let y = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let scale = args
                    .get(2)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        Value::I32(n) => Some(*n as i64),
                        _ => None,
                    })
                    .unwrap_or(4);
                game::draw_text_lose(x, y, 0xFF0000, scale); // Red
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

            // ===== NEW STDLIB FUNCTIONS =====

            // Core functions
            "assert" => {
                let condition = args
                    .get(0)
                    .map(|v| matches!(v, Value::Bool(true)))
                    .unwrap_or(false);
                let message = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("Assertion failed");
                core::assert_true(condition, message)?;
                Ok(Some(Value::None))
            }
            "panic" => {
                let message = args
                    .get(0)
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("Panic");
                core::panic(message)?;
                Ok(Some(Value::None))
            }
            "type_of" => {
                if let Some(value) = args.first() {
                    Ok(Some(Value::String(core::type_of(value))))
                } else {
                    Ok(Some(Value::String("None".to_string())))
                }
            }
            "is_none" => {
                let result = args.first().map(|v| core::is_none(v)).unwrap_or(true);
                Ok(Some(Value::Bool(result)))
            }
            "is_some" => {
                let result = args.first().map(|v| core::is_some(v)).unwrap_or(false);
                Ok(Some(Value::Bool(result)))
            }
            "unwrap" => {
                if let Some(value) = args.first() {
                    core::unwrap(value.clone()).map(Some)
                } else {
                    Ok(Some(Value::None))
                }
            }

            // String functions
            "string_len" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Some(Value::Int(string::string_len(s))))
                } else {
                    Ok(Some(Value::Int(0)))
                }
            }
            "to_upper" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Some(Value::String(string::string_to_upper(s))))
                } else {
                    Ok(Some(Value::String(String::new())))
                }
            }
            "to_lower" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Some(Value::String(string::string_to_lower(s))))
                } else {
                    Ok(Some(Value::String(String::new())))
                }
            }
            "trim" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Some(Value::String(string::string_trim(s))))
                } else {
                    Ok(Some(Value::String(String::new())))
                }
            }
            "contains" => {
                if let (Some(Value::String(s)), Some(Value::String(substr))) =
                    (args.get(0), args.get(1))
                {
                    Ok(Some(Value::Bool(string::string_contains(s, substr))))
                } else {
                    Ok(Some(Value::Bool(false)))
                }
            }
            "split" => {
                if let (Some(Value::String(s)), Some(Value::String(delim))) =
                    (args.get(0), args.get(1))
                {
                    Ok(Some(string::string_split(s, delim)))
                } else {
                    Ok(Some(Value::Array(Vec::new())))
                }
            }
            "replace" => {
                if let (
                    Some(Value::String(s)),
                    Some(Value::String(from)),
                    Some(Value::String(to)),
                ) = (args.get(0), args.get(1), args.get(2))
                {
                    Ok(Some(Value::String(string::string_replace(s, from, to))))
                } else {
                    Ok(Some(Value::String(String::new())))
                }
            }
            "parse_int" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Some(string::string_parse_int(s)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "parse_float" => {
                if let Some(Value::String(s)) = args.first() {
                    Ok(Some(string::string_parse_float(s)))
                } else {
                    Ok(Some(Value::None))
                }
            }

            // Math - New functions
            "tan" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::tan(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "atan2" => {
                if args.len() >= 2 {
                    Ok(Some(math::atan2(&args[0], &args[1])))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "lerp" => {
                if args.len() >= 3 {
                    Ok(Some(math::lerp_value(&args[0], &args[1], &args[2])))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "sign" => {
                if let Some(value) = args.first() {
                    Ok(Some(math::sign(value)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "e" => Ok(Some(math::e())),
            "tau" => Ok(Some(math::tau())),
            "random_float" => Ok(Some(math::random_float())),

            // Time - New functions
            "now_secs" => Ok(Some(time::now_secs())),
            "monotonic_ms" => Ok(Some(Value::Int(time::monotonic_ms()))),
            "instant_now" => Ok(Some(time::instant_now())),
            "instant_elapsed" => {
                if let Some(Value::Int(id)) = args.first() {
                    Ok(Some(time::instant_elapsed_ms(*id)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "delta_time" => Ok(Some(Value::Float(time::delta_time()))),
            "fps" => Ok(Some(Value::Float(time::fps()))),

            // File system functions
            "read_file" => {
                if let Some(Value::String(path)) = args.first() {
                    fs::read_file(path).map(Some)
                } else {
                    Ok(Some(Value::None))
                }
            }
            "write_file" => {
                if let (Some(Value::String(path)), Some(Value::String(content))) =
                    (args.get(0), args.get(1))
                {
                    fs::write_file(path, content).map(Some)
                } else {
                    Ok(Some(Value::Bool(false)))
                }
            }
            "file_exists" => {
                if let Some(Value::String(path)) = args.first() {
                    Ok(Some(Value::Bool(fs::file_exists(path))))
                } else {
                    Ok(Some(Value::Bool(false)))
                }
            }
            "is_file" => {
                if let Some(Value::String(path)) = args.first() {
                    Ok(Some(Value::Bool(fs::is_file(path))))
                } else {
                    Ok(Some(Value::Bool(false)))
                }
            }
            "is_dir" => {
                if let Some(Value::String(path)) = args.first() {
                    Ok(Some(Value::Bool(fs::is_dir(path))))
                } else {
                    Ok(Some(Value::Bool(false)))
                }
            }
            "list_dir" => {
                if let Some(Value::String(path)) = args.first() {
                    fs::list_dir(path).map(Some)
                } else {
                    Ok(Some(Value::Array(Vec::new())))
                }
            }
            "current_dir" => fs::current_dir().map(Some),

            // Environment functions
            "args" => Ok(Some(env::args())),
            "args_count" => Ok(Some(Value::Int(env::args_count()))),
            "env_var" => {
                if let Some(Value::String(name)) = args.first() {
                    Ok(Some(env::env_var(name)))
                } else {
                    Ok(Some(Value::None))
                }
            }
            "os_name" => Ok(Some(env::os_name())),
            "os_arch" => Ok(Some(env::os_arch())),
            "is_windows" => Ok(Some(Value::Bool(env::is_windows()))),
            "is_linux" => Ok(Some(Value::Bool(env::is_linux()))),
            "temp_dir" => Ok(Some(env::temp_dir())),

            // Thread functions
            "thread_sleep" => {
                if let Some(Value::Int(ms)) = args.first() {
                    thread::thread_sleep_ms(*ms);
                }
                Ok(Some(Value::None))
            }
            "thread_yield" => {
                thread::thread_yield();
                Ok(Some(Value::None))
            }
            "thread_id" => Ok(Some(Value::Int(thread::current_thread_id() as i64))),
            "thread_info" => Ok(Some(thread::thread_info())),
            "cpu_cores" => Ok(Some(Value::Int(thread::available_parallelism()))),

            // Memory functions
            "size_of" => {
                if let Some(value) = args.first() {
                    Ok(Some(Value::Int(mem::size_of_value(value))))
                } else {
                    Ok(Some(Value::Int(0)))
                }
            }
            "mem_info" => Ok(Some(mem::memory_usage())),

            // Process functions
            "exit" => {
                let code = args
                    .get(0)
                    .and_then(|v| match v {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                process::exit(code);
            }
            "pid" => Ok(Some(Value::Int(process::pid()))),

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
