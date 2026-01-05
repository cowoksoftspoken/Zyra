//! Zyra Lexer
//!
//! Converts source code into a stream of tokens for parsing.

pub mod token;

pub use token::{Span, Token, TokenKind};

use crate::error::{SourceLocation, ZyraError, ZyraResult};

/// Lexer for Zyra source code
pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    filename: String,

    // Position tracking
    pos: usize,
    line: usize,
    column: usize,
    start: usize,
    start_line: usize,
    start_column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, filename: &str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            filename: filename.to_string(),
            pos: 0,
            line: 1,
            column: 1,
            start: 0,
            start_line: 1,
            start_column: 1,
        }
    }

    /// Tokenize the entire source code
    pub fn tokenize(&mut self) -> ZyraResult<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace_and_comments();

            if self.is_at_end() {
                break;
            }

            self.start = self.pos;
            self.start_line = self.line;
            self.start_column = self.column;

            let token = self.scan_token()?;
            if token.kind != TokenKind::Newline {
                tokens.push(token);
            }
        }

        // Add EOF token
        tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.pos, self.pos, self.line, self.column),
            String::new(),
        ));

        Ok(tokens)
    }

    fn scan_token(&mut self) -> ZyraResult<Token> {
        let c = self.advance();

        let kind = match c {
            // Single character tokens
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            ':' => {
                if self.match_char(':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }
            ';' => TokenKind::Semicolon,
            '+' => {
                if self.match_char('=') {
                    TokenKind::PlusAssign
                } else {
                    TokenKind::Plus
                }
            }
            '*' => {
                if self.match_char('=') {
                    TokenKind::StarAssign
                } else {
                    TokenKind::Star
                }
            }
            '%' => {
                if self.match_char('=') {
                    TokenKind::PercentAssign
                } else {
                    TokenKind::Percent
                }
            }

            // Potentially multi-character tokens
            '-' => {
                if self.match_char('>') {
                    TokenKind::Arrow
                } else if self.match_char('=') {
                    TokenKind::MinusAssign
                } else {
                    TokenKind::Minus
                }
            }
            '/' => {
                if self.match_char('=') {
                    TokenKind::SlashAssign
                } else {
                    TokenKind::Slash
                }
            }
            '.' => {
                if self.match_char('.') {
                    if self.match_char('=') {
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            '=' => {
                if self.match_char('=') {
                    TokenKind::EqualEqual
                } else if self.match_char('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Equal
                }
            }
            '!' => {
                if self.match_char('=') {
                    TokenKind::NotEqual
                } else {
                    TokenKind::Not
                }
            }
            '<' => {
                if self.match_char('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if self.match_char('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '&' => {
                if self.match_char('&') {
                    TokenKind::And
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.match_char('|') {
                    TokenKind::Or
                } else {
                    TokenKind::Pipe // Single | for closure syntax
                }
            }

            // Lifetime or Char
            '\'' => self.scan_lifetime_or_char()?,

            // String literal
            '"' => self.scan_string()?,

            // Number
            '0'..='9' => self.scan_number(c)?,

            // Identifier or keyword
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier(c),

            // Newline
            '\n' => {
                self.line += 1;
                self.column = 1;
                TokenKind::Newline
            }

            _ => {
                return Err(self.error(&format!("Unexpected character '{}'", c)));
            }
        };

        let lexeme = self.source[self.start..self.pos].to_string();
        let span = Span::new(self.start, self.pos, self.start_line, self.start_column);

        Ok(Token::new(kind, span, lexeme))
    }

    fn scan_lifetime_or_char(&mut self) -> ZyraResult<TokenKind> {
        // We're at the char after initial ' because scan_token called advance() logic?
        // No, scan_token doesn't consume ' in match arm, it peeks.
        // Wait, current impl: match self.advance() { ... }.
        // No, scan_token calls self.advance() into 'c'.
        // So ' is consumed.

        // Scan char/lifetime content
        let first_char = if self.peek() == '\\' {
            self.advance(); // consume \
            match self.advance() {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                '0' => '\0',
                c => return Err(self.error(&format!("Invalid escape sequence '\\{}'", c))),
            }
        } else {
            self.advance()
        };

        // Check if it's a char literal (ends with ')
        if self.peek() == '\'' {
            self.advance(); // consume closing '
            return Ok(TokenKind::Char(first_char));
        }

        // If not char literal, must be lifetime
        // Lifetimes cannot start with space (unless ' ' which is caught above if closed)
        // Check if first_char is valid start of identifier
        if !first_char.is_alphabetic() && first_char != '_' {
            // If we are here, it means we scanned something like '1... or ' ... and no closing quote.
            // This is invalid syntax for both char and lifetime.
            return Err(
                self.error("Expected closing ' for character literal or valid lifetime name")
            );
        }

        let mut name = String::from(first_char);
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            name.push(self.advance());
        }
        Ok(TokenKind::Lifetime(name))
    }

    fn scan_string(&mut self) -> ZyraResult<TokenKind> {
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut has_interpolation = false;

        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }

            // Check for interpolation start ${
            if self.peek() == '$' && self.peek_next() == '{' {
                has_interpolation = true;

                // Push accumulated literal part
                if !current_literal.is_empty() {
                    parts.push((false, current_literal.clone()));
                    current_literal.clear();
                }

                self.advance(); // Consume $
                self.advance(); // Consume {

                // Capture expression content
                let mut expr = String::new();
                let mut brace_depth = 1;

                while !self.is_at_end() && brace_depth > 0 {
                    let c = self.peek();

                    if c == '{' {
                        brace_depth += 1;
                        expr.push(self.advance());
                    } else if c == '}' {
                        brace_depth -= 1;
                        if brace_depth > 0 {
                            expr.push(self.advance());
                        }
                    } else if c == '"' {
                        // Handle string inside expression to avoid confusing braces
                        expr.push(self.advance()); // open quote
                        while !self.is_at_end() && self.peek() != '"' {
                            if self.peek() == '\\' {
                                expr.push(self.advance()); // \
                                if !self.is_at_end() {
                                    expr.push(self.advance());
                                } // escaped char
                            } else {
                                expr.push(self.advance());
                            }
                        }
                        if !self.is_at_end() {
                            expr.push(self.advance()); // close quote
                        }
                    } else {
                        // Normal char
                        expr.push(self.advance());
                    }
                }

                if self.is_at_end() && brace_depth > 0 {
                    return Err(self.error("Unterminated string interpolation"));
                }

                self.advance(); // Consume closing }
                parts.push((true, expr));
                continue;
            }

            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => {
                        self.advance();
                        current_literal.push('\n');
                    }
                    't' => {
                        self.advance();
                        current_literal.push('\t');
                    }
                    'r' => {
                        self.advance();
                        current_literal.push('\r');
                    }
                    '\\' => {
                        self.advance();
                        current_literal.push('\\');
                    }
                    '"' => {
                        self.advance();
                        current_literal.push('"');
                    }
                    '$' => {
                        // Escape $ sign
                        self.advance();
                        current_literal.push('$');
                    }
                    c => {
                        return Err(self.error(&format!("Invalid escape sequence '\\{}'", c)));
                    }
                }
            } else {
                current_literal.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(self.error("Unterminated string literal"));
        }

        self.advance(); // Consume closing quote

        if has_interpolation {
            if !current_literal.is_empty() {
                parts.push((false, current_literal));
            }
            Ok(TokenKind::FormatString(parts))
        } else {
            Ok(TokenKind::String(current_literal))
        }
    }

    fn scan_number(&mut self, first: char) -> ZyraResult<TokenKind> {
        let mut num_str = String::from(first);
        let mut is_float = false;

        while self.peek().is_ascii_digit() {
            num_str.push(self.advance());
        }

        // Check for decimal part
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            is_float = true;
            num_str.push(self.advance()); // Consume '.'

            while self.peek().is_ascii_digit() {
                num_str.push(self.advance());
            }
        }

        if is_float {
            let value: f64 = num_str
                .parse()
                .map_err(|_| self.error(&format!("Invalid float literal '{}'", num_str)))?;
            Ok(TokenKind::Float(value))
        } else {
            let value: i64 = num_str
                .parse()
                .map_err(|_| self.error(&format!("Invalid integer literal '{}'", num_str)))?;
            Ok(TokenKind::Int(value))
        }
    }

    fn scan_identifier(&mut self, first: char) -> TokenKind {
        let mut name = String::from(first);

        while self.peek().is_alphanumeric() || self.peek() == '_' {
            name.push(self.advance());
        }

        // Check if it's a keyword
        TokenKind::keyword_from_str(&name).unwrap_or(TokenKind::Identifier(name))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    // Don't skip newlines - they might be significant
                    break;
                }
                '/' => {
                    if self.peek_next() == '/' {
                        // Single-line comment
                        while !self.is_at_end() && self.peek() != '\n' {
                            self.advance();
                        }
                    } else if self.peek_next() == '*' {
                        // Multi-line comment
                        self.advance(); // Consume /
                        self.advance(); // Consume *

                        let mut depth = 1;
                        while !self.is_at_end() && depth > 0 {
                            if self.peek() == '/' && self.peek_next() == '*' {
                                self.advance();
                                self.advance();
                                depth += 1;
                            } else if self.peek() == '*' && self.peek_next() == '/' {
                                self.advance();
                                self.advance();
                                depth -= 1;
                            } else {
                                if self.peek() == '\n' {
                                    self.line += 1;
                                    self.column = 0;
                                }
                                self.advance();
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    // Helper methods
    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.chars[self.pos]
        }
    }

    fn peek_next(&self) -> char {
        if self.pos + 1 >= self.chars.len() {
            '\0'
        } else {
            self.chars[self.pos + 1]
        }
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.pos];
        self.pos += 1;
        self.column += 1;
        c
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.pos += 1;
            self.column += 1;
            true
        }
    }

    fn error(&self, message: &str) -> ZyraError {
        let line_start = self.source[..self.start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_end = self.source[self.start..]
            .find('\n')
            .map(|i| self.start + i)
            .unwrap_or(self.source.len());
        let snippet = &self.source[line_start..line_end];

        ZyraError::syntax_error(
            message,
            SourceLocation::new(&self.filename, self.start_line, self.start_column)
                .with_snippet(snippet),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("let x = 5;", "test.zr");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Equal));
        assert!(matches!(tokens[3].kind, TokenKind::Int(5)));
        assert!(matches!(tokens[4].kind, TokenKind::Semicolon));
    }

    #[test]
    fn test_string_literal() {
        let mut lexer = Lexer::new("\"hello world\"", "test.zr");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "hello world"));
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / == != < > <= >= && ||", "test.zr");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::EqualEqual));
        assert!(matches!(tokens[5].kind, TokenKind::NotEqual));
    }

    #[test]
    fn test_lifetime() {
        let mut lexer = Lexer::new("'a 'b 'lifetime", "test.zr");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(&tokens[0].kind, TokenKind::Lifetime(s) if s == "a"));
        assert!(matches!(&tokens[1].kind, TokenKind::Lifetime(s) if s == "b"));
        assert!(matches!(&tokens[2].kind, TokenKind::Lifetime(s) if s == "lifetime"));
    }
}
