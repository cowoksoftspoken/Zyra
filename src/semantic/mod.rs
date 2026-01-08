//! Semantic Analyzer for Zyra
//!
//! Performs type checking, type inference, and ownership validation

pub mod borrow;
pub mod lifetime;
pub mod ownership;
pub mod scope;
pub mod types;

pub use borrow::{BorrowChecker, BorrowError, BorrowKind};
pub use lifetime::{LifetimeChecker, LifetimeError};
pub use ownership::{OwnershipChecker, OwnershipError};
pub use scope::{ReferenceInfo, ScopeId, ScopeStack, ValueOrigin, VariableInfo};
pub use types::ZyraType;

use std::collections::HashMap;

use crate::error::{SourceLocation, ZyraError, ZyraResult};
use crate::parser::ast::*;

/// Symbol table entry
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: ZyraType,
    pub mutable: bool,
    pub scope_depth: usize,
    /// Unique scope ID where this symbol was declared
    pub scope_id: ScopeId,
    /// Origin of the value (Param, Local, Temporary, Global)
    pub origin: ValueOrigin,
    /// Line where declared
    pub decl_line: usize,
}

/// Type definition in the type registry
#[derive(Debug, Clone)]
pub enum TypeDef {
    /// Struct type with name and fields
    Struct {
        name: String,
        fields: Vec<(String, ZyraType)>,
    },
    /// Enum type with name and variants
    Enum { name: String, variants: Vec<String> },
}

impl TypeDef {
    /// Convert to corresponding ZyraType
    pub fn to_zyra_type(&self) -> ZyraType {
        match self {
            TypeDef::Struct { name, .. } => ZyraType::Struct(name.clone()),
            TypeDef::Enum { name, .. } => ZyraType::Enum(name.clone()),
        }
    }
}

/// Unique identifier for expressions (for type tracking)
pub type ExprId = usize;

/// Semantic analyzer
pub struct SemanticAnalyzer {
    symbols: HashMap<String, Symbol>,
    functions: HashMap<String, FunctionSignature>,
    /// Type registry: maps type names to their definitions
    types: HashMap<String, TypeDef>,
    /// Expression type cache: maps expression IDs to their resolved types
    /// Used for type-aware features like DCE and compile-time borrow checking
    expr_types: HashMap<ExprId, ZyraType>,
    /// Counter for generating unique expression IDs
    next_expr_id: ExprId,
    ownership: OwnershipChecker,
    lifetime_checker: LifetimeChecker,
    borrow_checker: BorrowChecker,
    scope_depth: usize,
    current_function: Option<String>,
    errors: Vec<ZyraError>,
    /// Scope stack for tracking nested scopes with unique IDs
    scope_stack: ScopeStack,
    /// Tracks active references and their origins
    references: HashMap<String, ReferenceInfo>,
    /// Function scope for return checks
    #[allow(dead_code)]
    function_scope: Option<ScopeId>,
    /// Imported standard library modules (e.g., "std::math", "std::fs")
    imported_std_modules: std::collections::HashSet<String>,
    /// Specific items imported from std modules (e.g., "sqrt" from "std::math")
    imported_std_items: HashMap<String, String>, // item_name -> module_name
    /// Module aliases (e.g., "math" -> "std::math") for shorthand access
    module_aliases: HashMap<String, String>,
    /// Tracks if `self` is mutable in current method (None = not in method)
    self_is_mutable: Option<bool>,
}

