//! Zyra CLI - Command Line Interface
//!
//! Usage:
//!   zyra run <file.zr>     - Run a Zyra program
//!   zyra check <file.zr>   - Check syntax and types without running
//!   zyra compile <file.zr> - Compile to bytecode
//!   zyra build <file.zr>   - Alias for compile

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use colored::Colorize;
use zyra::compiler::Compiler;
use zyra::error::ZyraError;
use zyra::lexer::Lexer;
use zyra::parser::Parser;
use zyra::resolver::ModuleResolver;
use zyra::semantic::SemanticAnalyzer;
use zyra::vm::VM;

/// Project configuration from zyra.toml
struct ProjectConfig {
    main: Option<String>,
    output: Option<String>,
}

/// Configuration validation result
enum ConfigResult {
    Valid(ProjectConfig),
    InvalidMainEntry(String),
    NoConfig,
}

/// Find zyra.toml - first check source file's directory, then current directory
fn find_project_config_for_file(file_path: Option<&str>) -> ConfigResult {
    // First try the source file's directory
    if let Some(path) = file_path {
        let source_path = Path::new(path);
        if let Some(parent) = source_path.parent() {
            let toml_in_source_dir = parent.join("zyra.toml");
            if toml_in_source_dir.exists() {
                return parse_project_config(&toml_in_source_dir);
            }
        }
    }

    // Fall back to current directory
    let toml_path = Path::new("zyra.toml");
    if toml_path.exists() {
        return parse_project_config(toml_path);
    }

    ConfigResult::NoConfig
}

/// Parse zyra.toml from given path
fn parse_project_config(toml_path: &Path) -> ConfigResult {
    let content = match fs::read_to_string(toml_path) {
        Ok(c) => c,
        Err(_) => return ConfigResult::NoConfig,
    };

    // Simple TOML parsing for main and output
    let mut main: Option<String> = None;
    let mut output: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        // Parse "main = ..." in [build] section
        if line.starts_with("main") && !line.starts_with("main_entry") {
            if let Some(value) = line.split('=').nth(1) {
                let val = value.trim().trim_matches('"').to_string();
                if !val.is_empty() {
                    main = Some(val);
                }
            }
        }
        // Also support legacy main_entry for backwards compatibility
        if line.starts_with("main_entry") {
            if let Some(value) = line.split('=').nth(1) {
                let val = value.trim().trim_matches('"').to_string();
                if !val.is_empty() && main.is_none() {
                    main = Some(val);
                }
            }
        }
        if line.starts_with("output") {
            if let Some(value) = line.split('=').nth(1) {
                let val = value.trim().trim_matches('"').to_string();
                if !val.is_empty() {
                    output = Some(val);
                }
            }
        }
    }

    // Validate main has valid extension if present
    if let Some(ref entry) = main {
        if !entry.ends_with(".zr") && !entry.ends_with(".zy") && !entry.ends_with(".za") {
            return ConfigResult::InvalidMainEntry(entry.clone());
        }
    }

    ConfigResult::Valid(ProjectConfig { main, output })
}

