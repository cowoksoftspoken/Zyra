//! Module Resolver for Zyra
//!
//! Resolves import paths (e.g., `src::ball`) to actual .zr file paths
//! and loads/parses their content.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ZyraError, ZyraResult};
use crate::lexer::Lexer;
use crate::parser::ast::{Program, Statement};
use crate::parser::Parser;

/// Module resolver for loading .zr files
pub struct ModuleResolver {
    /// Base directory for resolving imports
    base_dir: PathBuf,
    /// Cache of already loaded modules
    loaded_modules: HashMap<String, Program>,
}

impl ModuleResolver {
    /// Create a new resolver with the given base directory
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            loaded_modules: HashMap::new(),
        }
    }

    /// Resolve an import path to a file path
    /// Example: ["src", "ball"] -> "src/ball.zr"
    pub fn resolve_path(&self, import_path: &[String]) -> PathBuf {
        let mut path = self.base_dir.clone();
        for segment in import_path {
            path = path.join(segment);
        }
        path.with_extension("zr")
    }

    /// Check if a path is a stdlib import (starts with "std")
    pub fn is_stdlib_import(import_path: &[String]) -> bool {
        import_path.first().map(|s| s == "std").unwrap_or(false)
    }

    /// Load a module from an import path
    pub fn load_module(&mut self, import_path: &[String]) -> ZyraResult<Option<Program>> {
        // Skip stdlib imports - they're handled by the VM
        if Self::is_stdlib_import(import_path) {
            return Ok(None);
        }

        // Create module key
        let module_key = import_path.join("::");

        // Check cache
        if let Some(program) = self.loaded_modules.get(&module_key) {
            return Ok(Some(program.clone()));
        }

        // Resolve to file path
        let file_path = self.resolve_path(import_path);

        // Check if file exists
        if !file_path.exists() {
            return Err(ZyraError::new(
                "ImportError",
                &format!(
                    "Module not found: '{}' (looking for {:?})",
                    module_key, file_path
                ),
                None,
            ));
        }

        // Read file
        let source = fs::read_to_string(&file_path).map_err(|e| {
            ZyraError::new(
                "ImportError",
                &format!("Could not read module '{}': {}", module_key, e),
                None,
            )
        })?;

        // Parse the module
        let file_str = file_path.to_string_lossy().to_string();
        let mut lexer = Lexer::new(&source, &file_str);
        let tokens = lexer.tokenize()?;

        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        // Cache the module
        self.loaded_modules.insert(module_key, program.clone());

        Ok(Some(program))
    }

    /// Resolve all imports in a program and merge their statements
    pub fn resolve_imports(&mut self, program: &mut Program) -> ZyraResult<()> {
        let mut imported_statements: Vec<Statement> = Vec::new();

        // Process each import statement
        for stmt in &program.statements {
            if let Statement::Import {
                path,
                items: _,
                span,
            } = stmt
            {
                // Skip stdlib imports
                if Self::is_stdlib_import(path) {
                    continue;
                }

                // Get module name (last part of path)
                let module_name = path.last().cloned().unwrap_or_default();

                // Prevent importing main.zr
                if module_name == "main" {
                    return Err(ZyraError::new(
                        "ImportError",
                        "Cannot import 'main' - it is the entry point and cannot be imported",
                        Some(crate::error::SourceLocation::new(
                            "",
                            span.line,
                            span.column,
                        )),
                    ));
                }

                // Load the module
                if let Some(module_program) = self.load_module(path)? {
                    // Add statements from the module
                    for mut module_stmt in module_program.statements {
                        match &module_stmt {
                            // Keep stdlib imports so semantic analyzer sees them
                            Statement::Import {
                                path: import_path, ..
                            } => {
                                if Self::is_stdlib_import(import_path) {
                                    imported_statements.push(module_stmt);
                                }
                                // Skip local module imports (they would have been resolved separately)
                            }
                            _ => {
                                // Add namespace prefix to function and struct names
                                Self::add_namespace_prefix(&module_name, &mut module_stmt);
                                imported_statements.push(module_stmt);
                            }
                        }
                    }
                }
            }
        }

        // Prepend imported statements to the program
        // (They need to come before the code that uses them)
        let original_statements = std::mem::take(&mut program.statements);
        program.statements = imported_statements;
        program.statements.extend(original_statements);

        Ok(())
    }

    /// Add namespace prefix to function and struct names
    fn add_namespace_prefix(module_name: &str, stmt: &mut Statement) {
        match stmt {
            Statement::Function { name, .. } => {
                *name = format!("{}::{}", module_name, name);
            }
            Statement::Struct { name, .. } => {
                *name = format!("{}::{}", module_name, name);
            }
            Statement::Enum { name, .. } => {
                *name = format!("{}::{}", module_name, name);
            }
            _ => {}
        }
    }
}
