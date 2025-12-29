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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("{}", "Error: Missing file argument".red());
                eprintln!("Usage: zyra run <file.zr>");
                process::exit(1);
            }
            run_file(&args[2]);
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("{}", "Error: Missing file argument".red());
                eprintln!("Usage: zyra check <file.zr>");
                process::exit(1);
            }
            check_file(&args[2]);
        }
        "build" | "compile" => {
            if args.len() < 3 {
                eprintln!("{}", "Error: Missing file argument".red());
                eprintln!("Usage: zyra compile <file.zr>");
                process::exit(1);
            }
            build_file(&args[2]);
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-v" => {
            println!("{}", "Zyra Programming Language v0.1.0".cyan().bold());
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
    println!("{}", "Zyra Programming Language v0.1.0".cyan().bold());
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
    let ast = parser.parse()?;

    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;

    // Compilation
    let mut compiler = Compiler::new();
    let bytecode = compiler.compile(&ast)?;

    // Write bytecode to file
    let output_path = Path::new(path).with_extension("zyc");
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

[dependencies]
# Add dependencies here

[build]
entry = "main.zr"
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
