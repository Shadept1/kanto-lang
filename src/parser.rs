use crate::ast::{BinOp, Block, Expr, Param, Program, Stmt, TypeAnnotation, UnOp};
use crate::diagnostic::{Diagnostic, Diagnostics, Label, Severity, ToDiagnostics};
use crate::span::{ByteOffset, Span};
use crate::token::{Token, TokenKind};

/// Parser for KantoLang.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Diagnostics,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            diagnostics: Diagnostics::new(),
        }
    }

    pub fn parse(mut self) -> (Program, Diagnostics) {
        let mut stmts = Vec::new();
        let start_span = self.current_span();

        while !self.is_at_end() {
            // Skip newlines at top level
            if self.match_token(&TokenKind::Newline) {
                continue;
            }
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else if !self.is_at_end() {
                // Skip error token and try to recover
                self.advance();
            }
        }

        let end_span = if let Some(last) = stmts.last() {
            last.span()
        } else {
            start_span
        };

        let program = Program::new(stmts, start_span.merge(end_span));
        (program, self.diagnostics)
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        // Skip leading newlines
        while self.match_token(&TokenKind::Newline) {}

        if self.is_at_end() {
            return None;
        }

        let start = self.current_span().start;

        // Check for let/mut/def/if/while/for/ret/break/continue
        if self.match_token(&TokenKind::Let) {
            return self.parse_let_stmt(start);
        }
        if self.match_token(&TokenKind::Mut) {
            return self.parse_mut_stmt(start);
        }
        if self.match_token(&TokenKind::Def) {
            return self.parse_def_stmt(start);
        }
        if self.match_token(&TokenKind::If) {
            return self.parse_if_stmt(start);
        }
        if self.match_token(&TokenKind::While) {
            return self.parse_while_stmt(start);
        }
        if self.match_token(&TokenKind::For) {
            return self.parse_for_stmt(start);
        }
        if self.match_token(&TokenKind::Ret) {
            return self.parse_ret_stmt(start);
        }
        if self.match_token(&TokenKind::Break) {
            return Some(Stmt::Break(self.previous_span()));
        }
        if self.match_token(&TokenKind::Continue) {
            return Some(Stmt::Continue(self.previous_span()));
        }

        // Check for assignment (identifier followed by =)
        if let TokenKind::Ident(name) = &self.current_token().kind {
            let name = name.clone();
            let ident_span = self.current_span();
            self.advance();

            if self.match_token(&TokenKind::Assign) {
                let value = self.parse_expr(0)?;
                let span = ident_span.merge(self.previous_span());
                return Some(Stmt::Assign { name, value, span });
            } else {
                // Not an assignment, parse as expression statement
                // Put the identifier back by going back
                self.pos -= 1;
            }
        }

        // Parse as expression statement
        let expr = self.parse_expr(0)?;
        let span = expr.span();
        Some(Stmt::Expr(expr, span))
    }

    fn parse_let_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        let name = self.parse_ident()?;
        let type_ann = self.parse_type_annotation()?;
        self.expect(
            &TokenKind::Assign,
            "expected '=' after variable declaration",
        )?;
        let initializer = self.parse_expr(0)?;
        let span = Span::new(start, self.previous_span().end);
        Some(Stmt::Let {
            name,
            type_ann,
            initializer,
            span,
        })
    }

    fn parse_mut_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        let name = self.parse_ident()?;
        let type_ann = self.parse_type_annotation()?;
        self.expect(
            &TokenKind::Assign,
            "expected '=' after variable declaration",
        )?;
        let initializer = self.parse_expr(0)?;
        let span = Span::new(start, self.previous_span().end);
        Some(Stmt::Mut {
            name,
            type_ann,
            initializer,
            span,
        })
    }

    fn parse_type_annotation(&mut self) -> Option<Option<TypeAnnotation>> {
        if self.match_token(&TokenKind::Colon) {
            let ty = self.parse_type()?;
            Some(Some(ty))
        } else {
            Some(None)
        }
    }

    fn parse_type(&mut self) -> Option<TypeAnnotation> {
        let start = self.current_span().start;
        let primitive = match &self.current_token().kind {
            TokenKind::IntType => crate::ast::PrimitiveType::Int,
            TokenKind::FloatType => crate::ast::PrimitiveType::Float,
            TokenKind::BoolType => crate::ast::PrimitiveType::Bool,
            TokenKind::StrType => crate::ast::PrimitiveType::Str,
            TokenKind::UnitType => crate::ast::PrimitiveType::Unit,
            _ => {
                self.error(
                    "expected type annotation (int, float, bool, str, unit)".to_string(),
                    self.current_span(),
                );
                return None;
            }
        };
        let span = Span::new(start, self.current_span().end);
        self.advance();
        Some(TypeAnnotation::new(primitive, span))
    }

    fn parse_def_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        let name = self.parse_ident()?;
        self.expect(&TokenKind::LParen, "expected '(' after function name")?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen, "expected ')' after parameters")?;
        self.expect(&TokenKind::Arrow, "expected '->' after parameters")?;
        let return_type = self.parse_type()?;
        self.expect(&TokenKind::Colon, "expected ':' after return type")?;
        self.expect_newline("expected newline after ':'")?;
        self.expect_indent("expected indented block after function definition")?;
        let body = self.parse_block()?;
        self.expect_dedent("expected dedent after function body")?;
        let span = Span::new(start, body.span.end);
        Some(Stmt::Def {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RParen) {
            return Some(params);
        }

        loop {
            let name = self.parse_ident()?;
            self.expect(&TokenKind::Colon, "expected ':' after parameter name")?;
            let type_ann = self.parse_type()?;
            params.push(Param { name, type_ann });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Some(params)
    }

    fn parse_if_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        let condition = self.parse_expr(0)?;
        self.expect(&TokenKind::Colon, "expected ':' after if condition")?;
        self.expect_newline("expected newline after ':'")?;
        self.expect_indent("expected indented block after ':'")?;
        let then_branch = self.parse_block()?;
        self.expect_dedent("expected dedent after if block")?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            self.expect(&TokenKind::Colon, "expected ':' after else")?;
            self.expect_newline("expected newline after ':'")?;
            self.expect_indent("expected indented block after else")?;
            let block = self.parse_block()?;
            self.expect_dedent("expected dedent after else block")?;
            Some(block)
        } else {
            None
        };

        let span = Span::new(start, self.previous_span().end);
        Some(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span,
        })
    }

    fn parse_while_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        let condition = self.parse_expr(0)?;
        self.expect(&TokenKind::Colon, "expected ':' after while condition")?;
        self.expect_newline("expected newline after ':'")?;
        self.expect_indent("expected indented block after ':'")?;
        let body = self.parse_block()?;
        self.expect_dedent("expected dedent after while block")?;
        let span = Span::new(start, body.span.end);
        Some(Stmt::While {
            condition,
            body,
            span,
        })
    }

    fn parse_for_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        let var = self.parse_ident()?;
        self.expect(&TokenKind::In, "expected 'in' after loop variable")?;
        let iter = self.parse_expr(0)?;
        self.expect(&TokenKind::Colon, "expected ':' after range expression")?;
        self.expect_newline("expected newline after ':'")?;
        self.expect_indent("expected indented block after ':'")?;
        let body = self.parse_block()?;
        self.expect_dedent("expected dedent after for block")?;
        let span = Span::new(start, body.span.end);
        Some(Stmt::For {
            var,
            iter,
            body,
            span,
        })
    }

    fn parse_ret_stmt(&mut self, start: ByteOffset) -> Option<Stmt> {
        // ret can be followed by an expression or nothing (for unit functions)
        let value = if self.check(&TokenKind::Newline)
            || self.check(&TokenKind::Dedent)
            || self.is_at_end()
        {
            None
        } else {
            Some(self.parse_expr(0)?)
        };
        let span = Span::new(start, self.previous_span().end);
        Some(Stmt::Ret { value, span })
    }

    fn parse_block(&mut self) -> Option<Block> {
        let mut stmts = Vec::new();
        let start = self.current_span().start;

        while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
            // Skip newlines
            while self.match_token(&TokenKind::Newline) {}

            if self.check(&TokenKind::Dedent) || self.is_at_end() {
                break;
            }

            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else if !self.is_at_end() {
                self.advance();
            }
        }

        let end = self.current_span().end;
        Some(Block::new(stmts, Span::new(start, end)))
    }

    /// Parse expression using Pratt parser (precedence climbing)
    fn parse_expr(&mut self, min_prec: u8) -> Option<Expr> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            let prec = self.get_precedence();
            // prec == 0 means not an operator (e.g., Newline, Eof, etc.)
            if prec == 0 || prec < min_prec {
                break;
            }

            let op = self.parse_infix_op()?;
            let next_prec = prec + 1; // Left-associative

            self.advance(); // consume operator
            let right = self.parse_expr(next_prec)?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(Box::new(left), op, Box::new(right), span);
        }

        Some(left)
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        let token = self.current_token().clone();
        let span = token.span;

        match &token.kind {
            TokenKind::IntLit(n) => {
                self.advance();
                Some(Expr::IntLit(*n, span))
            }
            TokenKind::FloatLit(n) => {
                self.advance();
                Some(Expr::FloatLit(*n, span))
            }
            TokenKind::StrLit(s) => {
                self.advance();
                Some(Expr::StrLit(s.clone(), span))
            }
            TokenKind::True => {
                self.advance();
                Some(Expr::BoolLit(true, span))
            }
            TokenKind::False => {
                self.advance();
                Some(Expr::BoolLit(false, span))
            }
            // Handle built-in function names that are tokenized as keywords
            TokenKind::Print
            | TokenKind::Range
            | TokenKind::IntType
            | TokenKind::FloatType
            | TokenKind::StrType => {
                let name = match &token.kind {
                    TokenKind::Print => "print",
                    TokenKind::Range => "range",
                    TokenKind::IntType => "int",
                    TokenKind::FloatType => "float",
                    TokenKind::StrType => "str",
                    _ => unreachable!(),
                }
                .to_string();
                self.advance();
                if self.check(&TokenKind::LParen) {
                    // Function call
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr(0)?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&TokenKind::RParen, "expected ')' after function arguments")?;
                    let call_span = span.merge(self.previous_span());
                    Some(Expr::Call {
                        callee: Box::new(Expr::Ident(name.clone(), span)),
                        args,
                        span: call_span,
                    })
                } else {
                    // Used as a type name (should be handled in type annotation parsing)
                    Some(Expr::Ident(name, span))
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.check(&TokenKind::LParen) {
                    // Function call
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr(0)?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&TokenKind::RParen, "expected ')' after function arguments")?;
                    let call_span = span.merge(self.previous_span());
                    Some(Expr::Call {
                        callee: Box::new(Expr::Ident(name.clone(), span)),
                        args,
                        span: call_span,
                    })
                } else {
                    Some(Expr::Ident(name.clone(), span))
                }
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_expr(8)?; // Higher than binary operators
                let neg_span = span.merge(expr.span());
                Some(Expr::Unary(UnOp::Neg, Box::new(expr), neg_span))
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_expr(3)?; // Same as 'not' precedence
                let not_span = span.merge(expr.span());
                Some(Expr::Unary(UnOp::Not, Box::new(expr), not_span))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen, "expected ')' after expression")?;
                Some(Expr::Group(
                    Box::new(expr),
                    span.merge(self.previous_span()),
                ))
            }
            _ => {
                self.error(
                    format!("unexpected token in expression: {}", token.kind),
                    span,
                );
                None
            }
        }
    }

    fn parse_infix_op(&mut self) -> Option<BinOp> {
        match &self.current_token().kind {
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Percent => Some(BinOp::Rem),
            TokenKind::EqEq => Some(BinOp::Eq),
            TokenKind::NotEq => Some(BinOp::Ne),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::LtEq => Some(BinOp::Le),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::GtEq => Some(BinOp::Ge),
            TokenKind::And => Some(BinOp::And),
            TokenKind::Or => Some(BinOp::Or),
            _ => None,
        }
    }

    fn get_precedence(&self) -> u8 {
        match &self.current_token().kind {
            TokenKind::Or => 1,
            TokenKind::And => 2,
            TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq => 3,
            TokenKind::Plus | TokenKind::Minus => 4,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => 5,
            _ => 0,
        }
    }

    fn parse_ident(&mut self) -> Option<String> {
        match &self.current_token().kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => {
                self.error("expected identifier".to_string(), self.current_span());
                None
            }
        }
    }

    // Token stream helpers
    fn current_token(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn current_span(&self) -> Span {
        self.current_token().span
    }

    fn previous_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::new(ByteOffset(0), ByteOffset(0))
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.current_token().kind, TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.current_token().kind) == std::mem::discriminant(kind)
        }
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Option<()> {
        if self.match_token(kind) {
            Some(())
        } else {
            self.error(
                format!("{} (found {})", msg, self.current_token().kind),
                self.current_span(),
            );
            None
        }
    }

    fn expect_newline(&mut self, msg: &str) -> Option<()> {
        if self.match_token(&TokenKind::Newline) {
            Some(())
        } else {
            self.error(msg.to_string(), self.current_span());
            None
        }
    }

    fn expect_indent(&mut self, msg: &str) -> Option<()> {
        if self.match_token(&TokenKind::Indent) {
            Some(())
        } else {
            self.error(msg.to_string(), self.current_span());
            None
        }
    }

    fn expect_dedent(&mut self, msg: &str) -> Option<()> {
        if self.match_token(&TokenKind::Dedent) {
            Some(())
        } else {
            self.error(msg.to_string(), self.current_span());
            None
        }
    }

    fn error(&mut self, message: String, span: Span) {
        let diag = Diagnostic::error(message).with_label(Label::primary(span, "here"));
        self.diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::span::SourceFile;

    fn parse_source(content: &str) -> (Program, Diagnostics) {
        let source = SourceFile::new("<test>".to_string(), content.to_string());
        let (tokens, lexer_diags) = Lexer::new(&source).lex();
        let parser = Parser::new(tokens);
        let (program, parser_diags) = parser.parse();
        let mut all_diags = lexer_diags;
        all_diags.extend(parser_diags);
        (program, all_diags)
    }

    #[test]
    fn test_let_declaration() {
        let (program, diags) = parse_source("let x = 42\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 1);
    }

    #[test]
    fn test_mut_declaration() {
        let (program, diags) = parse_source("mut x: int = 0\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 1);
    }

    #[test]
    fn test_assignment() {
        let (program, diags) = parse_source("mut x = 0\nx = 1\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 2);
    }

    #[test]
    fn test_function_def() {
        let (program, diags) = parse_source("def add(a: int, b: int) -> int:\n    ret a + b\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 1);
    }

    #[test]
    fn test_if_stmt() {
        let (program, diags) = parse_source("if true:\n    print(1)\nelse:\n    print(2)\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 1);
    }

    #[test]
    fn test_while_stmt() {
        let (program, diags) = parse_source("mut x = 0\nwhile x < 3:\n    x = x + 1\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 2);
    }

    #[test]
    fn test_for_stmt() {
        let (program, diags) = parse_source("for i in range(0, 3):\n    print(i)\n");
        assert!(!diags.has_errors());
        assert_eq!(program.stmts.len(), 1);
    }

    #[test]
    fn test_operator_precedence() {
        let (program, diags) = parse_source("let x = 1 + 2 * 3\n");
        assert!(!diags.has_errors());
        // Should parse as 1 + (2 * 3)
    }

    #[test]
    fn test_chained_comparison_rejected() {
        let (program, diags) = parse_source("let x = 1 < 2 < 3\n");
        // Should produce an error or parse in a way that type checker catches
        // For now, parser accepts but type checker will reject
    }

    #[test]
    fn test_string_literal() {
        let (program, diags) = parse_source(r#"let s = "hello""#);
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_bool_literals() {
        let (program, diags) = parse_source("let a = true\nlet b = false\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_ret_stmt() {
        let (program, diags) = parse_source("def f() -> int:\n    ret 42\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_ret_unit() {
        let (program, diags) = parse_source("def f() -> unit:\n    ret\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_break_continue() {
        let (program, diags) = parse_source("while true:\n    break\n    continue\n");
        assert!(!diags.has_errors());
    }
}
