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
                    ("width".to_string(), ZyraType::I64),
                    ("height".to_string(), ZyraType::I64),
                    ("title".to_string(), ZyraType::String),
                ],
                return_type: ZyraType::Object(HashMap::new()),
                lifetimes: vec![],
                has_mut_self: false,
            },
        );
    }

    /// Register functions from a specific std module
    fn register_std_module_functions(&mut self, module_name: &str) {
        let functions: Vec<(&str, Vec<(&str, ZyraType)>, ZyraType)> = match module_name {
            "std::math" => vec![
                ("abs", vec![("x", ZyraType::F64)], ZyraType::F64),
                ("sqrt", vec![("x", ZyraType::F64)], ZyraType::F64),
                (
                    "pow",
                    vec![("base", ZyraType::F64), ("exp", ZyraType::F64)],
                    ZyraType::F64,
                ),
                ("sin", vec![("x", ZyraType::F64)], ZyraType::F64),
                ("cos", vec![("x", ZyraType::F64)], ZyraType::F64),
                ("tan", vec![("x", ZyraType::F64)], ZyraType::F64),
                (
                    "min",
                    vec![("a", ZyraType::F64), ("b", ZyraType::F64)],
                    ZyraType::F64,
                ),
                (
                    "max",
                    vec![("a", ZyraType::F64), ("b", ZyraType::F64)],
                    ZyraType::F64,
                ),
                ("floor", vec![("x", ZyraType::F64)], ZyraType::I64),
                ("ceil", vec![("x", ZyraType::F64)], ZyraType::I64),
                ("round", vec![("x", ZyraType::F64)], ZyraType::I64),
                (
                    "random",
                    vec![("min", ZyraType::I64), ("max", ZyraType::I64)],
                    ZyraType::I64,
                ),
                (
                    "lerp",
                    vec![
                        ("a", ZyraType::F64),
                        ("b", ZyraType::F64),
                        ("t", ZyraType::F64),
                    ],
                    ZyraType::F64,
                ),
                (
                    "clamp",
                    vec![
                        ("x", ZyraType::F64),
                        ("min", ZyraType::F64),
                        ("max", ZyraType::F64),
                    ],
                    ZyraType::F64,
                ),
                ("pi", vec![], ZyraType::F64),
                ("e", vec![], ZyraType::F64),
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
                ("sleep", vec![("ms", ZyraType::I64)], ZyraType::Void),
                ("monotonic_ms", vec![], ZyraType::I64),
                ("instant_now", vec![], ZyraType::I64),
                (
                    "instant_elapsed",
                    vec![("id", ZyraType::I64)],
                    ZyraType::I64,
                ),
                ("delta_time", vec![], ZyraType::F64),
                ("fps", vec![], ZyraType::F64),
            ],
            "std::string" => vec![
                ("string_len", vec![("s", ZyraType::String)], ZyraType::I64),
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
                ("parse_int", vec![("s", ZyraType::String)], ZyraType::I64),
                ("parse_float", vec![("s", ZyraType::String)], ZyraType::F64),
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
                        ("width", ZyraType::I64),
                        ("height", ZyraType::I64),
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
                        ("x", ZyraType::I64),
                        ("y", ZyraType::I64),
                        ("w", ZyraType::I64),
                        ("h", ZyraType::I64),
                    ],
                    ZyraType::Void,
                ),
            ],
            _ => vec![],
        };

        for (name, params, return_type) in functions {
            let param_types: Vec<_> = params
                .into_iter()
                .map(|(n, t)| (n.to_string(), t))
                .collect();

            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    name: name.to_string(),
                    params: param_types,
                    return_type,
                    lifetimes: vec![],
                    has_mut_self: false,
                },
            );

            // Track that this function came from this module
            self.imported_std_items
                .insert(name.to_string(), module_name.to_string());
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
        for stmt in &program.statements {
            self.analyze_statement(stmt)?;
        }

        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
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

                // Check return type
                if let Some(ret) = return_type {
                    let expected = ZyraType::from_ast_type(ret);
                    if !body_type.is_compatible(&expected) {
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

                        // If specific items are imported, register them
                        if !items.is_empty() {
                            for item in items {
                                self.imported_std_items
                                    .insert(item.clone(), module_name.clone());
                            }
                        } else {
                            // Import all functions from this module
                            self.register_std_module_functions(&module_name);
                        }
                        Ok(ZyraType::Void)
                    }
                    "game" | "math" | "io" | "time" | "fs" | "env" | "process" | "thread"
                    | "mem" | "string" | "core" => {
                        // Legacy single-word modules - convert to std:: form
                        let module_name = format!("std::{}", root);
                        self.imported_std_modules.insert(module_name.clone());
                        self.register_std_module_functions(&module_name);
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
                        if !return_type.is_compatible(&sig.return_type) {
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
            Expression::Int { .. } => Ok(ZyraType::I32),
            Expression::Float { .. } => Ok(ZyraType::F32),
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
                            // Promote to float if either is float
                            if left_type.is_float() || right_type.is_float() {
                                Ok(ZyraType::F32)
                            } else {
                                Ok(ZyraType::I32)
                            }
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
                    ZyraType::String => Ok(ZyraType::String),
                    ZyraType::Unknown => Ok(ZyraType::Unknown),
                    _ => Err(ZyraError::type_error(
                        &format!("Cannot index {}", obj_type.display_name()),
                        Some(SourceLocation::new("", span.line, span.column)),
                    )),
                }
            }

            Expression::List { elements, .. } => {
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
        }
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
