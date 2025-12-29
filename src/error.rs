//! Zyra Error Handling
//! 
//! Provides human-readable error messages with source locations and suggestions.

use std::fmt;

/// Source location for error reporting
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub snippet: Option<String>,
}

impl SourceLocation {
    pub fn new(file: &str, line: usize, column: usize) -> Self {
        Self {
            file: file.to_string(),
            line,
            column,
            snippet: None,
        }
    }
    
    pub fn with_snippet(mut self, snippet: &str) -> Self {
        self.snippet = Some(snippet.to_string());
        self
    }
}

/// Main error type for Zyra
#[derive(Debug, Clone)]
pub struct ZyraError {
    pub kind: String,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub suggestion: Option<String>,
}

impl ZyraError {
    pub fn new(kind: &str, message: &str, location: Option<SourceLocation>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.to_string(),
            location,
            suggestion: None,
        }
    }
    
    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }
    
    // Common error constructors
    pub fn syntax_error(message: &str, location: SourceLocation) -> Self {
        Self::new("SyntaxError", message, Some(location))
    }
    
    pub fn type_error(message: &str, location: Option<SourceLocation>) -> Self {
        Self::new("TypeError", message, location)
    }
    
    pub fn name_error(message: &str, location: Option<SourceLocation>) -> Self {
        Self::new("NameError", message, location)
    }
    
    pub fn ownership_error(message: &str, location: Option<SourceLocation>) -> Self {
        Self::new("OwnershipError", message, location)
    }
    
    pub fn runtime_error(message: &str) -> Self {
        Self::new("RuntimeError", message, None)
    }
}

impl fmt::Display for ZyraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Error header
        writeln!(f, "\x1b[1;31merror[{}]\x1b[0m: {}", self.kind, self.message)?;
        
        // Location info
        if let Some(ref loc) = self.location {
            writeln!(f, "  \x1b[1;34m-->\x1b[0m {}:{}:{}", loc.file, loc.line, loc.column)?;
            
            // Code snippet
            if let Some(ref snippet) = loc.snippet {
                writeln!(f, "   \x1b[1;34m|\x1b[0m")?;
                writeln!(f, " \x1b[1;34m{:3} |\x1b[0m {}", loc.line, snippet)?;
                
                // Underline the error position
                let padding = " ".repeat(loc.column + 4);
                writeln!(f, "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m^\x1b[0m", padding)?;
            }
        }
        
        // Suggestion
        if let Some(ref suggestion) = self.suggestion {
            writeln!(f)?;
            writeln!(f, "\x1b[1;32mhelp\x1b[0m: {}", suggestion)?;
        }
        
        Ok(())
    }
}

impl std::error::Error for ZyraError {}

/// Result type alias for Zyra operations
pub type ZyraResult<T> = Result<T, ZyraError>;