/// Get the main entry file, either from arg or zyra.toml
/// If zyra.toml exists, main must be specified even when running with explicit file
fn get_main_entry(args: &[String], arg_index: usize) -> Option<String> {
    let explicit_file = if args.len() > arg_index {
        Some(args[arg_index].clone())
    } else {
        None
    };

    // Check for zyra.toml - if it exists, main must be configured
    // Also check in the explicit file's directory
    match find_project_config_for_file(explicit_file.as_deref()) {
        ConfigResult::Valid(config) => {
            if config.main.is_none() {
                eprintln!(
                    "{}: main is not specified in zyra.toml",
                    "ConfigError".red()
                );
                eprintln!("  Add [build] section with main = \"main.zr\"");
                return None;
            }

            // If explicit file provided, use it; otherwise use config.main
            if let Some(file) = explicit_file {
                return Some(file);
            }

            let entry = config.main.unwrap();
            let entry_path = Path::new(&entry);
            if entry_path.exists() {
                return Some(entry);
            } else {
                eprintln!("{}: main '{}' not found", "ConfigError".red(), entry);
                eprintln!("  The file specified in zyra.toml does not exist.");
                return None;
            }
        }
        ConfigResult::InvalidMainEntry(entry) => {
            eprintln!(
                "{}: main '{}' has invalid extension",
                "ConfigError".red(),
                entry
            );
            eprintln!("  main must end with .zr, .zy, or .za");
            return None;
        }
        ConfigResult::NoConfig => {
            // No zyra.toml found - allow running explicit file
            if let Some(file) = explicit_file {
                return Some(file);
            }
            return None;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "run" => match get_main_entry(&args, 2) {
            Some(file) => run_file(&file),
            None => {
                eprintln!(
                    "{}",
                    "Error: No file specified and no zyra.toml found".red()
                );
                eprintln!("Usage: zyra run <file.zr>");
                eprintln!("  Or create a project with: zyra init <name>");
                process::exit(1);
            }
        },
        "check" => match get_main_entry(&args, 2) {
            Some(file) => check_file(&file),
            None => {
                eprintln!(
                    "{}",
                    "Error: No file specified and no zyra.toml found".red()
                );
                eprintln!("Usage: zyra check <file.zr>");
                process::exit(1);
            }
        },
        "build" | "compile" => match get_main_entry(&args, 2) {
            Some(file) => build_file(&file),
            None => {
                eprintln!(
                    "{}",
                    "Error: No file specified and no zyra.toml found".red()
                );
                eprintln!("Usage: zyra compile <file.zr>");
                process::exit(1);
            }
        },
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-v" => {
            println!("{}", "Zyra Programming Language v1.0.2".cyan().bold());
        }
        "init" => {
            let name = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            if let Err(e) = init_project(name) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        _ => {
            // Check if it's a file path (for convenience: `zyra file.zr`)
            if is_zyra_file(command) {
                run_file(command);
            } else {
                eprintln!("{}: Unknown command '{}'", "Error".red(), command);
                print_usage();
                process::exit(1);
            }
        }
    }
}

fn print_usage() {
    println!("{}", "Zyra Programming Language v1.0.2".cyan().bold());
    println!();
    println!("{}", "Usage:".yellow().bold());
    println!(
        "  {} {}     Run a Zyra program",
        "zyra run".green(),
        "<file>".white()
    );
    println!(
        "  {} {}   Check syntax and types",
        "zyra check".green(),
        "<file>".white()
    );
    println!(
        "  {} {} Compile to bytecode",
        "zyra compile".green(),
        "<file>".white()
    );
    println!(
        "  {} {}     Alias for compile",
        "zyra build".green(),
        "<file>".white()
    );
    println!("  {}           Show this help", "zyra help".green());
    println!("  {}        Show version", "zyra version".green());
    println!(
        "  {} {}  Initialize new project",
        "zyra init".green(),
        "<name>".white()
    );
    println!();
    println!("Supported file extensions: {}", ".zr, .zy, .za".cyan());
}

fn is_zyra_file(path: &str) -> bool {
    let path = Path::new(path);
    match path.extension() {
        Some(ext) => {
            let ext = ext.to_string_lossy().to_lowercase();
            ext == "zr" || ext == "zy" || ext == "za"
        }
        None => false,
    }
}

fn validate_file_extension(path: &str) -> Result<(), ZyraError> {
    if !is_zyra_file(path) {
        return Err(ZyraError::new(
            "InvalidExtension",
            &format!(
                "File '{}' does not have a valid Zyra extension.\n\
                 Supported extensions: .zr, .zy, .za",
                path
            ),
            None,
        ));
    }
    Ok(())
}

fn read_source_file(path: &str) -> Result<String, ZyraError> {
    validate_file_extension(path)?;

    fs::read_to_string(path).map_err(|e| {
        ZyraError::new(
            "FileError",
            &format!("Could not read file '{}': {}", path, e),
            None,
        )
    })
}

fn run_file(path: &str) {
    match run_file_internal(path) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn run_file_internal(path: &str) -> Result<(), ZyraError> {
    // Check if it's a compiled bytecode file
    if path.ends_with(".zyc") {
        return run_bytecode_file(path);
    }

    let source = read_source_file(path)?;

    // Lexical analysis
    let mut lexer = Lexer::new(&source, path);
    let tokens = lexer.tokenize()?;

    // Parsing
    // Parsing
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse()?;

    // Module Resolution
    let file_path = Path::new(path);
    let base_dir = file_path.parent().unwrap_or_else(|| Path::new("."));
    let mut resolver = ModuleResolver::new(base_dir);
    resolver.resolve_imports(&mut ast)?;

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;

    // Compilation
    let mut compiler = Compiler::new();
    let bytecode = compiler.compile(&ast)?;

    // Execution
    let mut vm = VM::new();
    vm.run(&bytecode)?;

    Ok(())
}

/// Run a pre-compiled bytecode file
fn run_bytecode_file(path: &str) -> Result<(), ZyraError> {
    use zyra::compiler::bytecode::Bytecode;

    // Read bytecode file
    let data = fs::read(path).map_err(|e| {
        ZyraError::new(
            "FileError",
            &format!("Could not read bytecode file '{}': {}", path, e),
            None,
        )
    })?;

    // Deserialize bytecode
    let bytecode = Bytecode::deserialize(&data)
        .map_err(|e| ZyraError::new("BytecodeError", e.as_str(), None))?;

    // Execute
    let mut vm = VM::new();
    vm.run(&bytecode)?;

    Ok(())
}

fn check_file(path: &str) {
    match check_file_internal(path) {
        Ok(summary) => {
            println!("{}", "═══════════════════════════════════════════".green());
            println!("{}", format!("✓ Check passed: '{}'", path).green().bold());
            println!("{}", "═══════════════════════════════════════════".green());
            println!();
            println!("{}", "Check Summary:".cyan().bold());
            println!("  ✓ Lexical analysis      - {} tokens", summary.token_count);
            println!(
                "  ✓ Syntax parsing        - {} statements",
                summary.statement_count
            );
            println!("  ✓ Semantic analysis     - types validated");
            println!("  ✓ Ownership checking    - moves tracked");
            println!("  ✓ Borrow checking       - references safe");
            println!("  ✓ Lifetime checking     - no dangling refs");
            println!();
            println!("{}", "No errors found!".green().bold());
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

struct CheckSummary {
    token_count: usize,
    statement_count: usize,
}

fn check_file_internal(path: &str) -> Result<CheckSummary, ZyraError> {
    let source = read_source_file(path)?;

    // Lexical analysis
    let mut lexer = Lexer::new(&source, path);
    let tokens = lexer.tokenize()?;
    let token_count = tokens.len();

    // Parsing
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    let statement_count = ast.statements.len();

    // Semantic analysis (includes ownership, borrow, and lifetime checking)
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;

    Ok(CheckSummary {
        token_count,
        statement_count,
    })
}

fn build_file(path: &str) {
    match build_file_internal(path) {
        Ok(output_path) => {
            println!("✓ Compiled '{}' to '{}'", path, output_path);
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn build_file_internal(path: &str) -> Result<String, ZyraError> {
    let source = read_source_file(path)?;

    // Lexical analysis
    let mut lexer = Lexer::new(&source, path);
    let tokens = lexer.tokenize()?;

    // Parsing
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse()?;

    // Module Resolution - merge imported modules
    let file_path = Path::new(path);
    let base_dir = file_path.parent().unwrap_or_else(|| Path::new("."));
    let mut resolver = ModuleResolver::new(base_dir);
    resolver.resolve_imports(&mut ast)?;

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;

    // Compilation
    let mut compiler = Compiler::new();
    let bytecode = compiler.compile(&ast)?;

    // Write bytecode to file
    let mut output_path = Path::new(path).with_extension("zyc");

    // Check for project config output directory
    if let ConfigResult::Valid(config) = find_project_config_for_file(Some(path)) {
        if let Some(ref output_dir) = config.output {
            let out_dir = Path::new(output_dir);
            if out_dir != Path::new("./") && out_dir != Path::new(".") {
                // Create output directory if it doesn't exist
                if !out_dir.exists() {
                    fs::create_dir_all(out_dir).map_err(|e| {
                        ZyraError::new(
                            "FileError",
                            &format!("Could not create output directory '{}': {}", output_dir, e),
                            None,
                        )
                    })?;
                }

                // Construct new path: output_dir + filename.zyc
                if let Some(filename) = output_path.file_name() {
                    output_path = out_dir.join(filename);
                }
            }
        } else {
            // Output not specified in zyra.toml
            return Err(ZyraError::new(
                "ConfigError",
                "output is not specified in zyra.toml [build] section",
                None,
            ));
        }
    }

    let output_str = output_path.to_string_lossy().to_string();

    // Serialize bytecode (simple binary format)
    let serialized = bytecode.serialize();
    fs::write(&output_path, serialized).map_err(|e| {
        ZyraError::new(
            "FileError",
            &format!("Could not write output file '{}': {}", output_str, e),
            None,
        )
    })?;

    Ok(output_str)
}

/// Initialize a new Zyra project
fn init_project(name: &str) -> Result<(), ZyraError> {
    use std::path::PathBuf;

    let project_dir = if name == "." {
        std::env::current_dir().map_err(|e| {
            ZyraError::new(
                "InitError",
                &format!("Cannot get current directory: {}", e),
                None,
            )
        })?
    } else {
        PathBuf::from(name)
    };

    // Extract project name from the path (just the last component)
    let project_name = project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "zyra_project".to_string());

    // Check if zyra.toml already exists (re-initialization)
    let toml_file = project_dir.join("zyra.toml");
    let is_reinit = toml_file.exists();

    // Create project directory if it doesn't exist
    if name != "." && !project_dir.exists() {
        fs::create_dir_all(&project_dir).map_err(|e| {
            ZyraError::new(
                "InitError",
                &format!("Cannot create directory '{}': {}", name, e),
                None,
            )
        })?;
    }

    // Only create src directory if not re-initializing
    let src_dir = project_dir.join("src");
    if !is_reinit && !src_dir.exists() {
        fs::create_dir_all(&src_dir).map_err(|e| {
            ZyraError::new(
                "InitError",
                &format!("Cannot create src directory: {}", e),
                None,
            )
        })?;
    }

    // Only create main.zr if not re-initializing and it doesn't exist
    let main_file = project_dir.join("main.zr");
    if !is_reinit && !main_file.exists() {
        let main_content = format!(
            r#"// {} - Main Entry Point
//
// Run with: zyra run

func main() {{
    println("Hello, {}!");
}}
"#,
            project_name, project_name
        );

        fs::write(&main_file, main_content).map_err(|e| {
            ZyraError::new("InitError", &format!("Cannot create main.zr: {}", e), None)
        })?;
    }

    // Always create/update zyra.toml
    let toml_content = format!(
        r#"[project]
name = "{}"
version = "0.1.0"
edition = "2025"
zyra = ">=1.0.2"
description = "-"
authors = "-"
license = ["MIT"]

[dependencies]
# Add dependencies here
# not supported yet

[build]
main = "main.zr"
output = "./"
"#,
        project_name
    );

    fs::write(&toml_file, toml_content).map_err(|e| {
        ZyraError::new(
            "InitError",
            &format!("Cannot create zyra.toml: {}", e),
            None,
        )
    })?;

    // Success message
    if is_reinit {
        println!("{}", "✓ Zyra project reinitialized!".green().bold());
        println!();
        println!("  Updated:");
        println!("    {} - project configuration", "zyra.toml".cyan());
        println!();
        println!("  Note: Existing files were not modified.");
    } else {
        println!("{}", "✓ Zyra project initialized!".green().bold());
        println!();
        println!("  Created:");
        println!("    {} - project entry point", "main.zr".cyan());
        println!("    {} - project configuration", "zyra.toml".cyan());
        println!("    {} - source directory", "src/".cyan());
        println!();
        println!("  Get started:");
        println!("    {}", "zyra run".green());
    }

    Ok(())
}
