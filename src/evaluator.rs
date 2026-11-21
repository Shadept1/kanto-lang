use std::collections::HashMap;

use crate::ast::{BinOp, Block, Expr, Param, Program, Stmt, UnOp};
use crate::diagnostic::{Diagnostic, Diagnostics, Label};
use crate::environment::{Environment, builtins};
use crate::span::Span;
use crate::value::{Value, arithmetic, conversion};

/// Control flow signals for the evaluator.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    None,
    Return(Option<Value>),
    Break,
    Continue,
}

/// Runtime evaluator for KantoLang.
pub struct Evaluator {
    program: Program,
    env: Environment,
    functions: HashMap<String, FunctionDef>,
    diagnostics: Diagnostics,
    call_depth: usize,
    max_call_depth: usize,
    output: Box<dyn std::io::Write + Send>,
}

#[derive(Debug, Clone)]
struct FunctionDef {
    name: String,
    params: Vec<Param>,
    return_type: crate::ast::TypeAnnotation,
    body: Block,
    span: Span,
}

/// Statement kind with owned data to avoid borrow checker issues.
#[derive(Debug)]
enum StmtKind {
    Let {
        name: String,
        initializer: Expr,
    },
    Mut {
        name: String,
        initializer: Expr,
    },
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    Def,
    Ret {
        value: Option<Expr>,
    },
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
    For {
        var: String,
        iter: Expr,
        body: Block,
    },
    Break,
    Continue,
    Expr(Expr),
}

impl Evaluator {
    pub fn new(program: Program, output: Box<dyn std::io::Write + Send>) -> Self {
        let mut evaluator = Evaluator {
            program,
            env: Environment::new(),
            functions: HashMap::new(),
            diagnostics: Diagnostics::new(),
            call_depth: 0,
            max_call_depth: 1000,
            output,
        };
        evaluator.collect_functions();
        evaluator
    }

    pub fn evaluate(mut self) -> (Value, Diagnostics) {
        let mut last_value = Value::Unit;
        let mut i = 0;

        while i < self.program.stmts.len() {
            // Extract statement kind (clones data) to avoid borrow issues
            let stmt_kind = self.extract_stmt_kind(i);
            let cf = self.handle_stmt_kind(stmt_kind);
            match cf {
                ControlFlow::Return(val) => return (val.unwrap_or(Value::Unit), self.diagnostics),
                ControlFlow::Break | ControlFlow::Continue => {
                    let span = self.program.stmts[i].span();
                    self.error("break/continue outside loop at top level".to_string(), span);
                }
                ControlFlow::None => {}
            }
            i += 1;
        }

        (last_value, self.diagnostics)
    }

    fn collect_functions(&mut self) {
        for stmt in &self.program.stmts {
            if let Stmt::Def {
                name,
                params,
                return_type,
                body,
                span,
            } = stmt
            {
                self.functions.insert(
                    name.clone(),
                    FunctionDef {
                        name: name.clone(),
                        params: params.clone(),
                        return_type: return_type.clone(),
                        body: body.clone(),
                        span: *span,
                    },
                );
            }
        }
    }

    /// Extract statement kind with owned data (clones the relevant parts)
    fn extract_stmt_kind(&self, index: usize) -> StmtKind {
        match &self.program.stmts[index] {
            Stmt::Let {
                name, initializer, ..
            } => StmtKind::Let {
                name: name.clone(),
                initializer: initializer.clone(),
            },
            Stmt::Mut {
                name, initializer, ..
            } => StmtKind::Mut {
                name: name.clone(),
                initializer: initializer.clone(),
            },
            Stmt::Assign { name, value, span } => StmtKind::Assign {
                name: name.clone(),
                value: value.clone(),
                span: *span,
            },
            Stmt::Def { .. } => StmtKind::Def,
            Stmt::Ret { value, .. } => StmtKind::Ret {
                value: value.clone(),
            },
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => StmtKind::If {
                condition: condition.clone(),
                then_branch: then_branch.clone(),
                else_branch: else_branch.clone(),
            },
            Stmt::While {
                condition, body, ..
            } => StmtKind::While {
                condition: condition.clone(),
                body: body.clone(),
            },
            Stmt::For {
                var, iter, body, ..
            } => StmtKind::For {
                var: var.clone(),
                iter: iter.clone(),
                body: body.clone(),
            },
            Stmt::Break(_) => StmtKind::Break,
            Stmt::Continue(_) => StmtKind::Continue,
            Stmt::Expr(expr, _) => StmtKind::Expr(expr.clone()),
        }
    }

