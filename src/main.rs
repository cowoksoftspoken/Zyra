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
    main_entry: String,
    output: String,
}

/// Find zyra.toml in current directory and parse it
fn find_project_config() -> Option<ProjectConfig> {
    let toml_path = Path::new("zyra.toml");
    if !toml_path.exists() {
        return None;
    }

    let content = fs::read_to_string(toml_path).ok()?;

    // Simple TOML parsing for main_entry and output
    let mut main_entry = String::from("main.zr");
    let mut output = String::from("./");

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("main_entry") {
            if let Some(value) = line.split('=').nth(1) {
                main_entry = value.trim().trim_matches('"').to_string();
            }
        }
        if line.starts_with("output") {
            if let Some(value) = line.split('=').nth(1) {
                output = value.trim().trim_matches('"').to_string();
            }
        }
    }

    // Validate main_entry has valid extension
    if !main_entry.ends_with(".zr") && !main_entry.ends_with(".zy") && !main_entry.ends_with(".za")
    {
        eprintln!(
            "{}: main_entry must be main.zr, main.zy, or main.za",
            "ConfigError".red()
        );
        return None;
    }

    Some(ProjectConfig { main_entry, output })
}

/// Get the main entry file, either from arg or zyra.toml
fn get_main_entry(args: &[String], arg_index: usize) -> Option<String> {
    // If file argument provided, use it
    if args.len() > arg_index {
        return Some(args[arg_index].clone());
    }

    // Otherwise check for zyra.toml
    if let Some(config) = find_project_config() {
        let entry_path = Path::new(&config.main_entry);
        if entry_path.exists() {
            return Some(config.main_entry);
        } else {
            eprintln!(
                "{}: main_entry '{}' not found",
                "ConfigError".red(),
                config.main_entry
            );
            return None;
        }
    }

    None
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
    if let Some(config) = find_project_config() {
        let out_dir = Path::new(&config.output);
        if out_dir != Path::new("./") && out_dir != Path::new(".") {
            // Create output directory if it doesn't exist
            if !out_dir.exists() {
                fs::create_dir_all(out_dir).map_err(|e| {
                    ZyraError::new(
                        "FileError",
                        &format!(
                            "Could not create output directory '{}': {}",
                            config.output, e
                        ),
                        None,
                    )
                })?;
            }

            // Construct new path: output_dir + filename.zyc
            if let Some(filename) = output_path.file_name() {
                output_path = out_dir.join(filename);
            }
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

    let project_name = if name == "." {
        project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "zyra_project".to_string())
    } else {
        name.to_string()
    };

    // Create project directory
    if name != "." {
        fs::create_dir_all(&project_dir).map_err(|e| {
            ZyraError::new(
                "InitError",
                &format!("Cannot create directory '{}': {}", name, e),
                None,
            )
        })?;
    }

    // Create src directory
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| {
        ZyraError::new(
            "InitError",
            &format!("Cannot create src directory: {}", e),
            None,
        )
    })?;

    // Create main.zr
    let main_file = project_dir.join("main.zr");
    let main_content = format!(
        r#"// {} - Main Entry Point
//
// Run with: zyra run main.zr

func main() {{
    println("Hello, {}!");
}}
"#,
        project_name, project_name
    );

    fs::write(&main_file, main_content)
        .map_err(|e| ZyraError::new("InitError", &format!("Cannot create main.zr: {}", e), None))?;

    // Create zyra.toml
    let toml_file = project_dir.join("zyra.toml");
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
main_entry = "main.zr"
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
    println!("{}", "✓ Zyra project initialized!".green().bold());
    println!();
    println!("  Created:");
    println!("    {} - project entry point", "main.zr".cyan());
    println!("    {} - project configuration", "zyra.toml".cyan());
    println!("    {} - source directory", "src/".cyan());
    println!();
    println!("  Get started:");
    println!("    {} {}", "zyra run".green(), "main.zr");

    Ok(())
}
