use crate::diagnostic::{Diagnostic, Diagnostics, Label, Severity, ToDiagnostics};
use crate::span::{ByteOffset, SourceFile, Span};
use crate::token::{Token, TokenKind, is_ident_continue, is_ident_start, keyword_token_kind};

/// The lexer for KantoLang.
pub struct Lexer<'a> {
    source: &'a SourceFile,
    content: &'a str,
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    pending_dedents: usize,
    at_line_start: bool,
    paren_depth: usize,
    diagnostics: Diagnostics,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        Lexer {
            source,
            content: &source.content,
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            pending_dedents: 0,
            at_line_start: true,
            paren_depth: 0,
            diagnostics: Diagnostics::new(),
        }
    }

    /// Lex the entire source file into tokens.
    pub fn lex(mut self) -> (Vec<Token>, Diagnostics) {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            if let Some(token) = self.next_token() {
                tokens.push(token);
            }
        }

        // Emit pending dedents
        while self.pending_dedents > 0 {
            tokens.push(self.make_dedent());
            self.pending_dedents -= 1;
        }

        // Emit dedents for any remaining indentation levels at EOF
        // (skip the base level 0)
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(self.make_dedent());
        }

        // Final EOF
        tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(ByteOffset(self.pos), ByteOffset(self.pos)),
        ));

        (tokens, self.diagnostics)
    }

    fn next_token(&mut self) -> Option<Token> {
        // Handle pending dedents first
        if self.pending_dedents > 0 {
            self.pending_dedents -= 1;
            return Some(self.make_dedent());
        }

        // At line start with non-whitespace: check for dedents based on column 0
        if self.at_line_start {
            let c = self.peek();
            if c.is_some()
                && !matches!(
                    c,
                    Some(' ') | Some('\t') | Some('\n') | Some('\r') | Some('#')
                )
            {
                // Non-whitespace at column 0 - emit dedents if needed
                let current_indent = 0;
                while let Some(&level) = self.indent_stack.last() {
                    if level <= current_indent {
                        break;
                    }
                    self.indent_stack.pop();
                    self.pending_dedents += 1;
                }
                if self.pending_dedents > 0 {
                    self.pending_dedents -= 1;
                    return Some(self.make_dedent());
                }
            }
        }

        let start = self.pos;
        let c = self.peek();

        match c {
            Some('\n') | Some('\r') => self.lex_newline(),
            Some(' ') | Some('\t') if self.at_line_start => self.lex_indentation(),
            Some(' ') | Some('\t') => {
                self.advance();
                self.next_token()
            }
            Some('#') => self.lex_comment(),
            Some(c) if is_ident_start(c) => self.lex_identifier_or_keyword(),
            Some('0'..='9') => self.lex_number(),
            Some('"') => self.lex_string(),
            Some('+') => Some(self.make_single(TokenKind::Plus)),
            Some('-') => self.lex_minus(),
            Some('*') => Some(self.make_single(TokenKind::Star)),
            Some('/') => Some(self.make_single(TokenKind::Slash)),
            Some('%') => Some(self.make_single(TokenKind::Percent)),
            Some('=') => self.lex_eq(),
            Some('!') => self.lex_not_eq(),
            Some('<') => self.lex_lt(),
            Some('>') => self.lex_gt(),
            Some(':') => self.lex_colon(),
            Some(',') => Some(self.make_single(TokenKind::Comma)),
            Some('(') => {
                self.paren_depth += 1;
                Some(self.make_single(TokenKind::LParen))
            }
            Some(')') => {
                if self.paren_depth > 0 {
                    self.paren_depth -= 1;
                }
                Some(self.make_single(TokenKind::RParen))
            }
            Some(c) => {
                self.advance();
                self.error(
                    format!("unexpected character '{}'", c),
                    Span::new(ByteOffset(start), ByteOffset(self.pos)),
                );
                None
            }
            None => None,
        }
    }

    fn lex_newline(&mut self) -> Option<Token> {
        let start = self.pos;
        if self.peek() == Some('\r') {
            self.advance();
            if self.peek() == Some('\n') {
                self.advance();
            }
        } else if self.peek() == Some('\n') {
            self.advance();
        }
        self.line += 1;
        self.col = 1;
        self.at_line_start = true;

        // If we're inside parentheses, newlines don't create newline tokens
        if self.paren_depth > 0 {
            return self.next_token();
        }

        Some(Token::new(
            TokenKind::Newline,
            Span::new(ByteOffset(start), ByteOffset(self.pos)),
        ))
    }

    fn lex_indentation(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut indent = 0;

        while let Some(c) = self.peek() {
            match c {
                ' ' => {
                    indent += 1;
                    self.advance();
                }
                '\t' => {
                    self.error(
                        "tabs are not allowed in indentation; use 4 spaces".to_string(),
                        Span::new(ByteOffset(start), ByteOffset(self.pos + 1)),
                    );
                    self.advance();
                }
                _ => break,
            }
        }

        // If line is empty or comment only, ignore indentation
        if matches!(self.peek(), Some('\n') | Some('\r') | Some('#') | None) {
            self.at_line_start = true;
            return self.next_token();
        }

        let current_indent = *self.indent_stack.last().unwrap_or(&0);

        if indent > current_indent {
            if indent != current_indent + 4 {
                self.error(
                    format!(
                        "inconsistent indentation: expected {} spaces, found {}",
                        current_indent + 4,
                        indent
                    ),
                    Span::new(ByteOffset(start), ByteOffset(self.pos)),
                );
            }
            self.indent_stack.push(indent);
            self.at_line_start = false;
            Some(Token::new(
                TokenKind::Indent,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else if indent < current_indent {
            // Count how many levels to dedent
            let mut dedents = 0;
            while let Some(&level) = self.indent_stack.last() {
                if level <= indent {
                    break;
                }
                self.indent_stack.pop();
                dedents += 1;
            }
            if dedents > 0 {
                self.pending_dedents = dedents - 1;
                self.at_line_start = false;
                Some(self.make_dedent())
            } else {
                self.at_line_start = false;
                self.next_token()
            }
        } else {
            self.at_line_start = false;
            self.next_token()
        }
    }

    fn lex_comment(&mut self) -> Option<Token> {
        self.advance(); // skip '#'
        while let Some(c) = self.peek() {
            if c == '\n' || c == '\r' {
                // Consume the newline so it doesn't produce a token
                self.advance();
                if c == '\r' && self.peek() == Some('\n') {
                    self.advance();
                }
                self.line += 1;
                self.col = 1;
                self.at_line_start = true;
                break;
            }
            self.advance();
        }
        // Comments don't produce tokens, continue lexing
        self.next_token()
    }

    fn lex_identifier_or_keyword(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut ident = String::new();

        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        self.at_line_start = false;

        // Check for built-in names that are context-dependent
        if ident == "str" || ident == "int" || ident == "float" {
            // These could be type names or built-in functions
            // We'll disambiguate in the parser based on context
            if let Some(kind) = keyword_token_kind(&ident) {
                return Some(Token::new(
                    kind,
                    Span::new(ByteOffset(start), ByteOffset(self.pos)),
                ));
            }
        }

        if let Some(kind) = keyword_token_kind(&ident) {
            Some(Token::new(
                kind,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else {
            Some(Token::new(
                TokenKind::Ident(ident),
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        }
    }

    fn lex_number(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut value = String::new();
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                value.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                // Check if next char is a digit
                if matches!(self.peek_nth(1), Some('0'..='9')) {
                    is_float = true;
                    value.push(c);
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.at_line_start = false;

        if is_float {
            match value.parse::<f64>() {
                Ok(f) => Some(Token::new(
                    TokenKind::FloatLit(f),
                    Span::new(ByteOffset(start), ByteOffset(self.pos)),
                )),
                Err(_) => {
                    self.error(
                        format!("invalid float literal: {}", value),
                        Span::new(ByteOffset(start), ByteOffset(self.pos)),
                    );
                    None
                }
            }
        } else {
            match value.parse::<i64>() {
                Ok(i) => Some(Token::new(
                    TokenKind::IntLit(i),
                    Span::new(ByteOffset(start), ByteOffset(self.pos)),
                )),
                Err(_) => {
                    self.error(
                        format!("integer literal out of range: {}", value),
                        Span::new(ByteOffset(start), ByteOffset(self.pos)),
                    );
                    None
                }
            }
        }
    }

    fn lex_string(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance(); // skip opening quote
        let mut value = String::new();
        let mut terminated = false;

        while let Some(c) = self.peek() {
            match c {
                '"' => {
                    self.advance();
                    terminated = true;
                    break;
                }
                '\\' => {
                    self.advance();
                    match self.peek() {
                        Some('n') => {
                            value.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            value.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            value.push('\r');
                            self.advance();
                        }
                        Some('"') => {
                            value.push('"');
                            self.advance();
                        }
                        Some('\\') => {
                            value.push('\\');
                            self.advance();
                        }
                        Some(c) => {
                            self.error(
                                format!("invalid escape sequence: \\{}", c),
                                Span::new(ByteOffset(self.pos - 1), ByteOffset(self.pos + 1)),
                            );
                            self.advance();
                            value.push(c);
                        }
                        None => {
                            self.error(
                                "unterminated escape sequence".to_string(),
                                Span::new(ByteOffset(self.pos - 1), ByteOffset(self.pos)),
                            );
                            break;
                        }
                    }
                }
                '\n' | '\r' => {
                    self.error(
                        "unterminated string literal".to_string(),
                        Span::new(ByteOffset(start), ByteOffset(self.pos)),
                    );
                    break;
                }
                _ => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        if !terminated {
            self.error(
                "unterminated string literal".to_string(),
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            );
        }

        self.at_line_start = false;

        Some(Token::new(
            TokenKind::StrLit(value),
            Span::new(ByteOffset(start), ByteOffset(self.pos)),
        ))
    }

    fn lex_eq(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            Some(Token::new(
                TokenKind::EqEq,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else {
            Some(Token::new(
                TokenKind::Assign,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        }
    }

    fn lex_not_eq(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            Some(Token::new(
                TokenKind::NotEq,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else {
            self.error(
                "expected '=' after '!'".to_string(),
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            );
            None
        }
    }

    fn lex_lt(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            Some(Token::new(
                TokenKind::LtEq,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else {
            Some(Token::new(
                TokenKind::Lt,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        }
    }

    fn lex_gt(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance();
        if self.peek() == Some('=') {
            self.advance();
            Some(Token::new(
                TokenKind::GtEq,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else if self.peek() == Some('-') {
            // This could be confused with -> but we handle -> in lex_colon
            Some(Token::new(
                TokenKind::Gt,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else {
            Some(Token::new(
                TokenKind::Gt,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        }
    }

    fn lex_minus(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance();
        if self.peek() == Some('>') {
            self.advance();
            Some(Token::new(
                TokenKind::Arrow,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        } else {
            Some(Token::new(
                TokenKind::Minus,
                Span::new(ByteOffset(start), ByteOffset(self.pos)),
            ))
        }
    }

    fn lex_colon(&mut self) -> Option<Token> {
        let start = self.pos;
        self.advance();
        Some(Token::new(
            TokenKind::Colon,
            Span::new(ByteOffset(start), ByteOffset(self.pos)),
        ))
    }

    fn make_single(&mut self, kind: TokenKind) -> Token {
        let start = self.pos;
        self.advance();
        self.at_line_start = false;
        Token::new(kind, Span::new(ByteOffset(start), ByteOffset(self.pos)))
    }

    fn make_dedent(&mut self) -> Token {
        let start = self.pos;
        Token::new(
            TokenKind::Dedent,
            Span::new(ByteOffset(start), ByteOffset(start)),
        )
    }

    fn peek(&self) -> Option<char> {
        self.content[self.pos..].chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.content[self.pos + n..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += c.len_utf8();
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.content.len()
    }

    fn error(&mut self, message: String, span: Span) {
        let diag = Diagnostic::error(message).with_label(Label::primary(span, "here"));
        self.diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::SourceFile;

    fn lex_source(content: &str) -> (Vec<Token>, Diagnostics) {
        let source = SourceFile::new("<test>".to_string(), content.to_string());
        let lexer = Lexer::new(&source);
        lexer.lex()
    }

    #[test]
    fn test_simple_let() {
        let (tokens, diags) = lex_source("let x = 42\n");
        assert!(!diags.has_errors());
        assert_eq!(tokens.len(), 6); // let, ident, =, int, newline, eof
    }

    #[test]
    fn test_indentation() {
        let (tokens, diags) = lex_source("if true:\n    print(1)\n");
        assert!(!diags.has_errors());
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(kinds.contains(&&TokenKind::Indent));
        assert!(kinds.contains(&&TokenKind::Dedent));
    }

    #[test]
    fn test_string_escapes() {
        let (tokens, diags) = lex_source(r#""hello\nworld""#);
        assert!(!diags.has_errors());
        if let TokenKind::StrLit(s) = &tokens[0].kind {
            assert_eq!(s, "hello\nworld");
        }
    }

    #[test]
    fn test_invalid_escape() {
        let (tokens, diags) = lex_source(r#""hello\z""#);
        assert!(diags.has_errors());
    }

    #[test]
    fn test_unterminated_string() {
        let (tokens, diags) = lex_source(r#""hello"#);
        assert!(diags.has_errors());
    }

    #[test]
    fn test_tabs_in_indentation() {
        let (tokens, diags) = lex_source("if true:\n\tprint(1)\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_inconsistent_indentation() {
        let (tokens, diags) = lex_source("if true:\n   print(1)\n"); // 3 spaces instead of 4
        assert!(diags.has_errors());
    }

    #[test]
    fn test_float_literal() {
        let (tokens, diags) = lex_source("3.14\n");
        assert!(!diags.has_errors());
        assert!(matches!(tokens[0].kind, TokenKind::FloatLit(_)));
    }

    #[test]
    fn test_int_literal() {
        let (tokens, diags) = lex_source("42\n");
        assert!(!diags.has_errors());
        assert!(matches!(tokens[0].kind, TokenKind::IntLit(42)));
    }

    #[test]
    fn test_keywords() {
        let (tokens, diags) = lex_source(
            "let mut def ret if else while for in break continue true false and or not\n",
        );
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_operators() {
        let (tokens, diags) = lex_source("+ - * / % == != < <= > >= = : -> , ( )\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_comment() {
        let (tokens, diags) = lex_source("# comment\nlet x = 1\n");
        assert!(!diags.has_errors());
        // comment should not produce a token
        assert!(matches!(tokens[0].kind, TokenKind::Let));
    }

    #[test]
    fn test_newline_in_parens() {
        let (tokens, diags) = lex_source("(1\n+\n2)\n");
        assert!(!diags.has_errors());
        // Should not have Newline tokens inside parens
        let newlines: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Newline))
            .collect();
        assert_eq!(newlines.len(), 1); // only the final one
    }
}
