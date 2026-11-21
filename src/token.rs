use std::fmt;

use crate::span::Span;

/// Token kinds for KantoLang.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Let,
    Mut,
    Def,
    Ret,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    True,
    False,
    And,
    Or,
    Not,

    // Types
    IntType,
    FloatType,
    BoolType,
    StrType,
    UnitType,

    // Built-ins
    Print,
    StrBuiltin,
    IntBuiltin,
    FloatBuiltin,
    Range,

    // Identifiers and literals
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),

    // Operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Colon,
    Arrow,
    Comma,
    LParen,
    RParen,
    Assign,

    // Structure
    Newline,
    Indent,
    Dedent,
    Eof,

    // Error tokens
    Error(String),
}

impl Eq for TokenKind {}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::Def => write!(f, "def"),
            TokenKind::Ret => write!(f, "ret"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::IntType => write!(f, "int"),
            TokenKind::FloatType => write!(f, "float"),
            TokenKind::BoolType => write!(f, "bool"),
            TokenKind::StrType => write!(f, "str"),
            TokenKind::UnitType => write!(f, "unit"),
            TokenKind::Print => write!(f, "print"),
            TokenKind::StrBuiltin => write!(f, "str"),
            TokenKind::IntBuiltin => write!(f, "int"),
            TokenKind::FloatBuiltin => write!(f, "float"),
            TokenKind::Range => write!(f, "range"),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::IntLit(n) => write!(f, "{}", n),
            TokenKind::FloatLit(n) => write!(f, "{}", n),
            TokenKind::StrLit(s) => write!(f, "\"{}\"", s),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::Assign => write!(f, "="),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Indent => write!(f, "INDENT"),
            TokenKind::Dedent => write!(f, "DEDENT"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Error(s) => write!(f, "ERROR: {}", s),
        }
    }
}

/// A token with its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    pub fn error(msg: String, span: Span) -> Self {
        Token {
            kind: TokenKind::Error(msg),
            span,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

/// Check if a character can start an identifier.
pub fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Check if a character can continue an identifier.
pub fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Keywords that are reserved and cannot be used as identifiers.
pub fn is_keyword(ident: &str) -> bool {
    matches!(
        ident,
        "let"
            | "mut"
            | "def"
            | "ret"
            | "if"
            | "else"
            | "while"
            | "for"
            | "in"
            | "break"
            | "continue"
            | "true"
            | "false"
            | "and"
            | "or"
            | "not"
            | "int"
            | "float"
            | "bool"
            | "str"
            | "unit"
            | "print"
            | "range"
    )
}

/// Get the token kind for a keyword identifier.
pub fn keyword_token_kind(ident: &str) -> Option<TokenKind> {
    match ident {
        "let" => Some(TokenKind::Let),
        "mut" => Some(TokenKind::Mut),
        "def" => Some(TokenKind::Def),
        "ret" => Some(TokenKind::Ret),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "and" => Some(TokenKind::And),
        "or" => Some(TokenKind::Or),
        "not" => Some(TokenKind::Not),
        "int" => Some(TokenKind::IntType),
        "float" => Some(TokenKind::FloatType),
        "bool" => Some(TokenKind::BoolType),
        "str" => Some(TokenKind::StrType),
        "unit" => Some(TokenKind::UnitType),
        "print" => Some(TokenKind::Print),
        "range" => Some(TokenKind::Range),
        _ => None,
    }
}