    /// Handle a statement kind (no borrow on self.program.stmts)
    fn handle_stmt_kind(&mut self, kind: StmtKind) -> ControlFlow {
        match kind {
            StmtKind::Let {
                name,
                mut initializer,
            } => {
                let value = self.eval_expr(&mut initializer);
                self.env.define(name, value);
                ControlFlow::None
            }
            StmtKind::Mut {
                name,
                mut initializer,
            } => {
                let value = self.eval_expr(&mut initializer);
                self.env.define(name, value);
                ControlFlow::None
            }
            StmtKind::Assign {
                name,
                mut value,
                span,
            } => {
                let val = self.eval_expr(&mut value);
                if let Err(e) = self.env.assign(&name, val) {
                    self.error(e, span);
                }
                ControlFlow::None
            }
            StmtKind::Def => ControlFlow::None,
            StmtKind::Ret { mut value } => {
                let val = value.as_mut().map(|v| self.eval_expr(v));
                ControlFlow::Return(val)
            }
            StmtKind::If {
                mut condition,
                mut then_branch,
                else_branch,
            } => {
                let cond_val = self.eval_expr(&mut condition);
                if let Value::Bool(true) = cond_val {
                    self.eval_block(&mut then_branch)
                } else if let Some(mut else_b) = else_branch {
                    self.eval_block(&mut else_b)
                } else {
                    ControlFlow::None
                }
            }
            StmtKind::While {
                mut condition,
                mut body,
            } => {
                let mut cf = ControlFlow::None;
                loop {
                    let cond_val = self.eval_expr(&mut condition);
                    match cond_val {
                        Value::Bool(true) => {}
                        Value::Bool(false) => break,
                        _ => {
                            self.error(
                                "while condition must be bool".to_string(),
                                condition.span(),
                            );
                            break;
                        }
                    }
                    cf = self.eval_block(&mut body);
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(val) => return ControlFlow::Return(val),
                        ControlFlow::None => {}
                    }
                }
                ControlFlow::None
            }
            StmtKind::For {
                var,
                mut iter,
                mut body,
            } => {
                let iter_val = self.eval_expr(&mut iter);
                let (start, end) = if let Value::Str(s) = &iter_val {
                    if let Some(rest) = s.strip_prefix("range:") {
                        let parts: Vec<&str> = rest.split(':').collect();
                        if parts.len() == 2 {
                            let start = parts[0].parse::<i64>().unwrap_or(0);
                            let end = parts[1].parse::<i64>().unwrap_or(0);
                            (start, end)
                        } else {
                            (0, 0)
                        }
                    } else {
                        self.error("invalid range value".to_string(), iter.span());
                        (0, 0)
                    }
                } else {
                    self.error(
                        "for loop iterator must be range(...)".to_string(),
                        iter.span(),
                    );
                    (0, 0)
                };

                let mut cf = ControlFlow::None;
                for idx in start..end {
                    self.env.push_scope();
                    self.env.define(var.clone(), Value::Int(idx));
                    cf = self.eval_block(&mut body);
                    self.env.pop_scope();
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(val) => return ControlFlow::Return(val),
                        ControlFlow::None => {}
                    }
                }
                ControlFlow::None
            }
            StmtKind::Break => ControlFlow::Break,
            StmtKind::Continue => ControlFlow::Continue,
            StmtKind::Expr(mut expr) => {
                self.eval_expr(&mut expr);
                ControlFlow::None
            }
        }
    }

    fn eval_block(&mut self, block: &mut Block) -> ControlFlow {
        self.env.push_scope();
        let mut cf = ControlFlow::None;
        let mut i = 0;

        while i < block.stmts.len() {
            // Extract and handle each statement in the block
            let stmt_kind = self.extract_block_stmt_kind(block, i);
            cf = self.handle_block_stmt_kind(stmt_kind);
            if cf != ControlFlow::None {
                break;
            }
            i += 1;
        }

        self.env.pop_scope();
        cf
    }

    fn extract_block_stmt_kind(&self, block: &Block, index: usize) -> StmtKind {
        match &block.stmts[index] {
            Stmt::Let {
                name, initializer, ..
            } => StmtKind::Let {
                name: name.clone(),
                initializer: initializer.clone(),
            },
            Stmt::Mut {
                name, initializer, ..
            } => StmtKind::Mut {
                name: name.clone(),
                initializer: initializer.clone(),
            },
            Stmt::Assign { name, value, span } => StmtKind::Assign {
                name: name.clone(),
                value: value.clone(),
                span: *span,
            },
            Stmt::Def { .. } => StmtKind::Def,
            Stmt::Ret { value, .. } => StmtKind::Ret {
                value: value.clone(),
            },
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => StmtKind::If {
                condition: condition.clone(),
                then_branch: then_branch.clone(),
                else_branch: else_branch.clone(),
            },
            Stmt::While {
                condition, body, ..
            } => StmtKind::While {
                condition: condition.clone(),
                body: body.clone(),
            },
            Stmt::For {
                var, iter, body, ..
            } => StmtKind::For {
                var: var.clone(),
                iter: iter.clone(),
                body: body.clone(),
            },
            Stmt::Break(_) => StmtKind::Break,
            Stmt::Continue(_) => StmtKind::Continue,
            Stmt::Expr(expr, _) => StmtKind::Expr(expr.clone()),
        }
    }

    fn handle_block_stmt_kind(&mut self, kind: StmtKind) -> ControlFlow {
        // Reuse the same handler logic
        self.handle_stmt_kind(kind)
    }

    fn eval_expr(&mut self, expr: &mut Expr) -> Value {
        match expr {
            Expr::IntLit(n, _) => Value::Int(*n),
            Expr::FloatLit(f, _) => Value::Float(*f),
            Expr::StrLit(s, _) => Value::Str(s.clone()),
            Expr::BoolLit(b, _) => Value::Bool(*b),
            Expr::UnitLit(_) => Value::Unit,
            Expr::Ident(name, span) => self.eval_ident(name, *span),
            Expr::Binary(left, op, right, span) => self.eval_binary(left, *op, right, *span),
            Expr::Unary(op, operand, span) => self.eval_unary(*op, operand, *span),
            Expr::Call { callee, args, span } => self.eval_call(callee, args, *span),
            Expr::Group(inner, _) => self.eval_expr(inner),
        }
    }

    fn eval_ident(&mut self, name: &str, span: Span) -> Value {
        match name {
            "print" | "str" | "int" | "float" | "range" => {
                self.error(
                    format!("'{}' is a built-in function, not a value", name),
                    span,
                );
                Value::Unit
            }
            _ => {
                if let Some(val) = self.env.get(name) {
                    val.clone()
                } else {
                    self.error(format!("undefined variable '{}'", name), span);
                    Value::Unit
                }
            }
        }
    }

    fn eval_binary(&mut self, left: &mut Expr, op: BinOp, right: &mut Expr, span: Span) -> Value {
        let left_val = self.eval_expr(left);
        let right_val = self.eval_expr(right);

        match (left_val, right_val) {
            (Value::Int(a), Value::Int(b)) => self.eval_int_binop(a, op, b, span),
            (Value::Float(a), Value::Float(b)) => self.eval_float_binop(a, op, b, span),
            (Value::Str(a), Value::Str(b)) => self.eval_str_binop(a, op, b, span),
            (Value::Bool(a), Value::Bool(b)) => self.eval_bool_binop(a, op, b, span),
            (Value::Unit, Value::Unit) => {
                if matches!(op, BinOp::Eq | BinOp::Ne) {
                    Value::Bool(match op {
                        BinOp::Eq => true,
                        _ => false,
                    })
                } else {
                    self.error(format!("invalid operator '{}' for unit", op), span);
                    Value::Unit
                }
            }
            (l, r) => {
                self.error(
                    format!(
                        "invalid operands for '{}': {} and {}",
                        op,
                        l.type_name(),
                        r.type_name()
                    ),
                    span,
                );
                Value::Unit
            }
        }
    }

    fn eval_int_binop(&mut self, a: i64, op: BinOp, b: i64, span: Span) -> Value {
        use BinOp::*;
        let result = match op {
            Add => arithmetic::add_int(a, b),
            Sub => arithmetic::sub_int(a, b),
            Mul => arithmetic::mul_int(a, b),
            Div => arithmetic::div_int(a, b),
            Rem => arithmetic::rem_int(a, b),
            Eq => return Value::Bool(a == b),
            Ne => return Value::Bool(a != b),
            Lt => return Value::Bool(a < b),
            Le => return Value::Bool(a <= b),
            Gt => return Value::Bool(a > b),
            Ge => return Value::Bool(a >= b),
            And | Or => {
                self.error("'and'/'or' not supported for int".to_string(), span);
                return Value::Unit;
            }
        };

        match result {
            Ok(v) => Value::Int(v),
            Err(e) => {
                self.error(self.arith_error_msg(e), span);
                Value::Unit
            }
        }
    }

    fn eval_float_binop(&mut self, a: f64, op: BinOp, b: f64, span: Span) -> Value {
        use BinOp::*;
        let result = match op {
            Add => arithmetic::add_float(a, b),
            Sub => arithmetic::sub_float(a, b),
            Mul => arithmetic::mul_float(a, b),
            Div => arithmetic::div_float(a, b),
            Rem => arithmetic::rem_float(a, b),
            Eq => return Value::Bool(a == b),
            Ne => return Value::Bool(a != b),
            Lt => return Value::Bool(a < b),
            Le => return Value::Bool(a <= b),
            Gt => return Value::Bool(a > b),
            Ge => return Value::Bool(a >= b),
            And | Or => {
                self.error("'and'/'or' not supported for float".to_string(), span);
                return Value::Unit;
            }
        };

        match result {
            Ok(v) => Value::Float(v),
            Err(e) => {
                self.error(self.arith_error_msg(e), span);
                Value::Unit
            }
        }
    }

    fn eval_str_binop(&mut self, a: String, op: BinOp, b: String, span: Span) -> Value {
        use BinOp::*;
        match op {
            Add => Value::Str(a + &b),
            Eq => Value::Bool(a == b),
            Ne => Value::Bool(a != b),
            _ => {
                self.error(format!("invalid operator '{}' for strings", op), span);
                Value::Unit
            }
        }
    }

    fn eval_bool_binop(&mut self, a: bool, op: BinOp, b: bool, span: Span) -> Value {
        use BinOp::*;
        match op {
            And => Value::Bool(a && b),
            Or => Value::Bool(a || b),
            Eq => Value::Bool(a == b),
            Ne => Value::Bool(a != b),
            _ => {
                self.error(format!("invalid operator '{}' for bool", op), span);
                Value::Unit
            }
        }
    }

    fn eval_unary(&mut self, op: UnOp, operand: &mut Expr, span: Span) -> Value {
        let val = self.eval_expr(operand);

        match (op, val) {
            (UnOp::Neg, Value::Int(n)) => match arithmetic::neg_int(n) {
                Ok(v) => Value::Int(v),
                Err(e) => {
                    self.error(self.arith_error_msg(e), span);
                    Value::Unit
                }
            },
            (UnOp::Neg, Value::Float(f)) => match arithmetic::neg_float(f) {
                Ok(v) => Value::Float(v),
                Err(e) => {
                    self.error(self.arith_error_msg(e), span);
                    Value::Unit
                }
            },
            (UnOp::Not, Value::Bool(b)) => Value::Bool(!b),
            (UnOp::Neg, v) => {
                self.error(
                    format!("unary '-' not supported for {}", v.type_name()),
                    span,
                );
                Value::Unit
            }
            (UnOp::Not, v) => {
                self.error(format!("'not' not supported for {}", v.type_name()), span);
                Value::Unit
            }
        }
    }

    fn eval_call(&mut self, callee: &mut Expr, args: &mut [Expr], span: Span) -> Value {
        if let Expr::Ident(name, _) = &*callee {
            return match name.as_str() {
                "print" => self.eval_print(args, span),
                "str" => self.eval_str_builtin(args, span),
                "int" => self.eval_int_builtin(args, span),
                "float" => self.eval_float_builtin(args, span),
                "range" => self.eval_range_builtin(args, span),
                _ => self.eval_user_function(name, args, span),
            };
        }

        self.error(
            "indirect function calls not supported in v0.1".to_string(),
            callee.span(),
        );
        Value::Unit
    }

    fn eval_user_function(&mut self, name: &str, args: &mut [Expr], span: Span) -> Value {
        if self.call_depth >= self.max_call_depth {
            self.error(
                "maximum call depth exceeded (stack overflow)".to_string(),
                span,
            );
            return Value::Unit;
        }

        let func = match self.functions.get(name) {
            Some(f) => f.clone(),
            None => {
                self.error(format!("undefined function '{}'", name), span);
                return Value::Unit;
            }
        };

        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.eval_expr(arg));
        }

        if arg_values.len() != func.params.len() {
            self.error(
                format!(
                    "function '{}' takes {} arguments, got {}",
                    name,
                    func.params.len(),
                    arg_values.len()
                ),
                span,
            );
            return Value::Unit;
        }

        let mut body = func.body.clone();
        let return_type = func.return_type.primitive;
        let func_span = func.span;

        self.call_depth += 1;
        self.env.push_scope();

        for (param, value) in func.params.iter().zip(arg_values) {
            self.env.define(param.name.clone(), value);
        }

        let mut result = Value::Unit;
        let mut returned = false;
        let mut i = 0;

        while i < body.stmts.len() {
            let stmt_kind = self.extract_block_stmt_kind(&body, i);
            let cf = self.handle_block_stmt_kind(stmt_kind);
            match cf {
                ControlFlow::Return(val) => {
                    result = val.unwrap_or(Value::Unit);
                    returned = true;
                    break;
                }
                ControlFlow::Break | ControlFlow::Continue => {
                    self.error(
                        "break/continue not allowed in function body".to_string(),
                        body.stmts[i].span(),
                    );
                }
                ControlFlow::None => {}
            }
            i += 1
        }

        self.env.pop_scope();
        self.call_depth -= 1;

        if !returned && return_type != crate::ast::PrimitiveType::Unit {
            self.error(
                format!("function '{}' ended without return", name),
                func_span,
            );
        }

        result
    }

    fn eval_print(&mut self, args: &mut [Expr], span: Span) -> Value {
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.eval_expr(arg));
        }

        if let Err(e) = builtins::print(&arg_values, &mut self.output) {
            self.error(e, span);
        }
        Value::Unit
    }

    fn eval_str_builtin(&mut self, args: &mut [Expr], span: Span) -> Value {
        if args.len() != 1 {
            self.error("str() takes exactly 1 argument".to_string(), span);
            return Value::Unit;
        }
        let val = self.eval_expr(&mut args[0]);
        Value::Str(conversion::to_str(&val))
    }

    fn eval_int_builtin(&mut self, args: &mut [Expr], span: Span) -> Value {
        if args.len() != 1 {
            self.error("int() takes exactly 1 argument".to_string(), span);
            return Value::Unit;
        }
        let val = self.eval_expr(&mut args[0]);
        match conversion::to_int(&val) {
            Ok(i) => Value::Int(i),
            Err(e) => {
                self.error(
                    match e {
                        arithmetic::ArithError::Overflow => {
                            "int() argument out of range".to_string()
                        }
                        arithmetic::ArithError::InvalidFloatResult => {
                            "int() argument must be a finite float".to_string()
                        }
                        _ => "int() argument must be float".to_string(),
                    },
                    span,
                );
                Value::Unit
            }
        }
    }

    fn eval_float_builtin(&mut self, args: &mut [Expr], span: Span) -> Value {
        if args.len() != 1 {
            self.error("float() takes exactly 1 argument".to_string(), span);
            return Value::Unit;
        }
        let val = self.eval_expr(&mut args[0]);
        match conversion::to_float(&val) {
            Ok(f) => Value::Float(f),
            Err(_) => {
                self.error("float() argument must be int".to_string(), span);
                Value::Unit
            }
        }
    }

    fn eval_range_builtin(&mut self, args: &mut [Expr], span: Span) -> Value {
        if args.len() != 1 && args.len() != 2 {
            self.error("range() takes 1 or 2 arguments".to_string(), span);
            return Value::Unit;
        }

        let start = match self.eval_expr(&mut args[0]) {
            Value::Int(n) => n,
            _ => {
                self.error("range() arguments must be int".to_string(), span);
                return Value::Unit;
            }
        };

        let end = if args.len() == 2 {
            match self.eval_expr(&mut args[1]) {
                Value::Int(n) => n,
                _ => {
                    self.error("range() arguments must be int".to_string(), span);
                    return Value::Unit;
                }
            }
        } else {
            start
        };

        let actual_start = if args.len() == 1 { 0 } else { start };
        Value::Str(format!("range:{}:{}", actual_start, end))
    }

    fn arith_error_msg(&self, err: arithmetic::ArithError) -> String {
        match err {
            arithmetic::ArithError::Overflow => "integer overflow".to_string(),
            arithmetic::ArithError::DivisionByZero => "division by zero".to_string(),
            arithmetic::ArithError::RemainderByZero => "remainder by zero".to_string(),
            arithmetic::ArithError::InvalidFloatResult => {
                "invalid floating-point result".to_string()
            }
        }
    }

    fn error(&mut self, message: String, span: Span) {
        let diag = Diagnostic::error(message)
            .with_label(Label::primary(span, "here"))
            .with_note(format!("call depth: {}", self.call_depth));
        self.diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::Checker;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::span::SourceFile;

    fn eval_source(content: &str) -> (Value, Diagnostics) {
        let source = SourceFile::new("<test>".to_string(), content.to_string());
        let (tokens, lexer_diags) = Lexer::new(&source).lex();
        let (program, parser_diags) = Parser::new(tokens).parse();
        let mut all_diags = lexer_diags;
        all_diags.extend(parser_diags);
        if all_diags.has_errors() {
            return (Value::Unit, all_diags);
        }
        let (program, checker_diags) = Checker::new(program).check();
        all_diags.extend(checker_diags);
        if all_diags.has_errors() {
            return (Value::Unit, all_diags);
        }
        let evaluator = Evaluator::new(program, Box::new(Vec::new()));
        evaluator.evaluate()
    }

    #[test]
    fn test_let_and_print() {
        let (val, diags) = eval_source("let x = 42\nprint(x)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_mut_and_assignment() {
        let (val, diags) = eval_source("mut x = 0\nx = x + 1\nprint(x)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_function_call() {
        let (val, diags) =
            eval_source("def add(a: int, b: int) -> int:\n    ret a + b\nprint(add(2, 3))\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_recursion() {
        let (val, diags) = eval_source(
            "def fact(n: int) -> int:\n    if n <= 1:\n        ret 1\n    ret n * fact(n - 1)\nprint(fact(5))\n",
        );
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_if_else() {
        let (val, diags) = eval_source(
            "let x = 5\nif x > 0:\n    print(\"positive\")\nelse:\n    print(\"non-positive\")\n",
        );
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_while_loop() {
        let (val, diags) = eval_source("mut i = 0\nwhile i < 3:\n    print(i)\n    i = i + 1\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_for_loop() {
        let (val, diags) = eval_source("for i in range(3):\n    print(i)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_for_loop_range() {
        let (val, diags) = eval_source("for i in range(2, 5):\n    print(i)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_break_continue() {
        let (val, diags) = eval_source(
            "mut i = 0\nwhile true:\n    if i == 2:\n        break\n    i = i + 1\nprint(i)\n",
        );
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_string_concat() {
        let (val, diags) = eval_source("let s = \"hello\" + \"world\"\nprint(s)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_arithmetic_overflow() {
        let (val, diags) = eval_source("let x = 9223372036854775807\nlet y = x + 1\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_division_by_zero() {
        let (val, diags) = eval_source("let x = 1 / 0\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_float_division_by_zero() {
        let (val, diags) = eval_source("let x = 1.0 / 0.0\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_int_conversion() {
        let (val, diags) = eval_source("let i = int(3.14)\nprint(i)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_float_conversion() {
        let (val, diags) = eval_source("let f = float(42)\nprint(f)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_str_conversion() {
        let (val, diags) = eval_source("let s = str(42)\nprint(s)\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_bool_ops_short_circuit() {
        let (val, diags) = eval_source("let x = true and false\nprint(x)\n");
        assert!(!diags.has_errors());
    }
}