/// Function signature for type checking
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<(String, ZyraType)>,
    pub return_type: ZyraType,
    pub lifetimes: Vec<String>,
    /// True if this is a method with `&mut self` (requires exclusive borrow)
    pub has_mut_self: bool,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            symbols: HashMap::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            expr_types: HashMap::new(),
            next_expr_id: 0,
            ownership: OwnershipChecker::new(),
            lifetime_checker: LifetimeChecker::new(),
            borrow_checker: BorrowChecker::new(),
            scope_depth: 0,
            current_function: None,
            errors: Vec::new(),
            scope_stack: ScopeStack::new(),
            references: HashMap::new(),
            function_scope: None,
            imported_std_modules: std::collections::HashSet::new(),
            imported_std_items: HashMap::new(),
            module_aliases: HashMap::new(),
            self_is_mutable: None,
        };

        // Register built-in functions
        analyzer.register_builtins();

        analyzer
    }

    /// Allocate a new unique expression ID
    fn alloc_expr_id(&mut self) -> ExprId {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        id
    }

    /// Store the resolved type for an expression
    fn store_expr_type(&mut self, id: ExprId, ty: ZyraType) {
        self.expr_types.insert(id, ty);
    }

    /// Get the resolved type for an expression
    pub fn get_expr_type(&self, id: ExprId) -> Option<&ZyraType> {
        self.expr_types.get(&id)
    }

    /// Look up a type definition in the type registry
    /// Returns Some(TypeDef) if the type exists, None otherwise
    pub fn lookup_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Check if a type name is registered (struct or enum)
    pub fn is_type_defined(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }

    /// Analyze an expression and track its type for later retrieval
    /// Returns the resolved type and stores it in expr_types cache
    fn analyze_and_track(&mut self, expr: &Expression) -> ZyraResult<ZyraType> {
        let expr_id = self.alloc_expr_id();
        let ty = self.analyze_expression(expr)?;
        self.store_expr_type(expr_id, ty.clone());
        Ok(ty)
    }

    fn register_builtins(&mut self) {
        // print function
        self.functions.insert(
            "print".to_string(),
            FunctionSignature {
                name: "print".to_string(),
                params: vec![("value".to_string(), ZyraType::Unknown)],
                return_type: ZyraType::Void,
                lifetimes: vec![],
                has_mut_self: false,
            },
        );

        // Input module functions
        self.functions.insert(
            "input.key".to_string(),
            FunctionSignature {
                name: "input.key".to_string(),
                params: vec![("key".to_string(), ZyraType::String)],
                return_type: ZyraType::Bool,
                lifetimes: vec![],
                has_mut_self: false,
            },
        );

        // Draw module functions
        self.functions.insert(
            "draw.rect".to_string(),
            FunctionSignature {
                name: "draw.rect".to_string(),
                params: vec![
                    ("x".to_string(), ZyraType::I32),
                    ("y".to_string(), ZyraType::I32),
                    ("w".to_string(), ZyraType::I32),
                    ("h".to_string(), ZyraType::I32),
                ],
                return_type: ZyraType::Void,
                lifetimes: vec![],
                has_mut_self: false,
            },
        );

        // Window constructor
        self.functions.insert(
            "Window".to_string(),
            FunctionSignature {
                name: "Window".to_string(),
                params: vec![
                    ("width".to_string(), ZyraType::I32),
                    ("height".to_string(), ZyraType::I32),
                    ("title".to_string(), ZyraType::String),
                ],
                return_type: ZyraType::Object(HashMap::new()),
                lifetimes: vec![],
                has_mut_self: false,
            },
        );
    }

    /// Register functions from a specific std module
    fn register_std_module_functions(
        &mut self,
        module_name: &str,
        specific_imports: Option<&Vec<String>>,
    ) {
        let functions: Vec<(&str, Vec<(&str, ZyraType)>, ZyraType)> = match module_name {
            "std::math" => vec![
                ("abs", vec![("x", ZyraType::F32)], ZyraType::F32),
                ("sqrt", vec![("x", ZyraType::F32)], ZyraType::F32),
                (
                    "pow",
                    vec![("base", ZyraType::F32), ("exp", ZyraType::F32)],
                    ZyraType::F32,
                ),
                ("sin", vec![("x", ZyraType::F32)], ZyraType::F32),
                ("cos", vec![("x", ZyraType::F32)], ZyraType::F32),
                ("tan", vec![("x", ZyraType::F32)], ZyraType::F32),
                (
                    "min",
                    vec![("a", ZyraType::F32), ("b", ZyraType::F32)],
                    ZyraType::F32,
                ),
                (
                    "max",
                    vec![("a", ZyraType::F32), ("b", ZyraType::F32)],
                    ZyraType::F32,
                ),
                ("floor", vec![("x", ZyraType::F32)], ZyraType::I32),
                ("ceil", vec![("x", ZyraType::F32)], ZyraType::I32),
                ("round", vec![("x", ZyraType::F32)], ZyraType::I32),
                (
                    "random",
                    vec![("min", ZyraType::I32), ("max", ZyraType::I32)],
                    ZyraType::I32,
                ),
                (
                    "lerp",
                    vec![
                        ("a", ZyraType::F32),
                        ("b", ZyraType::F32),
                        ("t", ZyraType::F32),
                    ],
                    ZyraType::F32,
                ),
                (
                    "clamp",
                    vec![
                        ("x", ZyraType::F32),
                        ("min", ZyraType::F32),
                        ("max", ZyraType::F32),
                    ],
                    ZyraType::F32,
                ),
                ("pi", vec![], ZyraType::F32),
                ("e", vec![], ZyraType::F32),
            ],
            "std::io" => vec![
                ("print", vec![("value", ZyraType::Unknown)], ZyraType::Void),
                (
                    "println",
                    vec![("value", ZyraType::Unknown)],
                    ZyraType::Void,
                ),
                ("input", vec![], ZyraType::String),
            ],
            "std::time" => vec![
                ("now", vec![], ZyraType::I64),
                ("now_secs", vec![], ZyraType::F64),
                ("sleep", vec![("ms", ZyraType::I32)], ZyraType::Void),
                ("monotonic_ms", vec![], ZyraType::I64),
                ("instant_now", vec![], ZyraType::I64),
                (
                    "instant_elapsed",
                    vec![("id", ZyraType::I64)],
                    ZyraType::I64,
                ),
                ("delta_time", vec![], ZyraType::F32),
                ("fps", vec![], ZyraType::F32),
            ],
            "std::string" => vec![
                ("string_len", vec![("s", ZyraType::String)], ZyraType::I32),
                ("to_upper", vec![("s", ZyraType::String)], ZyraType::String),
                ("to_lower", vec![("s", ZyraType::String)], ZyraType::String),
                ("trim", vec![("s", ZyraType::String)], ZyraType::String),
                (
                    "contains",
                    vec![("s", ZyraType::String), ("sub", ZyraType::String)],
                    ZyraType::Bool,
                ),
                (
                    "replace",
                    vec![
                        ("s", ZyraType::String),
                        ("from", ZyraType::String),
                        ("to", ZyraType::String),
                    ],
                    ZyraType::String,
                ),
                (
                    "split",
                    vec![("s", ZyraType::String), ("delim", ZyraType::String)],
                    ZyraType::Vec(Box::new(ZyraType::String)),
                ),
                ("parse_int", vec![("s", ZyraType::String)], ZyraType::I32),
                ("parse_float", vec![("s", ZyraType::String)], ZyraType::F32),
            ],
            "std::fs" => vec![
                (
                    "read_file",
                    vec![("path", ZyraType::String)],
                    ZyraType::String,
                ),
                (
                    "write_file",
                    vec![("path", ZyraType::String), ("content", ZyraType::String)],
                    ZyraType::Bool,
                ),
                (
                    "file_exists",
                    vec![("path", ZyraType::String)],
                    ZyraType::Bool,
                ),
                ("is_file", vec![("path", ZyraType::String)], ZyraType::Bool),
                ("is_dir", vec![("path", ZyraType::String)], ZyraType::Bool),
                (
                    "list_dir",
                    vec![("path", ZyraType::String)],
                    ZyraType::Vec(Box::new(ZyraType::String)),
                ),
                ("current_dir", vec![], ZyraType::String),
            ],
            "std::env" => vec![
                ("args", vec![], ZyraType::Vec(Box::new(ZyraType::String))),
                ("args_count", vec![], ZyraType::I64),
                (
                    "env_var",
                    vec![("name", ZyraType::String)],
                    ZyraType::String,
                ),
                ("os_name", vec![], ZyraType::String),
                ("os_arch", vec![], ZyraType::String),
                ("is_windows", vec![], ZyraType::Bool),
                ("is_linux", vec![], ZyraType::Bool),
                ("temp_dir", vec![], ZyraType::String),
                ("pid", vec![], ZyraType::I64),
            ],
            "std::process" => vec![("exit", vec![("code", ZyraType::I64)], ZyraType::Void)],
            "std::thread" => vec![
                ("thread_sleep", vec![("ms", ZyraType::I64)], ZyraType::Void),
                ("thread_yield", vec![], ZyraType::Void),
                ("thread_id", vec![], ZyraType::I64),
                ("cpu_cores", vec![], ZyraType::I64),
            ],
            "std::mem" => vec![
                ("size_of", vec![("value", ZyraType::Unknown)], ZyraType::I64),
                (
                    "type_of",
                    vec![("value", ZyraType::Unknown)],
                    ZyraType::String,
                ),
            ],
            "std::core" => vec![
                (
                    "assert",
                    vec![("condition", ZyraType::Bool), ("message", ZyraType::String)],
                    ZyraType::Void,
                ),
                ("panic", vec![("message", ZyraType::String)], ZyraType::Void),
                (
                    "is_none",
                    vec![("value", ZyraType::Unknown)],
                    ZyraType::Bool,
                ),
                (
                    "is_some",
                    vec![("value", ZyraType::Unknown)],
                    ZyraType::Bool,
                ),
                (
                    "unwrap",
                    vec![("value", ZyraType::Unknown)],
                    ZyraType::Unknown,
                ),
            ],
            "std::game" => vec![
                (
                    "Window",
                    vec![
                        ("width", ZyraType::I32),
                        ("height", ZyraType::I32),
                        ("title", ZyraType::String),
                    ],
                    ZyraType::Object(HashMap::new()),
                ),
                ("is_open", vec![], ZyraType::Bool),
                ("clear", vec![], ZyraType::Void),
                ("display", vec![], ZyraType::Void),
                (
                    "key_pressed",
                    vec![("key", ZyraType::String)],
                    ZyraType::Bool,
                ),
                (
                    "draw_rect",
                    vec![
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("w", ZyraType::I32),
                        ("h", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                (
                    "draw_rect_color",
                    vec![
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("w", ZyraType::I32),
                        ("h", ZyraType::I32),
                        ("color", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                (
                    "draw_digit",
                    vec![
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("digit", ZyraType::I32),
                        ("color", ZyraType::I32),
                    ],
                    ZyraType::I32,
                ),
                (
                    "draw_number",
                    vec![
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("num", ZyraType::I32),
                        ("scale", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                (
                    "draw_text_win",
                    vec![
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("scale", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                (
                    "draw_text_lose",
                    vec![
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("scale", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                // Sprites
                (
                    "load_sprite",
                    vec![("path", ZyraType::String)],
                    ZyraType::I64,
                ),
                (
                    "draw_sprite",
                    vec![
                        ("id", ZyraType::I64),
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                (
                    "draw_sprite_scaled",
                    vec![
                        ("id", ZyraType::I64),
                        ("x", ZyraType::I32),
                        ("y", ZyraType::I32),
                        ("scale", ZyraType::I32),
                    ],
                    ZyraType::Void,
                ),
                // Icons
                (
                    "set_window_icon",
                    vec![("path", ZyraType::String)],
                    ZyraType::Bool,
                ),
                ("is_icon_supported", vec![], ZyraType::Bool),
            ],
            _ => vec![],
        };

        for (name, params, return_type) in functions {
            let param_types: Vec<_> = params
                .into_iter()
                .map(|(n, t)| (n.to_string(), t))
                .collect();

            let sig = FunctionSignature {
                name: name.to_string(),
                params: param_types,
                return_type,
                lifetimes: vec![],
                has_mut_self: false,
            };

            // 1. Always register fully qualified name (e.g., std::math::sin)
            let qualified_name = format!("{}::{}", module_name, name);
            self.functions.insert(qualified_name.clone(), sig.clone());
            self.imported_std_items
                .insert(qualified_name.clone(), module_name.to_string());

            // 2. Register short name if specifically requested OR importing entire module
            let should_import = match specific_imports {
                None => true, // Import all check
                Some(list) => list.contains(&name.to_string()),
            };

            if should_import {
                self.functions.insert(name.to_string(), sig);
                // Track that this function came from this module
                self.imported_std_items
                    .insert(name.to_string(), module_name.to_string());
            }
        }
    }

    /// Check if a stdlib function is available (imported)
    pub fn is_stdlib_function_available(&self, name: &str) -> bool {
        // Always allow `print`, `println` as builtins
        if matches!(name, "print" | "println" | "input") {
            return true;
        }

        // Check if specifically imported
        self.imported_std_items.contains_key(name)
    }

    /// Check if a function name is a stdlib function
    pub fn is_stdlib_function(&self, name: &str) -> bool {
        // Builtins always available
        if matches!(name, "print" | "println" | "input") {
            return false; // Not a restricted stdlib function
        }

        // List of known stdlib function names
        const STDLIB_FUNCTIONS: &[&str] = &[
            // std::core
            "assert",
            "panic",
            "type_of",
            "is_none",
            "is_some",
            "unwrap",
            "expect",
            // std::math
            "abs",
            "sqrt",
            "pow",
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "atan2",
            "floor",
            "ceil",
            "round",
            "min",
            "max",
            "clamp",
            "lerp",
            "random",
            "random_range",
            "pi",
            "e",
            "log",
            "log10",
            "exp",
            // std::string
            "string_len",
            "to_upper",
            "to_lower",
            "trim",
            "trim_start",
            "trim_end",
            "contains",
            "starts_with",
            "ends_with",
            "replace",
            "split",
            "join",
            "parse_int",
            "parse_float",
            "char_at",
            "substring",
            // std::io
            "read_line",
            "write",
            "writeln",
            "flush",
            // std::time
            "now",
            "now_secs",
            "now_millis",
            "sleep",
            "monotonic_ms",
            "instant_now",
            "instant_elapsed",
            "delta_time",
            "fps",
            // std::fs
            "read_file",
            "write_file",
            "append_file",
            "file_exists",
            "delete_file",
            "create_dir",
            "list_dir",
            "is_file",
            "is_dir",
            "current_dir",
            // std::env
            "env_var",
            "set_env_var",
            "args",
            "args_count",
            "os_name",
            "os_arch",
            "is_windows",
            "is_linux",
            "is_macos",
            "home_dir",
            "temp_dir",
            "pid",
            // std::process
            "exit",
            "exec",
            "shell",
            "spawn",
            // std::thread
            "thread_spawn",
            "thread_join",
            "thread_sleep",
            "thread_yield",
            "thread_id",
            "thread_name",
            "cpu_cores",
            "thread_park",
            // std::mem
            "size_of",
            "drop",
            "take",
            "swap",
            "replace",
            // std::game
            "Window",
            "is_open",
            "clear",
            "display",
            "key_pressed",
            "draw_rect",
            "draw_circle",
            "draw_line",
            "draw_text",
            "set_color",
        ];

        STDLIB_FUNCTIONS.contains(&name)
    }

    /// Get the module that provides a stdlib function
    pub fn get_stdlib_module_for_function(&self, name: &str) -> Option<&'static str> {
        match name {
            // std::core
            "assert" | "panic" | "type_of" | "is_none" | "is_some" | "unwrap" | "expect" => {
                Some("std::core")
            }
            // std::math
            "abs" | "sqrt" | "pow" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2"
            | "floor" | "ceil" | "round" | "min" | "max" | "clamp" | "lerp" | "random"
            | "random_range" | "pi" | "e" | "log" | "log10" | "exp" => Some("std::math"),
            // std::string
            "string_len" | "to_upper" | "to_lower" | "trim" | "trim_start" | "trim_end"
            | "contains" | "starts_with" | "ends_with" | "replace" | "split" | "join"
            | "parse_int" | "parse_float" | "char_at" | "substring" => Some("std::string"),
            // std::io
            "read_line" | "write" | "writeln" | "flush" => Some("std::io"),
            // std::time
            "now" | "now_secs" | "now_millis" | "sleep" | "monotonic_ms" | "instant_now"
            | "instant_elapsed" | "delta_time" | "fps" => Some("std::time"),
            // std::fs
            "read_file" | "write_file" | "append_file" | "file_exists" | "delete_file"
            | "create_dir" | "list_dir" | "is_file" | "is_dir" | "current_dir" => Some("std::fs"),
            // std::env
            "env_var" | "set_env_var" | "args" | "args_count" | "os_name" | "os_arch"
            | "is_windows" | "is_linux" | "is_macos" | "home_dir" | "temp_dir" | "pid" => {
                Some("std::env")
            }
            // std::process
            "exit" | "exec" | "shell" | "spawn" => Some("std::process"),
            // std::thread
            "thread_spawn" | "thread_join" | "thread_sleep" | "thread_yield" | "thread_id"
            | "thread_name" | "cpu_cores" | "thread_park" => Some("std::thread"),
            // std::mem
            "size_of" | "drop" | "take" | "swap" => Some("std::mem"),
            // std::game
            "Window" | "is_open" | "clear" | "display" | "key_pressed" | "draw_rect"
            | "draw_circle" | "draw_line" | "draw_text" | "set_color" => Some("std::game"),
            _ => None,
        }
    }

    /// Analyze a program
    pub fn analyze(&mut self, program: &Program) -> ZyraResult<()> {
        // First pass: collect function signatures
        for stmt in &program.statements {
            if let Statement::Function {
                name,
                params,
                return_type,
                lifetimes,
                ..
            } = stmt
            {
                let param_types: Vec<_> = params
                    .iter()
                    .map(|p| (p.name.clone(), ZyraType::from_ast_type(&p.param_type)))
                    .collect();

                let ret_type = return_type
                    .as_ref()
                    .map(ZyraType::from_ast_type)
                    .unwrap_or(ZyraType::Void);

                // Detect &mut self: first param named "self" with mutable reference type
                let has_mut_self = params.first().map_or(false, |first_param| {
                    first_param.name == "self"
                        && matches!(
                            &first_param.param_type,
                            crate::parser::ast::Type::Reference { mutable: true, .. }
                        )
                });

                self.functions.insert(
                    name.clone(),
                    FunctionSignature {
                        name: name.clone(),
                        params: param_types,
                        return_type: ret_type,
                        lifetimes: lifetimes.clone(),
                        has_mut_self,
                    },
                );
            }
        }

        // Second pass: analyze statements
        // Check for illegal top-level code (executable statements outside functions)
        for stmt in &program.statements {
            match stmt {
                // These are allowed at top level
                Statement::Function { .. }
                | Statement::Struct { .. }
                | Statement::Enum { .. }
                | Statement::Impl { .. }
                | Statement::Trait { .. }
                | Statement::Import { .. } => {}

                // These are NOT allowed at top level
                Statement::Let { name, span, .. } => {
                    return Err(ZyraError::new(
                        "CompileError",
                        &format!(
                            "Top-level variable '{}' not allowed. Move it inside 'func main() {{ ... }}'",
                            name
                        ),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }
                Statement::Expression { span, .. } => {
                    return Err(ZyraError::new(
                        "CompileError",
                        "Top-level expressions not allowed. Move them inside 'func main() { ... }'",
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }
                Statement::Return { span, .. } => {
                    return Err(ZyraError::new(
                        "CompileError",
                        "Return statement outside of function",
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }
                Statement::If { span, .. }
                | Statement::While { span, .. }
                | Statement::For { span, .. } => {
                    return Err(ZyraError::new(
                        "CompileError",
                        "Control flow statements not allowed at top level. Move them inside 'func main() { ... }'",
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }
                Statement::Block { .. } => {
                    return Err(ZyraError::new(
                        "CompileError",
                        "Top-level blocks not allowed. Move them inside 'func main() { ... }'",
                        None,
                    ));
                }
            }
        }

        // Third pass: analyze statements
        for stmt in &program.statements {
            self.analyze_statement(stmt)?;
        }

        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }

        // *** MAIN FUNCTION REQUIRED ***
        // Programs must have a main() function as entry point
        if !self.functions.contains_key("main") {
            return Err(ZyraError::new(
                "CompileError",
                "No 'main' function found. Programs must have a 'func main() { ... }' as entry point.",
                None,
            ));
        }

        // Verify main() has no parameters
        if let Some(main_sig) = self.functions.get("main") {
            if !main_sig.params.is_empty() {
                return Err(ZyraError::new(
                    "CompileError",
                    "main() function must not have parameters.",
                    None,
                ));
            }
        }

        Ok(())
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> ZyraResult<ZyraType> {
        match stmt {
            Statement::Let {
                name,
                mutable,
                type_annotation,
                value,
                span,
            } => {
                // Infer type from value
                let value_type = self.analyze_expression(value)?;

                // Check type annotation matches
                if let Some(annotation) = type_annotation {
                    let annotated_type = ZyraType::from_ast_type(annotation);
                    if !annotated_type.is_compatible(&value_type) {
                        return Err(ZyraError::type_error(
                            &format!(
                                "Type mismatch: expected {}, found {}",
                                annotated_type.display_name(),
                                value_type.display_name()
                            ),
                            Some(SourceLocation::new("", span.line, span.column)),
                        ));
                    }
                }

                // Register in symbol table
                self.symbols.insert(
                    name.clone(),
                    Symbol {
                        name: name.clone(),
                        symbol_type: value_type.clone(),
                        mutable: *mutable,
                        scope_depth: self.scope_depth,
                        scope_id: self.scope_stack.current(),
                        origin: ValueOrigin::Local,
                        decl_line: span.line,
                    },
                );

                // Register in ownership checker
                self.ownership
                    .define(name, *mutable, span.line)
                    .map_err(|e| self.ownership_error_to_zyra(e))?;

                // Track ownership/borrow semantics based on expression type:
                // - Plain identifier: MOVE (ownership transfer)
                // - &identifier: SHARED BORROW (immutable reference)
                // - &mut identifier: MUTABLE BORROW (exclusive reference)
                match value {
                    // &expr or &mut expr = borrow
                    Expression::Reference {
                        mutable: is_mut,
                        value: inner_value,
                        ..
                    } => {
                        if let Expression::Identifier {
                            name: source_name, ..
                        } = inner_value.as_ref()
                        {
                            // Get source variable info for reference tracking
                            let (source_origin, origin_scope) =
                                if let Some(src_sym) = self.symbols.get(source_name) {
                                    (src_sym.origin, src_sym.scope_id)
                                } else {
                                    (ValueOrigin::Local, self.scope_stack.current())
                                };

                            if *is_mut {
                                // &mut = mutable borrow
                                if let Err(borrow_err) =
                                    self.borrow_checker
                                        .borrow_mutable(source_name, name, span.line)
                                {
                                    return Err(ZyraError::ownership_error(
                                        &format!("{}", borrow_err),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }
                            } else {
                                // & = shared borrow
                                if let Err(borrow_err) =
                                    self.borrow_checker
                                        .borrow_shared(source_name, name, span.line)
                                {
                                    return Err(ZyraError::ownership_error(
                                        &format!("{}", borrow_err),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }
                            }

                            // Track this reference for return checks
                            self.references.insert(
                                name.clone(),
                                ReferenceInfo {
                                    ref_name: name.clone(),
                                    source_name: source_name.clone(),
                                    use_scope: self.scope_stack.current(),
                                    origin_scope,
                                    source_origin,
                                    is_mutable: *is_mut,
                                    created_at: span.line,
                                },
                            );
                        } else {
                            // Borrowing a non-identifier (temporary value) - this is illegal!
                            // Examples: &42, &"hello", &(a + b)
                            return Err(ZyraError::ownership_error(
                                &format!(
                                    "cannot borrow temporary value\n\
                                     note: temporary values are dropped at the end of the statement\n\
                                     hint: store the value in a variable first, then borrow that variable"
                                ),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                    }
                    // Plain identifier = move
                    Expression::Identifier {
                        name: source_name, ..
                    } => {
                        if let Err(borrow_err) =
                            self.borrow_checker
                                .record_move(source_name, name, span.line)
                        {
                            return Err(ZyraError::ownership_error(
                                &format!("{}", borrow_err),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                    }
                    // Other expressions (literals, etc) - no ownership tracking needed
                    _ => {}
                }

                Ok(ZyraType::Void)
            }

            Statement::Function {
                name,
                lifetimes: lifetime_params,
                params,
                return_type,
                body,
                span,
            } => {
                // Enter function scope
                self.enter_scope();
                self.current_function = Some(name.clone());

                // Declare lifetime parameters
                for lt in lifetime_params {
                    self.lifetime_checker.declare_lifetime(lt);
                }

                // Register parameters
                for param in params {
                    let param_type = ZyraType::from_ast_type(&param.param_type);
                    // Normalize self parameter names: &self, &mut self, mut self -> self
                    let is_self_param = param.name == "&self"
                        || param.name == "&mut self"
                        || param.name == "mut self"
                        || param.name == "self";
                    let normalized_name = if is_self_param {
                        "self".to_string()
                    } else if param.name.starts_with("mut ") {
                        param.name[4..].to_string()
                    } else {
                        param.name.clone()
                    };
                    let is_mutable = param.name.contains("mut");

                    // Track self mutability for method body analysis
                    if is_self_param {
                        self.self_is_mutable = Some(is_mutable);
                    }

                    self.symbols.insert(
                        normalized_name.clone(),
                        Symbol {
                            name: normalized_name.clone(),
                            symbol_type: param_type.clone(),
                            mutable: is_mutable, // &mut self and mut params are mutable
                            scope_depth: self.scope_depth,
                            scope_id: self.scope_stack.current(),
                            origin: ValueOrigin::Param,
                            decl_line: span.line,
                        },
                    );
                    self.ownership
                        .define(&normalized_name, is_mutable, span.line)
                        .map_err(|e| self.ownership_error_to_zyra(e))?;
                }

                // Analyze body
                let body_type = self.analyze_block(body)?;

                // Check return type (only if body has a trailing expression, not explicit returns)
                // Functions with explicit `return` statements have Void body type but
                // returns are validated separately in Statement::Return handling
                if let Some(ret) = return_type {
                    let expected = ZyraType::from_ast_type(ret);
                    // Skip check if body is Void - explicit returns are checked separately
                    if !matches!(body_type, ZyraType::Void) && !expected.is_compatible(&body_type) {
                        self.errors.push(ZyraError::type_error(
                            &format!(
                                "Function '{}' should return {}, but body returns {}",
                                name,
                                expected.display_name(),
                                body_type.display_name()
                            ),
                            Some(SourceLocation::new("", span.line, span.column)),
                        ));
                    }
                }

                // Check for dangling reference returns
                // If function has a trailing expression, check it for reference escapes
                if let Some(ref trailing_expr) = body.expression {
                    self.check_return_expression(trailing_expr, span.line)?;
                }

                self.current_function = None;
                self.self_is_mutable = None;
                self.exit_scope();

                Ok(ZyraType::Void)
            }

            Statement::Expression { expr, .. } => self.analyze_expression(expr),

            Statement::Import {
                path,
                items,
                span: _,
            } => {
                // Import statements bring module functions into scope
                let root = path.first().map(|s| s.as_str()).unwrap_or("");

                match root {
                    "std" => {
                        // Standard library modules: std::math, std::fs, etc.
                        let module_name = path.join("::");
                        self.imported_std_modules.insert(module_name.clone());

                        // Register module alias (e.g., "math" -> "std::math")
                        // For nested modules like std::game::physics, register:
                        //   "physics" -> "std::game::physics"
                        //   "game::physics" -> "std::game::physics"
                        if path.len() >= 2 {
                            // Always register the last segment as alias
                            let short_alias = path.last().unwrap().clone();

                            // Check for conflicts with user-defined functions/types
                            if self.functions.contains_key(&short_alias) {
                                return Err(ZyraError::new(
                                    "ImportError",
                                    &format!(
                                        "Module alias '{}' conflicts with existing function. Rename your function or use fully qualified import.",
                                        short_alias
                                    ),
                                    None,
                                ));
                            }
                            if self.types.contains_key(&short_alias) {
                                return Err(ZyraError::new(
                                    "ImportError",
                                    &format!(
                                        "Module alias '{}' conflicts with existing type. Rename your type or use fully qualified import.",
                                        short_alias
                                    ),
                                    None,
                                ));
                            }

                            self.module_aliases.insert(short_alias, module_name.clone());

                            // For deeper nesting, also register intermediate paths
                            // e.g., std::game::physics -> also register "game::physics"
                            for i in 1..path.len() - 1 {
                                let intermediate_alias = path[i..].join("::");
                                self.module_aliases
                                    .insert(intermediate_alias, module_name.clone());
                            }
                        }

                        // If specific items are imported, register them
                        if !items.is_empty() {
                            self.register_std_module_functions(&module_name, Some(items));
                        } else {
                            // Import all functions from this module
                            self.register_std_module_functions(&module_name, None);
                        }
                        Ok(ZyraType::Void)
                    }
                    "game" | "math" | "io" | "time" | "fs" | "env" | "process" | "thread"
                    | "mem" | "string" | "core" => {
                        // Legacy single-word modules - convert to std:: form
                        let module_name = format!("std::{}", root);
                        self.imported_std_modules.insert(module_name.clone());
                        self.register_std_module_functions(&module_name, None);
                        Ok(ZyraType::Void)
                    }
                    "src" => {
                        // Local module imports - handled by resolver
                        Ok(ZyraType::Void)
                    }
                    _ => {
                        // Local module imports (utils, player, etc.) - handled by resolver
                        // These are allowed and resolved by ModuleResolver
                        Ok(ZyraType::Void)
                    }
                }
            }

            Statement::Return { value, span } => {
                let return_type = if let Some(expr) = value {
                    self.analyze_expression(expr)?
                } else {
                    ZyraType::Void
                };

                // Check against function return type
                if let Some(ref func_name) = self.current_function {
                    if let Some(sig) = self.functions.get(func_name) {
                        if !sig.return_type.is_compatible(&return_type) {
                            return Err(ZyraError::type_error(
                                &format!(
                                    "Return type mismatch: expected {}, found {}",
                                    sig.return_type.display_name(),
                                    return_type.display_name()
                                ),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                    }
                }

                Ok(return_type)
            }

            Statement::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let cond_type = self.analyze_expression(condition)?;

                if !matches!(cond_type, ZyraType::Bool | ZyraType::Unknown) {
                    return Err(ZyraError::type_error(
                        &format!("Condition must be Bool, found {}", cond_type.display_name()),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                self.analyze_block(then_block)?;

                if let Some(else_blk) = else_block {
                    self.analyze_block(else_blk)?;
                }

                Ok(ZyraType::Void)
            }

            Statement::While {
                condition,
                body,
                span,
            } => {
                let cond_type = self.analyze_expression(condition)?;

                if !matches!(cond_type, ZyraType::Bool | ZyraType::Unknown) {
                    return Err(ZyraError::type_error(
                        &format!(
                            "While condition must be Bool, found {}",
                            cond_type.display_name()
                        ),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                self.enter_scope();
                self.analyze_block(body)?;
                self.exit_scope();

                Ok(ZyraType::Void)
            }

            Statement::For {
                variable,
                start,
                end,
                inclusive: _,
                body,
                span,
            } => {
                let start_type = self.analyze_expression(start)?;
                let end_type = self.analyze_expression(end)?;

                if !matches!(
                    start_type,
                    ZyraType::I32 | ZyraType::I64 | ZyraType::Unknown
                ) {
                    return Err(ZyraError::type_error(
                        &format!(
                            "For loop start must be Int, found {}",
                            start_type.display_name()
                        ),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                if !matches!(end_type, ZyraType::I32 | ZyraType::I64 | ZyraType::Unknown) {
                    return Err(ZyraError::type_error(
                        &format!(
                            "For loop end must be Int, found {}",
                            end_type.display_name()
                        ),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                // Loop variable is in body scope
                self.enter_scope();
                self.symbols.insert(
                    variable.clone(),
                    Symbol {
                        name: variable.clone(),
                        symbol_type: ZyraType::I32,
                        mutable: false,
                        scope_depth: self.scope_depth,
                        scope_id: self.scope_stack.current(),
                        origin: ValueOrigin::Local,
                        decl_line: span.line,
                    },
                );
                self.ownership
                    .define(variable, false, span.line)
                    .map_err(|e| self.ownership_error_to_zyra(e))?;

                self.analyze_block(body)?;
                self.exit_scope();

                Ok(ZyraType::Void)
            }

            Statement::Block(block) => {
                self.enter_scope();
                let result = self.analyze_block(block)?;
                self.exit_scope();
                Ok(result)
            }

            // Type definitions
            Statement::Struct {
                name: _name,
                fields,
                span: _span,
            } => {
                // Register struct type in type system
                // For now, just validate field types
                for field in fields {
                    let _ = ZyraType::from_ast_type(&field.field_type);
                }
                Ok(ZyraType::Void)
            }

            Statement::Enum {
                name: _name,
                variants,
                span: _span,
            } => {
                // Register enum type in type system
                // Validate variant types
                for variant in variants {
                    if let Some(ref types) = variant.data {
                        for typ in types {
                            let _ = ZyraType::from_ast_type(typ);
                        }
                    }
                }
                Ok(ZyraType::Void)
            }

            Statement::Impl {
                target_type: _target_type,
                trait_name: _trait_name,
                methods,
                span: _span,
            } => {
                // Analyze impl methods
                for method in methods {
                    self.analyze_statement(method)?;
                }
                Ok(ZyraType::Void)
            }

            Statement::Trait {
                name: _name,
                methods,
                span: _span,
            } => {
                // Register trait in type system
                // Validate method signatures
                for method in methods {
                    for param in &method.params {
                        let _ = ZyraType::from_ast_type(&param.param_type);
                    }
                    if let Some(ref ret) = method.return_type {
                        let _ = ZyraType::from_ast_type(ret);
                    }
                }
                Ok(ZyraType::Void)
            }
        }
    }

    fn analyze_block(&mut self, block: &Block) -> ZyraResult<ZyraType> {
        for stmt in &block.statements {
            self.analyze_statement(stmt)?;
        }

        if let Some(ref expr) = block.expression {
            self.analyze_expression(expr)
        } else {
            Ok(ZyraType::Void)
        }
    }

    fn analyze_expression(&mut self, expr: &Expression) -> ZyraResult<ZyraType> {
        match expr {
            Expression::Int { .. } => Ok(ZyraType::I32), // Default integer literals to i32 (memory efficient)
            Expression::Float { .. } => Ok(ZyraType::F32), // Default float literals to f32 (memory efficient)
            Expression::Bool { .. } => Ok(ZyraType::Bool),
            Expression::Char { .. } => Ok(ZyraType::Char),
            Expression::String { .. } => Ok(ZyraType::String),

            Expression::Identifier { name, span } => {
                // Check ownership
                self.ownership
                    .use_binding(name, span.line)
                    .map_err(|e| self.ownership_error_to_zyra(e))?;

                // Check borrow checker for use-after-move
                if let Err(borrow_err) = self.borrow_checker.can_use(name, span.line) {
                    return Err(ZyraError::ownership_error(
                        &format!("{}", borrow_err),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                // Look up type
                if let Some(symbol) = self.symbols.get(name) {
                    Ok(symbol.symbol_type.clone())
                } else {
                    // Could be a module or built-in
                    Ok(ZyraType::Unknown)
                }
            }

            Expression::Binary {
                left,
                operator,
                right,
                span,
            } => {
                let left_type = self.analyze_expression(left)?;
                let right_type = self.analyze_expression(right)?;

                match operator {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo => {
                        if left_type.is_numeric() && right_type.is_numeric() {
                            // STRICT TYPE CHECKING: Types must match exactly
                            // No implicit type promotion - use explicit `as` cast
                            if left_type != right_type {
                                return Err(ZyraError::type_error(
                                    &format!(
                                        "Cannot apply '{}' between {} and {} - types must match. Use explicit cast: `value as {}`",
                                        operator.as_str(),
                                        left_type.display_name(),
                                        right_type.display_name(),
                                        left_type.display_name()
                                    ),
                                    Some(SourceLocation::new("", span.line, span.column)),
                                ));
                            }

                            // Return the same type
                            Ok(left_type)
                        } else if matches!(operator, BinaryOp::Add)
                            && matches!(left_type, ZyraType::String)
                        {
                            Ok(ZyraType::String)
                        } else if matches!(left_type, ZyraType::Unknown)
                            || matches!(right_type, ZyraType::Unknown)
                        {
                            Ok(ZyraType::Unknown)
                        } else {
                            Err(ZyraError::type_error(
                                &format!(
                                    "Cannot apply '{}' to {} and {}",
                                    operator.as_str(),
                                    left_type.display_name(),
                                    right_type.display_name()
                                ),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ))
                        }
                    }
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Ok(ZyraType::Bool),
                    BinaryOp::And | BinaryOp::Or => {
                        if !matches!(left_type, ZyraType::Bool | ZyraType::Unknown) {
                            return Err(ZyraError::type_error(
                                &format!("Left side of '{}' must be Bool", operator.as_str()),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                        if !matches!(right_type, ZyraType::Bool | ZyraType::Unknown) {
                            return Err(ZyraError::type_error(
                                &format!("Right side of '{}' must be Bool", operator.as_str()),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                        Ok(ZyraType::Bool)
                    }
                }
            }

            Expression::Unary {
                operator,
                operand,
                span,
            } => {
                let operand_type = self.analyze_expression(operand)?;

                match operator {
                    UnaryOp::Negate => {
                        if operand_type.is_numeric() || matches!(operand_type, ZyraType::Unknown) {
                            Ok(operand_type)
                        } else {
                            Err(ZyraError::type_error(
                                &format!("Cannot negate {}", operand_type.display_name()),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ))
                        }
                    }
                    UnaryOp::Not => {
                        if matches!(operand_type, ZyraType::Bool | ZyraType::Unknown) {
                            Ok(ZyraType::Bool)
                        } else {
                            Err(ZyraError::type_error(
                                &format!("Cannot apply '!' to {}", operand_type.display_name()),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ))
                        }
                    }
                }
            }

            Expression::Assignment {
                target,
                value,
                span,
            } => {
                // Check target is assignable
                if let Expression::Identifier { name, .. } = target.as_ref() {
                    self.ownership
                        .assign(name, span.line)
                        .map_err(|e| self.ownership_error_to_zyra(e))?;

                    // Check borrow checker for mutate-while-borrowed
                    if let Err(borrow_err) = self.borrow_checker.can_mutate(name, span.line) {
                        return Err(ZyraError::ownership_error(
                            &format!("{}", borrow_err),
                            Some(SourceLocation::new("", span.line, span.column)),
                        ));
                    }
                } else if let Expression::FieldAccess { object, field, .. } = target.as_ref() {
                    // Check if assigning to self.field through immutable &self
                    if let Expression::Identifier { name, .. } = object.as_ref() {
                        if name == "self" {
                            // Check if self is mutable in current method
                            if let Some(is_mutable) = self.self_is_mutable {
                                if !is_mutable {
                                    return Err(ZyraError::ownership_error(
                                        &format!(
                                            "Cannot mutate field '{}' through immutable &self. Use &mut self instead",
                                            field
                                        ),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }
                            }
                        } else {
                            // Check if the object variable is mutable
                            if let Some(symbol) = self.symbols.get(name) {
                                if !symbol.mutable {
                                    return Err(ZyraError::ownership_error(
                                        &format!(
                                            "Cannot mutate field '{}' of immutable variable '{}'",
                                            field, name
                                        ),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }
                            }
                        }
                    }
                }

                let value_type = self.analyze_expression(value)?;
                Ok(value_type)
            }

            Expression::Call {
                callee,
                arguments,
                span,
            } => {
                // Get function name from callee
                // For method calls (obj.method), we use the RECEIVER TYPE name, not variable name
                // Also track receiver variable for &mut self borrow checking
                let (func_name, receiver_var_for_borrow) = match callee.as_ref() {
                    Expression::Identifier { name, .. } => (name.clone(), None),
                    Expression::FieldAccess { object, field, .. } => {
                        // Analyze the object to get its type and track it
                        let receiver_type = self.analyze_and_track(object)?;

                        // Extract receiver variable name for borrow checking
                        let receiver_var: Option<String> =
                            if let Expression::Identifier { name, .. } = object.as_ref() {
                                Some(name.clone())
                            } else {
                                None
                            };

                        // Use the type name for method resolution (enables type-aware DCE)
                        let func_name = match &receiver_type {
                            ZyraType::Struct(type_name) => {
                                format!("{}::{}", type_name, field)
                            }
                            ZyraType::Enum(type_name) => {
                                format!("{}::{}", type_name, field)
                            }
                            _ => {
                                // Static call or unknown type - try variable name as Type name
                                if let Expression::Identifier { name, .. } = object.as_ref() {
                                    format!("{}::{}", name, field)
                                } else {
                                    // Method call on complex expression
                                    field.clone()
                                }
                            }
                        };
                        (func_name, receiver_var)
                    }
                    _ => return Ok(ZyraType::Unknown),
                };

                // *** MODULE ALIAS EXPANSION ***
                // Expand aliased names like "math::abs" to "std::math::abs"
                let func_name = if func_name.contains("::") {
                    let parts: Vec<&str> = func_name.splitn(2, "::").collect();
                    if parts.len() == 2 {
                        if let Some(full_module) = self.module_aliases.get(parts[0]) {
                            // Expand alias: math::abs -> std::math::abs
                            format!("{}::{}", full_module, parts[1])
                        } else {
                            func_name
                        }
                    } else {
                        func_name
                    }
                } else {
                    func_name
                };

                // *** MAIN FUNCTION PROTECTION ***
                // main() is the program entry point and cannot be called directly
                if func_name == "main" {
                    return Err(ZyraError::new(
                        "SemanticError",
                        "Cannot call 'main' - it is the program entry point",
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                // *** STDLIB IMPORT ENFORCEMENT ***
                // Check if this is a stdlib function that requires import
                if self.is_stdlib_function(&func_name)
                    && !self.is_stdlib_function_available(&func_name)
                {
                    let module = self
                        .get_stdlib_module_for_function(&func_name)
                        .unwrap_or("std::?");
                    return Err(ZyraError::new(
                        "ImportError",
                        &format!(
                            "Function '{}' requires import. Add: import {};",
                            func_name, module
                        ),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                // Check argument types
                let mut arg_types = Vec::new();
                for arg in arguments {
                    arg_types.push(self.analyze_expression(arg)?);
                }

                // *** COMPILE-TIME ICON FORMAT CHECK ***
                // On Windows, set_window_icon requires .ico files
                #[cfg(target_os = "windows")]
                {
                    let short_name = if let Some(idx) = func_name.rfind("::") {
                        &func_name[idx + 2..]
                    } else {
                        &func_name
                    };

                    if short_name == "set_window_icon" || short_name == "set_icon" {
                        // Check if first argument is a string literal
                        if let Some(first_arg) = arguments.first() {
                            if let Expression::String { value: path, .. } = first_arg {
                                // Check 1: Must be .ico on Windows
                                if !path.to_lowercase().ends_with(".ico") {
                                    return Err(ZyraError::new(
                                        "TypeError",
                                        &format!(
                                            "set_window_icon requires .ico file on Windows. Got: '{}'. \
                                             Please convert your icon to .ico format.",
                                            path
                                        ),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }

                                // Check 2: File must exist
                                if !std::path::Path::new(path).exists() {
                                    return Err(ZyraError::new(
                                        "FileError",
                                        &format!(
                                            "Icon file not found: '{}'. \
                                             Please ensure the file exists at the specified path.",
                                            path
                                        ),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }
                            }
                        }
                    }
                }

                // *** COMPILE-TIME ICON FILE CHECK (non-Windows) ***
                // On non-Windows, just check if file exists
                #[cfg(not(target_os = "windows"))]
                {
                    let short_name = if let Some(idx) = func_name.rfind("::") {
                        &func_name[idx + 2..]
                    } else {
                        &func_name
                    };

                    if short_name == "set_window_icon" || short_name == "set_icon" {
                        if let Some(first_arg) = arguments.first() {
                            if let Expression::String { value: path, .. } = first_arg {
                                if !std::path::Path::new(path).exists() {
                                    return Err(ZyraError::new(
                                        "FileError",
                                        &format!(
                                            "Icon file not found: '{}'. \
                                             Please ensure the file exists at the specified path.",
                                            path
                                        ),
                                        Some(SourceLocation::new("", span.line, span.column)),
                                    ));
                                }
                            }
                        }
                    }
                }

                // Look up function signature
                // Try full name first (e.g., "paddle::move_up"), then short name (e.g., "move_up")
                let sig_option = self.functions.get(&func_name).or_else(|| {
                    // If prefixed lookup fails, try just the function name (after ::)
                    if let Some(idx) = func_name.rfind("::") {
                        let short_name = &func_name[idx + 2..];
                        self.functions.get(short_name)
                    } else {
                        None
                    }
                });

                if let Some(sig) = sig_option {
                    if arguments.len() != sig.params.len() {
                        return Err(ZyraError::type_error(
                            &format!(
                                "Function '{}' expects {} argument(s), got {}",
                                func_name,
                                sig.params.len(),
                                arguments.len()
                            ),
                            Some(SourceLocation::new("", span.line, span.column)),
                        ));
                    }

                    // *** COMPILE-TIME BORROW CHECK FOR &mut self ***
                    // If this is a method with &mut self, ensure the receiver can be mutably borrowed
                    if sig.has_mut_self {
                        if let Some(ref receiver_var) = receiver_var_for_borrow {
                            // Check if receiver can be mutably borrowed (no active borrows)
                            if let Err(borrow_err) = self.borrow_checker.borrow_mutable(
                                receiver_var,
                                &format!("&mut self in {}", func_name),
                                span.line,
                            ) {
                                return Err(ZyraError::ownership_error(
                                    &format!(
                                        "Cannot call method '{}' requiring &mut self: {}",
                                        func_name, borrow_err
                                    ),
                                    Some(SourceLocation::new("", span.line, span.column)),
                                ));
                            }
                        }
                    }

                    // Check each argument type matches parameter type
                    for (i, (arg_type, (_, param_type))) in
                        arg_types.iter().zip(sig.params.iter()).enumerate()
                    {
                        // param_type.is_compatible(arg_type) checks if param accepts arg (widening I32->I64)
                        if !param_type.is_compatible(arg_type)
                            && !matches!(arg_type, ZyraType::Unknown)
                            && !matches!(param_type, ZyraType::Unknown)
                        {
                            return Err(ZyraError::type_error(
                                &format!(
                                    "Function '{}' argument {} expects {}, got {}",
                                    func_name,
                                    i + 1,
                                    param_type.display_name(),
                                    arg_type.display_name()
                                ),
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                    }

                    // *** MOVE SEMANTICS TRACKING ***
                    // If argument is a variable and parameter expects ownership (not reference),
                    // mark the variable as moved. Skip Copy types (they don't move).
                    for (i, (arg, (_, param_type))) in
                        arguments.iter().zip(sig.params.iter()).enumerate()
                    {
                        // Check if param type is NOT a reference (ownership transfer)
                        let is_reference_param = matches!(param_type, ZyraType::Reference { .. });

                        if !is_reference_param {
                            // If argument is an identifier, check if it should be moved
                            if let Expression::Identifier {
                                name,
                                span: arg_span,
                            } = arg
                            {
                                // Get the argument's type from our earlier analysis
                                let arg_type = arg_types.get(i);

                                // Skip self and Copy types (Int, Float, Bool, Char - stack only)
                                let is_copy = arg_type.map(|t| t.is_copy_type()).unwrap_or(false);

                                if name != "self" && !is_copy {
                                    // Only Reference types trigger move
                                    // Mark as moved - subsequent use will error
                                    let _ =
                                        self.ownership.move_value(name, &func_name, arg_span.line);
                                }
                            }
                        }
                    }

                    Ok(sig.return_type.clone())
                } else {
                    // Allow unknown functions (for flexibility)
                    Ok(ZyraType::Unknown)
                }
            }

            Expression::FieldAccess { object, field, .. } => {
                let obj_type = self.analyze_expression(object)?;

                // Check for method calls on known types
                match &obj_type {
                    ZyraType::String => match field.as_str() {
                        "len" => Ok(ZyraType::I32),
                        _ => Ok(ZyraType::Unknown),
                    },
                    ZyraType::Vec(_) => match field.as_str() {
                        "len" | "length" => Ok(ZyraType::I32),
                        "push" | "pop" => Ok(ZyraType::Void),
                        _ => Ok(ZyraType::Unknown),
                    },
                    _ => Ok(ZyraType::Unknown),
                }
            }

            Expression::Index {
                object,
                index,
                span,
            } => {
                let obj_type = self.analyze_expression(object)?;
                let idx_type = self.analyze_expression(index)?;

                if !idx_type.is_integer() && !matches!(idx_type, ZyraType::Unknown) {
                    return Err(ZyraError::type_error(
                        &format!("Index must be integer, found {}", idx_type.display_name()),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                match obj_type {
                    ZyraType::Vec(inner) => Ok(*inner),
                    ZyraType::Array { elem, .. } => Ok(*elem),
                    ZyraType::String => Ok(ZyraType::String),
                    ZyraType::Unknown => Ok(ZyraType::Unknown),
                    _ => Err(ZyraError::type_error(
                        &format!("Cannot index {}", obj_type.display_name()),
                        Some(SourceLocation::new("", span.line, span.column)),
                    )),
                }
            }

            Expression::List { elements, .. } => {
                // Array literal [a, b, c] - fixed size, inferred as Array type
                if elements.is_empty() {
                    Ok(ZyraType::Array {
                        elem: Box::new(ZyraType::Unknown),
                        size: 0,
                    })
                } else {
                    let first_type = self.analyze_expression(&elements[0])?;
                    Ok(ZyraType::Array {
                        elem: Box::new(first_type),
                        size: elements.len(),
                    })
                }
            }

            Expression::VecLiteral { elements, .. } => {
                // Vec literal vec[a, b, c] - dynamic, resizable
                if elements.is_empty() {
                    Ok(ZyraType::Vec(Box::new(ZyraType::Unknown)))
                } else {
                    let first_type = self.analyze_expression(&elements[0])?;
                    Ok(ZyraType::Vec(Box::new(first_type)))
                }
            }

            Expression::Object { fields, .. } => {
                let mut field_types = HashMap::new();
                for (name, expr) in fields {
                    let field_type = self.analyze_expression(expr)?;
                    field_types.insert(name.clone(), field_type);
                }
                Ok(ZyraType::Object(field_types))
            }

            Expression::Reference {
                mutable,
                value,
                span,
            } => {
                // Check if we can borrow
                if let Expression::Identifier { name, .. } = value.as_ref() {
                    if *mutable {
                        self.ownership
                            .borrow_mut(name, "ref", span.line)
                            .map_err(|e| self.ownership_error_to_zyra(e))?;
                    } else {
                        self.ownership
                            .borrow(name, "ref", span.line)
                            .map_err(|e| self.ownership_error_to_zyra(e))?;
                    }
                }

                let inner = self.analyze_expression(value)?;
                Ok(ZyraType::Reference {
                    lifetime: None,
                    mutable: *mutable,
                    inner: Box::new(inner),
                })
            }

            Expression::Dereference { value, .. } => {
                let val_type = self.analyze_expression(value)?;
                match val_type {
                    ZyraType::Reference { inner, .. } => Ok(*inner),
                    _ => Ok(ZyraType::Unknown),
                }
            }

            Expression::Range { start, end, .. } => {
                self.analyze_expression(start)?;
                self.analyze_expression(end)?;
                Ok(ZyraType::Unknown) // Range type
            }

            Expression::Grouped { inner, .. } => self.analyze_expression(inner),

            // If expression - analyze branches and determine type
            Expression::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // Condition must be bool
                let cond_type = self.analyze_expression(condition)?;
                if !matches!(cond_type, ZyraType::Bool | ZyraType::Unknown) {
                    self.errors
                        .push(ZyraError::type_error("If condition must be Bool", None));
                }

                // Analyze then block
                let then_type = self.analyze_block(then_block)?;

                // Analyze else block if present
                if let Some(else_blk) = else_block {
                    let else_type = self.analyze_block(else_blk)?;
                    // Both branches should return compatible types
                    if then_type.is_compatible(&else_type) {
                        Ok(then_type)
                    } else {
                        Ok(ZyraType::Unknown)
                    }
                } else {
                    Ok(ZyraType::Void)
                }
            }

            // Struct instantiation: StructName { field: value, ... }
            Expression::StructInit { name, fields, .. } => {
                // Analyze all field values
                for (_, field_value) in fields {
                    self.analyze_expression(field_value)?;
                }
                // Return the struct type
                Ok(ZyraType::Struct(name.clone()))
            }

            // Enum variant: EnumName::Variant
            Expression::EnumVariant {
                enum_name, data, ..
            } => {
                // Analyze data if present
                if let Some(data_expr) = data {
                    self.analyze_expression(data_expr)?;
                }
                // Return the enum type
                Ok(ZyraType::Enum(enum_name.clone()))
            }

            // Match expression: match scrutinee { pattern => body, ... }
            Expression::Match {
                scrutinee,
                arms,
                span,
            } => {
                // Analyze scrutinee to get its type
                let scrutinee_type = self.analyze_expression(scrutinee)?;

                // Track return types from all arms
                let mut arm_types: Vec<ZyraType> = Vec::new();

                // Analyze each arm
                for arm in arms {
                    // Check guard purity if present
                    if let Some(ref guard) = arm.guard {
                        Self::check_guard_purity(guard)?;
                        // Analyze guard expression
                        let guard_type = self.analyze_expression(guard)?;
                        if !matches!(guard_type, ZyraType::Bool | ZyraType::Unknown) {
                            return Err(ZyraError::type_error(
                                "Match guard must be a boolean expression",
                                Some(SourceLocation::new("", span.line, span.column)),
                            ));
                        }
                    }

                    // Enter new scope for pattern bindings
                    self.enter_scope();

                    // Introduce pattern bindings into scope
                    self.analyze_pattern_bindings(&arm.pattern, &scrutinee_type)?;

                    // Analyze arm body
                    let body_type = self.analyze_expression(&arm.body)?;
                    arm_types.push(body_type);

                    // Exit pattern scope
                    self.exit_scope();
                }

                // Check exhaustiveness (conservative: require _ or all variants)
                Self::check_exhaustiveness(arms, &scrutinee_type, *span)?;

                // Return type is common type of all arms (or Unknown if mixed)
                if let Some(first) = arm_types.first() {
                    if arm_types.iter().all(|t| t.is_compatible(first)) {
                        Ok(first.clone())
                    } else {
                        Ok(ZyraType::Unknown)
                    }
                } else {
                    Ok(ZyraType::Void)
                }
            }

            // Type cast expression: expr as Type
            Expression::Cast {
                expr,
                target_type,
                span,
            } => {
                let source_type = self.analyze_expression(expr)?;
                let target = ZyraType::from_ast_type(target_type);

                // Validate cast is allowed
                if !source_type.is_castable(&target) {
                    return Err(ZyraError::type_error(
                        &format!(
                            "Cannot cast {} to {}",
                            source_type.display_name(),
                            target.display_name()
                        ),
                        Some(SourceLocation::new("", span.line, span.column)),
                    ));
                }

                // Cast succeeds - return target type
                Ok(target)
            }

            // Closure expression: |params| body
            Expression::Closure {
                params,
                return_type,
                body,
                capture_mode,
                span,
            } => {
                // Collect outer scope variable names before entering closure scope
                let outer_scope_vars: std::collections::HashSet<String> =
                    self.symbols.keys().cloned().collect();

                // Collect param names to exclude from captures
                let param_names: std::collections::HashSet<String> =
                    params.iter().map(|p| p.name.clone()).collect();

                // Increment scope depth for closure
                self.scope_depth += 1;
                let current_scope_id = self.scope_stack.current();

                // Register closure parameters in symbol table
                let param_types: Vec<ZyraType> = params
                    .iter()
                    .map(|p| {
                        let param_type = p
                            .param_type
                            .as_ref()
                            .map(|t| ZyraType::from_ast_type(t))
                            .unwrap_or(ZyraType::I32); // Default to i32 if not specified

                        // Register parameter in symbol table
                        self.symbols.insert(
                            p.name.clone(),
                            Symbol {
                                name: p.name.clone(),
                                symbol_type: param_type.clone(),
                                mutable: false,
                                scope_depth: self.scope_depth,
                                scope_id: current_scope_id,
                                origin: ValueOrigin::Param,
                                decl_line: span.line,
                            },
                        );

                        // Also register with ownership checker
                        let _ = self.ownership.define(&p.name, false, span.line);

                        param_type
                    })
                    .collect();

                // Analyze closure body with parameters in scope
                let body_type = self.analyze_expression(body)?;

                // Detect captured variables: outer scope vars referenced in body
                let captured_vars =
                    self.detect_captured_variables(body, &outer_scope_vars, &param_names);

                // Enforce ownership rules based on capture_mode
                for captured_var in &captured_vars {
                    match capture_mode {
                        crate::parser::ast::CaptureMode::Move => {
                            // Move semantics: mark variable as moved
                            let move_result = self.ownership.move_value(
                                captured_var,
                                &format!("closure_capture_{}", captured_var),
                                span.line,
                            );
                            if let Err(e) = move_result {
                                return Err(ZyraError::ownership_error(
                                    &format!("Cannot move '{}' into closure: {}", captured_var, e),
                                    Some(SourceLocation::new("", span.line, span.column)),
                                ));
                            }
                        }
                        crate::parser::ast::CaptureMode::Borrow => {
                            // Borrow semantics: create immutable borrow
                            let borrow_result = self.ownership.borrow(
                                captured_var,
                                &format!("closure_capture_{}", captured_var),
                                span.line,
                            );
                            if let Err(e) = borrow_result {
                                return Err(ZyraError::ownership_error(
                                    &format!("Cannot borrow '{}' in closure: {}", captured_var, e),
                                    Some(SourceLocation::new("", span.line, span.column)),
                                ));
                            }
                        }
                    }
                }

                // Remove closure params from symbols (they go out of scope)
                for p in params {
                    self.symbols.remove(&p.name);
                }

                // Exit closure scope
                self.scope_depth -= 1;

                let ret_type = return_type
                    .as_ref()
                    .map(|t| ZyraType::from_ast_type(t))
                    .unwrap_or(body_type);

                Ok(ZyraType::Closure {
                    params: param_types,
                    return_type: Box::new(ret_type),
                })
            }
        }
    }

    /// Check that a match guard is pure (no side effects)
    fn check_guard_purity(guard: &Expression) -> ZyraResult<()> {
        match guard {
            // Assignments are side effects
            Expression::Assignment { span, .. } => Err(ZyraError::type_error(
                "Match guard cannot contain assignment (must be pure)",
                Some(SourceLocation::new("", span.line, span.column)),
            )),
            // Function calls may have side effects - conservative rejection
            Expression::Call { span, .. } => Err(ZyraError::type_error(
                "Match guard cannot contain function calls (must be pure)",
                Some(SourceLocation::new("", span.line, span.column)),
            )),
            // Binary and unary expressions are pure if operands are pure
            Expression::Binary { left, right, .. } => {
                Self::check_guard_purity(left)?;
                Self::check_guard_purity(right)?;
                Ok(())
            }
            Expression::Unary { operand, .. } => {
                Self::check_guard_purity(operand)?;
                Ok(())
            }
            // Identifiers, literals are pure
            Expression::Identifier { .. }
            | Expression::Int { .. }
            | Expression::Float { .. }
            | Expression::Bool { .. }
            | Expression::Char { .. }
            | Expression::String { .. } => Ok(()),
            // Field access is pure
            Expression::FieldAccess { object, .. } => Self::check_guard_purity(object),
            // Grouped expressions
            Expression::Grouped { inner, .. } => Self::check_guard_purity(inner),
            // Other expressions - allow for now
            _ => Ok(()),
        }
    }

    /// Detect variables captured from outer scope in a closure body
    fn detect_captured_variables(
        &self,
        expr: &Expression,
        outer_scope_vars: &std::collections::HashSet<String>,
        param_names: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut captured = Vec::new();
        self.collect_variable_refs(expr, outer_scope_vars, param_names, &mut captured);
        // Remove duplicates
        captured.sort();
        captured.dedup();
        captured
    }

    /// Recursively collect variable references from an expression
    fn collect_variable_refs(
        &self,
        expr: &Expression,
        outer_scope_vars: &std::collections::HashSet<String>,
        param_names: &std::collections::HashSet<String>,
        captured: &mut Vec<String>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                // If it's an outer scope var and not a param, it's captured
                if outer_scope_vars.contains(name) && !param_names.contains(name) {
                    captured.push(name.clone());
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_variable_refs(left, outer_scope_vars, param_names, captured);
                self.collect_variable_refs(right, outer_scope_vars, param_names, captured);
            }
            Expression::Unary { operand, .. } => {
                self.collect_variable_refs(operand, outer_scope_vars, param_names, captured);
            }
            Expression::Call { arguments, .. } => {
                for arg in arguments {
                    self.collect_variable_refs(arg, outer_scope_vars, param_names, captured);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_variable_refs(object, outer_scope_vars, param_names, captured);
            }
            Expression::Index { object, index, .. } => {
                self.collect_variable_refs(object, outer_scope_vars, param_names, captured);
                self.collect_variable_refs(index, outer_scope_vars, param_names, captured);
            }
            Expression::Grouped { inner, .. } => {
                self.collect_variable_refs(inner, outer_scope_vars, param_names, captured);
            }
            Expression::List { elements, .. } => {
                for elem in elements {
                    self.collect_variable_refs(elem, outer_scope_vars, param_names, captured);
                }
            }
            Expression::Cast { expr, .. } => {
                self.collect_variable_refs(expr, outer_scope_vars, param_names, captured);
            }
            Expression::Closure { body, .. } => {
                // Nested closures - recursively check but don't capture their params
                self.collect_variable_refs(body, outer_scope_vars, param_names, captured);
            }
            Expression::Reference { value, .. } => {
                self.collect_variable_refs(value, outer_scope_vars, param_names, captured);
            }
            Expression::Dereference { value, .. } => {
                self.collect_variable_refs(value, outer_scope_vars, param_names, captured);
            }
            Expression::Assignment { target, value, .. } => {
                self.collect_variable_refs(target, outer_scope_vars, param_names, captured);
                self.collect_variable_refs(value, outer_scope_vars, param_names, captured);
            }
            // Literals don't capture
            Expression::Int { .. }
            | Expression::Float { .. }
            | Expression::Bool { .. }
            | Expression::Char { .. }
            | Expression::String { .. } => {}
            // Other expressions - skip for simplicity
            _ => {}
        }
    }

    /// Collect variable refs from a statement
    #[allow(dead_code)]
    fn collect_variable_refs_from_stmt(
        &self,
        stmt: &Statement,
        outer_scope_vars: &std::collections::HashSet<String>,
        param_names: &std::collections::HashSet<String>,
        captured: &mut Vec<String>,
    ) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.collect_variable_refs(expr, outer_scope_vars, param_names, captured);
            }
            Statement::Let { value, .. } => {
                self.collect_variable_refs(value, outer_scope_vars, param_names, captured);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_variable_refs(expr, outer_scope_vars, param_names, captured);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_variable_refs(condition, outer_scope_vars, param_names, captured);
                for s in &then_block.statements {
                    self.collect_variable_refs_from_stmt(
                        s,
                        outer_scope_vars,
                        param_names,
                        captured,
                    );
                }
                if let Some(else_blk) = else_block {
                    for s in &else_blk.statements {
                        self.collect_variable_refs_from_stmt(
                            s,
                            outer_scope_vars,
                            param_names,
                            captured,
                        );
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.collect_variable_refs(condition, outer_scope_vars, param_names, captured);
                for s in &body.statements {
                    self.collect_variable_refs_from_stmt(
                        s,
                        outer_scope_vars,
                        param_names,
                        captured,
                    );
                }
            }
            _ => {}
        }
    }

    /// Introduce pattern bindings into current scope (simplified)
    fn analyze_pattern_bindings(
        &mut self,
        pattern: &crate::parser::ast::Pattern,
        scrutinee_type: &ZyraType,
    ) -> ZyraResult<()> {
        use crate::parser::ast::Pattern;
        match pattern {
            Pattern::Identifier {
                name,
                mutable,
                span,
            } => {
                // Add to symbol table
                self.symbols.insert(
                    name.clone(),
                    Symbol {
                        name: name.clone(),
                        symbol_type: scrutinee_type.clone(),
                        mutable: *mutable,
                        scope_depth: self.scope_depth,
                        scope_id: self.scope_stack.current(),
                        origin: ValueOrigin::Local,
                        decl_line: span.line,
                    },
                );
                // Register with ownership checker
                let _ = self.ownership.define(name, *mutable, span.line);
                Ok(())
            }
            Pattern::RefBinding { name, span } => {
                // Add to symbol table
                self.symbols.insert(
                    name.clone(),
                    Symbol {
                        name: name.clone(),
                        symbol_type: scrutinee_type.clone(),
                        mutable: false,
                        scope_depth: self.scope_depth,
                        scope_id: self.scope_stack.current(),
                        origin: ValueOrigin::Local,
                        decl_line: span.line,
                    },
                );
                // Register with ownership checker (immutable ref binding)
                let _ = self.ownership.define(name, false, span.line);
                Ok(())
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    self.analyze_pattern_bindings(&field.pattern, &ZyraType::Unknown)?;
                }
                Ok(())
            }
            Pattern::Variant { inner, .. } => {
                if let Some(inner_pattern) = inner {
                    self.analyze_pattern_bindings(inner_pattern, &ZyraType::Unknown)?;
                }
                Ok(())
            }
            Pattern::Tuple { elements, .. } => {
                for elem in elements {
                    self.analyze_pattern_bindings(elem, &ZyraType::Unknown)?;
                }
                Ok(())
            }
            Pattern::Wildcard { .. } | Pattern::Literal { .. } => Ok(()),
        }
    }

    /// Check match exhaustiveness (conservative algorithm)
    fn check_exhaustiveness(
        arms: &[crate::parser::ast::MatchArm],
        scrutinee_type: &ZyraType,
        span: crate::lexer::Span,
    ) -> ZyraResult<()> {
        use crate::parser::ast::Pattern;

        // Check for wildcard or catch-all pattern (unconditional)
        let has_wildcard = arms.iter().any(|arm| {
            // Guards don't count for exhaustiveness
            if arm.guard.is_some() {
                return false;
            }
            matches!(
                &arm.pattern,
                Pattern::Wildcard { .. } | Pattern::Identifier { .. }
            )
        });

        if has_wildcard {
            return Ok(()); // Exhaustive via wildcard
        }

        // For enums, check if we have variant patterns
        // A more complete implementation would count variants against enum definition
        // For now, we allow enum matching if there are any variant patterns
        if let ZyraType::Enum(_enum_name) = scrutinee_type {
            let has_variant_patterns = arms
                .iter()
                .any(|arm| matches!(&arm.pattern, Pattern::Variant { .. }));

            if has_variant_patterns {
                // TODO: In the future, check all variants are covered
                // For now, allow variant patterns (assume programmer covers all cases)
                return Ok(());
            }

            // No variant patterns and no wildcard = definitely non-exhaustive
            return Err(ZyraError::type_error(
                "Non-exhaustive match: add a wildcard `_` pattern or cover all enum variants",
                Some(SourceLocation::new("", span.line, span.column)),
            ));
        }

        Ok(())
    }

    fn enter_scope(&mut self) {
        self.scope_depth += 1;
        self.scope_stack.enter();
        self.ownership.enter_scope();
        self.borrow_checker.enter_scope();
    }

    fn exit_scope(&mut self) {
        let exiting_scope = self.scope_stack.current();

        // Remove symbols from this scope
        self.symbols.retain(|_, s| s.scope_depth < self.scope_depth);

        // Remove references that were created in this scope
        self.references.retain(|_, r| r.use_scope != exiting_scope);

        // Exit scope in all checkers
        self.scope_stack.exit();
        self.ownership.exit_scope();
        self.borrow_checker.exit_scope();
        self.scope_depth -= 1;
    }

    fn ownership_error_to_zyra(&self, err: OwnershipError) -> ZyraError {
        ZyraError::ownership_error(&err.to_string(), None)
    }

    /// Check if a return expression contains a dangling reference
    /// Returns an error if returning a reference to a local variable
    fn check_return_expression(&self, expr: &Expression, _line: usize) -> ZyraResult<()> {
        match expr {
            // Direct reference expression: &x or &mut x
            Expression::Reference { value, span, .. } => {
                if let Expression::Identifier { name, .. } = value.as_ref() {
                    // Check if the referenced variable is local (cannot return)
                    if let Some(symbol) = self.symbols.get(name) {
                        match symbol.origin {
                            ValueOrigin::Local | ValueOrigin::Temporary => {
                                return Err(ZyraError::ownership_error(
                                    &format!(
                                        "cannot return reference to {} `{}` (declared at line {})\n\
                                         note: `{}` is a {} and will be dropped when the function returns",
                                        symbol.origin.display_name(),
                                        name,
                                        symbol.decl_line,
                                        name,
                                        symbol.origin.display_name()
                                    ),
                                    Some(SourceLocation::new("", span.line, span.column)),
                                ));
                            }
                            ValueOrigin::Param | ValueOrigin::Global => {
                                // OK - params and globals can be returned as references
                            }
                        }
                    }
                }
                Ok(())
            }
            // Identifier that might be a reference variable
            Expression::Identifier { name, span } => {
                // Check if this variable holds a reference to a local
                if let Some(ref_info) = self.references.get(name) {
                    let ref_info: &ReferenceInfo = ref_info;
                    if !ref_info.can_return() {
                        return Err(ZyraError::ownership_error(
                            &format!(
                                "cannot return reference to local variable `{}`\n\
                                 note: `{}` borrows from local variable `{}`",
                                ref_info.source_name, name, ref_info.source_name
                            ),
                            Some(SourceLocation::new("", span.line, span.column)),
                        ));
                    }
                }
                Ok(())
            }
            // If expression - check both branches
            Expression::If {
                then_block,
                else_block,
                span,
                ..
            } => {
                // Check trailing expression in then block
                if let Some(ref then_expr) = then_block.expression {
                    self.check_return_expression(then_expr, span.line)?;
                }
                // Check trailing expression in else block
                if let Some(ref else_blk) = else_block {
                    if let Some(ref else_expr) = else_blk.expression {
                        self.check_return_expression(else_expr, span.line)?;
                    }
                }
                Ok(())
            }
            // Other expressions are OK
            _ => Ok(()),
        }
    }

    /// Validate that all branches in a block return compatible types
    /// Returns all types from return statements and trailing expressions
    #[allow(dead_code)]
    fn collect_return_types(&self, body: &Block) -> Vec<ZyraType> {
        let mut types = Vec::new();

        for stmt in &body.statements {
            match stmt {
                Statement::Return { value, .. } => {
                    if let Some(expr) = value {
                        if let Ok(t) = self.infer_expression_type(expr) {
                            types.push(t);
                        }
                    } else {
                        types.push(ZyraType::Void);
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    // Recursively collect from branches
                    types.extend(self.collect_return_types(then_block));
                    if let Some(else_blk) = else_block {
                        types.extend(self.collect_return_types(else_blk));
                    }
                }
                _ => {}
            }
        }

        // Check trailing expression
        if let Some(ref expr) = body.expression {
            if let Ok(t) = self.infer_expression_type(expr) {
                types.push(t);
            }
        }

        types
    }

    /// Validate all branches are compatible with expected return type
    #[allow(dead_code)]
    pub fn validate_branch_returns(
        &self,
        body: &Block,
        expected: &ZyraType,
        line: usize,
    ) -> Result<(), ZyraError> {
        let branch_types = self.collect_return_types(body);

        for branch_type in branch_types {
            // For reference types, use strict compatibility
            if matches!(expected, ZyraType::Reference { .. }) {
                if !branch_type.is_compatible_strict(expected) {
                    return Err(ZyraError::type_error(
                        &format!(
                            "Branch return type {} is not strictly compatible with {}",
                            branch_type.display_name(),
                            expected.display_name()
                        ),
                        Some(SourceLocation::new("", line, 0)),
                    ));
                }
            } else if !branch_type.is_compatible(expected) {
                return Err(ZyraError::type_error(
                    &format!(
                        "Branch return type {} is not compatible with {}",
                        branch_type.display_name(),
                        expected.display_name()
                    ),
                    Some(SourceLocation::new("", line, 0)),
                ));
            }
        }

        Ok(())
    }

    /// Infer type of expression without full analysis (for branch validation)
    #[allow(dead_code)]
    fn infer_expression_type(&self, expr: &Expression) -> ZyraResult<ZyraType> {
        match expr {
            Expression::Int { .. } => Ok(ZyraType::I32),
            Expression::Float { .. } => Ok(ZyraType::F64),
            Expression::String { .. } => Ok(ZyraType::String),
            Expression::Bool { .. } => Ok(ZyraType::Bool),
            Expression::Char { .. } => Ok(ZyraType::Char),
            Expression::Identifier { name, .. } => {
                if let Some(sym) = self.symbols.get(name) {
                    Ok(sym.symbol_type.clone())
                } else {
                    Ok(ZyraType::Unknown)
                }
            }
            Expression::If {
                then_block,
                else_block,
                ..
            } => {
                // Return type is from then block
                if let Some(ref expr) = then_block.expression {
                    self.infer_expression_type(expr)
                } else if let Some(ref else_blk) = else_block {
                    if let Some(ref expr) = else_blk.expression {
                        self.infer_expression_type(expr)
                    } else {
                        Ok(ZyraType::Void)
                    }
                } else {
                    Ok(ZyraType::Void)
                }
            }
            _ => Ok(ZyraType::Unknown),
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
