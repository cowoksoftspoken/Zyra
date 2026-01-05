//! Token definitions for Zyra lexer

use std::fmt;

/// Position in source code
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

/// Token types for Zyra
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),

    // Identifiers and Keywords
    Identifier(String),

    // Keywords
    Let,
    Mut,
    Func,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Import,
    True,
    False,
    Struct,
    Enum,
    Impl,
    Trait,
    SelfType, // self
    Break,
    Continue,
    Match, // match keyword for pattern matching
    Ref,   // ref keyword for ref bindings
    As,    // as keyword for type casting
    Move,  // move keyword for closure captures

    // Types
    TypeInt,
    TypeFloat,
    TypeBool,
    TypeString,
    TypeList,
    TypeObject,

    // Interpolated string: segments of (is_expr, content)
    FormatString(Vec<(bool, String)>),

    // Operators
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    Equal,        // =
    EqualEqual,   // ==
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    And,          // &&
    Or,           // ||
    Not,          // !

    // Compound assignment operators
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Dot,          // .
    DotDot,       // ..
    DotDotEq,     // ..= (inclusive range)
    Colon,        // :
    ColonColon,   // :: (namespace separator)
    Semicolon,    // ;
    Arrow,        // ->
    FatArrow,     // => for match arms
    Ampersand,    // &
    Pipe,         // | for closures

    // Lifetimes
    Lifetime(String), // 'a, 'b, etc.

    // Special
    Newline,
    Eof,
}

impl TokenKind {
    pub fn keyword_from_str(s: &str) -> Option<TokenKind> {
        match s {
            "let" => Some(TokenKind::Let),
            "mut" => Some(TokenKind::Mut),
            "func" => Some(TokenKind::Func),
            "return" => Some(TokenKind::Return),
            "if" => Some(TokenKind::If),
            "else" => Some(TokenKind::Else),
            "while" => Some(TokenKind::While),
            "for" => Some(TokenKind::For),
            "in" => Some(TokenKind::In),
            "import" => Some(TokenKind::Import),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            "struct" => Some(TokenKind::Struct),
            "enum" => Some(TokenKind::Enum),
            "impl" => Some(TokenKind::Impl),
            "trait" => Some(TokenKind::Trait),
            "self" => Some(TokenKind::SelfType),
            "break" => Some(TokenKind::Break),
            "continue" => Some(TokenKind::Continue),
            "match" => Some(TokenKind::Match),
            "ref" => Some(TokenKind::Ref),
            "as" => Some(TokenKind::As),
            "move" => Some(TokenKind::Move),
            "Int" => Some(TokenKind::TypeInt),
            "Float" => Some(TokenKind::TypeFloat),
            "Bool" => Some(TokenKind::TypeBool),
            "String" => Some(TokenKind::TypeString),
            "List" => Some(TokenKind::TypeList),
            "Object" => Some(TokenKind::TypeObject),
            _ => None,
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Int(n) => write!(f, "{}", n),
            TokenKind::Float(n) => write!(f, "{}", n),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::Char(c) => write!(f, "'{}'", c),
            TokenKind::FormatString(parts) => {
                write!(f, "f\"")?;
                for (is_expr, content) in parts {
                    if *is_expr {
                        write!(f, "${{{}}}", content)?;
                    } else {
                        write!(f, "{}", content)?;
                    }
                }
                write!(f, "\"")
            }
            TokenKind::Bool(b) => write!(f, "{}", b),
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::Func => write!(f, "func"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Trait => write!(f, "trait"),
            TokenKind::SelfType => write!(f, "self"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Ref => write!(f, "ref"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Move => write!(f, "move"),
            TokenKind::TypeInt => write!(f, "Int"),
            TokenKind::TypeFloat => write!(f, "Float"),
            TokenKind::TypeBool => write!(f, "Bool"),
            TokenKind::TypeString => write!(f, "String"),
            TokenKind::TypeList => write!(f, "List"),
            TokenKind::TypeObject => write!(f, "Object"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Equal => write!(f, "="),
            TokenKind::EqualEqual => write!(f, "=="),
            TokenKind::NotEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Not => write!(f, "!"),
            TokenKind::PlusAssign => write!(f, "+="),
            TokenKind::MinusAssign => write!(f, "-="),
            TokenKind::StarAssign => write!(f, "*="),
            TokenKind::SlashAssign => write!(f, "/="),
            TokenKind::PercentAssign => write!(f, "%="),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotEq => write!(f, "..="),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Lifetime(l) => write!(f, "'{}", l),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// A token with its kind and source position
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: String) -> Self {
        Self { kind, span, lexeme }
    }
}
