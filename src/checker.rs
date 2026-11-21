use std::collections::HashMap;

use crate::ast::PrimitiveType;
use crate::ast::PrimitiveType as PT;
use crate::ast::{BinOp, Block, Expr, Param, Program, Stmt, TypeAnnotation, UnOp};
use crate::diagnostic::{Diagnostic, Diagnostics, Label, Severity};
use crate::span::Span;
use crate::types::{Symbol, SymbolTable, Type, builtin_types, types_compatible};

/// Type checker for KantoLang.
pub struct Checker {
    program: Program,
    symbols: SymbolTable,
    diagnostics: Diagnostics,
    current_function_return_type: Option<Type>,
    in_loop: bool,
    function_signatures: HashMap<String, Type>, // For forward references
}

impl Checker {
    pub fn new(program: Program) -> Self {
        let mut checker = Checker {
            program,
            symbols: SymbolTable::new(),
            diagnostics: Diagnostics::new(),
            current_function_return_type: None,
            in_loop: false,
            function_signatures: HashMap::new(),
        };
        checker.declare_builtins();
        checker
    }

    pub fn check(mut self) -> (Program, Diagnostics) {
        // First pass: collect function signatures
        self.collect_function_signatures();

        // Second pass: check everything
        self.check_program();

        (self.program, self.diagnostics)
    }

    fn declare_builtins(&mut self) {
        for (name, ty) in builtin_types() {
            let symbol = Symbol::Builtin {
                name: name.clone(),
                ty,
            };
            self.symbols.declare(symbol).unwrap();
        }
    }

    fn collect_function_signatures(&mut self) {
        let mut i = 0;
        while i < self.program.stmts.len() {
            if let Stmt::Def {
                name,
                params,
                return_type,
                span,
                ..
            } = &self.program.stmts[i]
            {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| Type::from_annotation(&p.type_ann))
                    .collect();
                let ret_type = Type::from_annotation(return_type);
                let fn_type = Type::function(param_types.clone(), ret_type.clone());
                self.function_signatures
                    .insert(name.clone(), fn_type.clone());

                let symbol = Symbol::Function {
                    name: name.clone(),
                    ty: fn_type,
                    params: params
                        .iter()
                        .map(|p| (p.name.clone(), Type::from_annotation(&p.type_ann)))
                        .collect(),
                    return_type: ret_type,
                    span: *span,
                    defined: true,
                };
                if let Err(e) = self.symbols.declare(symbol) {
                    self.error(e, *span);
                }
            }
            i += 1;
        }
    }

    fn check_program(&mut self) {
        let mut i = 0;
        while i < self.program.stmts.len() {
            // Use a helper that takes the statement by value (extracted/cloned)
            let stmt = self.program.stmts[i].clone();
            self.check_stmt_owned(stmt);
            i += 1;
        }
    }

    /// Check a statement that we own (no borrow issues)
    fn check_stmt_owned(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                initializer,
                span,
            } => {
                let mut init = initializer;
                self.check_let(&name, &type_ann, &mut init, span);
            }
            Stmt::Mut {
                name,
                type_ann,
                initializer,
                span,
            } => {
                let mut init = initializer;
                self.check_mut(&name, &type_ann, &mut init, span);
            }
            Stmt::Assign { name, value, span } => {
                let mut val = value;
                self.check_assign(&name, &mut val, span);
            }
            Stmt::Def {
                name,
                mut params,
                return_type,
                mut body,
                span,
            } => {
                self.check_def(&name, &mut params, &return_type, &mut body, span);
            }
            Stmt::Ret { value, span } => {
                let mut val = value;
                self.check_ret(&mut val, span);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let mut cond = condition;
                let mut then_b = then_branch;
                let mut else_b = else_branch;
                self.check_if(&mut cond, &mut then_b, &mut else_b, span);
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                let mut cond = condition;
                let mut b = body;
                self.check_while(&mut cond, &mut b, span);
            }
            Stmt::For {
                var,
                iter,
                body,
                span,
            } => {
                let mut it = iter;
                let mut b = body;
                self.check_for(&var, &mut it, &mut b, span);
            }
            Stmt::Break(span) => {
                self.check_break(span);
            }
            Stmt::Continue(span) => {
                self.check_continue(span);
            }
            Stmt::Expr(expr, _span) => {
                let mut e = expr;
                self.check_expr(&mut e);
            }
        }
    }

    fn check_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                initializer,
                span,
            } => {
                self.check_let(name, type_ann, initializer, *span);
            }
            Stmt::Mut {
                name,
                type_ann,
                initializer,
                span,
            } => {
                self.check_mut(name, type_ann, initializer, *span);
            }
            Stmt::Assign { name, value, span } => {
                self.check_assign(name, value, *span);
            }
            Stmt::Def {
                name,
                params,
                return_type,
                body,
                span,
            } => {
                self.check_def(name, params, return_type, body, *span);
            }
            Stmt::Ret { value, span } => {
                self.check_ret(value, *span);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                self.check_if(condition, then_branch, else_branch, *span);
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                self.check_while(condition, body, *span);
            }
            Stmt::For {
                var,
                iter,
                body,
                span,
            } => {
                self.check_for(var, iter, body, *span);
            }
            Stmt::Break(span) => {
                self.check_break(*span);
            }
            Stmt::Continue(span) => {
                self.check_continue(*span);
            }
            Stmt::Expr(expr, span) => {
                self.check_expr(expr);
                // Top-level expression statements are allowed but their value is discarded
            }
        }
    }

    fn check_let(
        &mut self,
        name: &str,
        type_ann: &Option<TypeAnnotation>,
        initializer: &mut Expr,
        span: Span,
    ) {
        if self.symbols.is_defined_in_current_scope(name) {
            self.error(format!("'{}' already declared in this scope", name), span);
            return;
        }

        // Check initializer
        let init_type = self.check_expr(initializer);

        // Check type annotation if present
        let declared_type = type_ann.as_ref().map(Type::from_annotation);

        let final_type = match (declared_type, init_type) {
            (Some(ann), Some(init)) => {
                if !types_compatible(&ann, &init) {
                    self.error(
                        format!("type mismatch: expected {}, found {}", ann, init),
                        initializer.span(),
                    );
                }
                Some(ann)
            }
            (Some(ann), None) => Some(ann),
            (None, Some(init)) => Some(init),
            (None, None) => {
                self.error(
                    "cannot infer type of variable without initializer or annotation".to_string(),
                    span,
                );
                None
            }
        };

        if let Some(ty) = final_type {
            let symbol = Symbol::Variable {
                name: name.to_string(),
                ty,
                mutable: false,
                initialized: true,
                span,
            };
            self.symbols.declare(symbol).unwrap();
        }
    }

    fn check_mut(
        &mut self,
        name: &str,
        type_ann: &Option<TypeAnnotation>,
        initializer: &mut Expr,
        span: Span,
    ) {
        if self.symbols.is_defined_in_current_scope(name) {
            self.error(format!("'{}' already declared in this scope", name), span);
            return;
        }

        let init_type = self.check_expr(initializer);
        let declared_type = type_ann.as_ref().map(Type::from_annotation);

        let final_type = match (declared_type, init_type) {
            (Some(ann), Some(init)) => {
                if !types_compatible(&ann, &init) {
                    self.error(
                        format!("type mismatch: expected {}, found {}", ann, init),
                        initializer.span(),
                    );
                }
                Some(ann)
            }
            (Some(ann), None) => Some(ann),
            (None, Some(init)) => Some(init),
            (None, None) => {
                self.error(
                    "cannot infer type of variable without initializer or annotation".to_string(),
                    span,
                );
                None
            }
        };

        if let Some(ty) = final_type {
            let symbol = Symbol::Variable {
                name: name.to_string(),
                ty,
                mutable: true,
                initialized: true,
                span,
            };
            self.symbols.declare(symbol).unwrap();
        }
    }

    fn check_assign(&mut self, name: &str, value: &mut Expr, span: Span) {
        let value_type = self.check_expr(value);

        // Extract info we need first
        let (is_mutable, var_type, is_function) = {
            if let Some(symbol) = self.symbols.resolve_mut(name) {
                match symbol {
                    Symbol::Variable {
                        mutable,
                        ty,
                        initialized,
                        ..
                    } => {
                        *initialized = true;
                        (*mutable, ty.clone(), false)
                    }
                    Symbol::Function { .. } | Symbol::Builtin { .. } => (false, Type::unit(), true),
                }
            } else {
                self.error(format!("undefined variable '{}'", name), span);
                return;
            }
        };

        if is_function {
            self.error(format!("cannot assign to function '{}'", name), span);
            return;
        }

        if !is_mutable {
            self.error(
                format!("cannot assign to immutable variable '{}'", name),
                span,
            );
        }

        if let Some(vt) = value_type {
            if !types_compatible(&var_type, &vt) {
                self.error(
                    format!("type mismatch: expected {}, found {}", var_type, vt),
                    value.span(),
                );
            }
        }
    }

    fn check_def(
        &mut self,
        name: &str,
        params: &mut [Param],
        return_type: &TypeAnnotation,
        body: &mut Block,
        span: Span,
    ) {
        let ret_type = Type::from_annotation(return_type);
        let old_return_type = self.current_function_return_type.replace(ret_type.clone());

        self.symbols.push_scope();

        // Declare parameters
        for param in params {
            let param_type = Type::from_annotation(&param.type_ann);
            let symbol = Symbol::Variable {
                name: param.name.clone(),
                ty: param_type,
                mutable: false, // Parameters are immutable in v0.1
                initialized: true,
                span: param.type_ann.span,
            };
            self.symbols.declare(symbol).unwrap();
        }

        // Check body
        self.check_block(body);

        // Check return path analysis for non-unit functions
        if ret_type != Type::unit() {
            self.check_all_paths_return(body, span);
        }

        self.symbols.pop_scope();
        self.current_function_return_type = old_return_type;
    }

    fn check_all_paths_return(&mut self, block: &Block, fn_span: Span) {
        if !self.block_always_returns(block) {
            self.error("function may return without a value".to_string(), fn_span);
        }
    }

    fn block_always_returns(&self, block: &Block) -> bool {
        // Conservative analysis: a block always returns if its last statement
        // is a return, or if it's an if/else where both branches return,
        // or a loop that always returns (not handled in v0.1)
        let stmts = &block.stmts;
        if stmts.is_empty() {
            return false;
        }

        match &stmts[stmts.len() - 1] {
            Stmt::Ret { .. } => true,
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                else_branch.is_some()
                    && self.block_always_returns(then_branch)
                    && self.block_always_returns(else_branch.as_ref().unwrap())
            }
            _ => false,
        }
    }

    fn check_ret(&mut self, value: &mut Option<Expr>, span: Span) {
        let expected = self
            .current_function_return_type
            .clone()
            .unwrap_or(Type::unit());

        if let Some(expr) = value {
            let actual = self.check_expr(expr);
            if let Some(at) = actual {
                if !types_compatible(&expected, &at) {
                    self.error(
                        format!("return type mismatch: expected {}, found {}", expected, at),
                        expr.span(),
                    );
                }
            }
        } else {
            // No return value - only valid for unit functions
            if expected != Type::unit() {
                self.error(format!("missing return value: expected {}", expected), span);
            }
        }
    }

    fn check_if(
        &mut self,
        condition: &mut Expr,
        then_branch: &mut Block,
        else_branch: &mut Option<Block>,
        span: Span,
    ) {
        let cond_type = self.check_expr(condition);
        if let Some(ct) = cond_type {
            if ct != Type::bool() {
                self.error(
                    format!("if condition must be bool, found {}", ct),
                    condition.span(),
                );
            }
        }

        self.check_block(then_branch);
        if let Some(else_b) = else_branch {
            self.check_block(else_b);
        }
    }

    fn check_while(&mut self, condition: &mut Expr, body: &mut Block, span: Span) {
        let cond_type = self.check_expr(condition);
        if let Some(ct) = cond_type {
            if ct != Type::bool() {
                self.error(
                    format!("while condition must be bool, found {}", ct),
                    condition.span(),
                );
            }
        }

        let old_in_loop = self.in_loop;
        self.in_loop = true;
        self.check_block(body);
        self.in_loop = old_in_loop;
    }

    fn check_for(&mut self, var: &str, iter: &mut Expr, body: &mut Block, span: Span) {
        // Check that iter is a range(...) call
        if let Expr::Call { callee, args, .. } = iter {
            if let Expr::Ident(name, _) = &**callee {
                if name != "range" {
                    self.error(
                        "for loop iterator must be range(...)".to_string(),
                        iter.span(),
                    );
                    return;
                }
            } else {
                self.error(
                    "for loop iterator must be range(...)".to_string(),
                    iter.span(),
                );
                return;
            }

            // Check arguments: range(stop) or range(start, stop)
            if args.len() != 1 && args.len() != 2 {
                self.error("range() takes 1 or 2 arguments".to_string(), iter.span());
                return;
            }

            for arg in args {
                let arg_type = self.check_expr(arg);
                if let Some(at) = arg_type {
                    if at != Type::int() {
                        self.error(
                            format!("range() arguments must be int, found {}", at),
                            arg.span(),
                        );
                    }
                }
            }
        } else {
            self.error(
                "for loop iterator must be range(...)".to_string(),
                iter.span(),
            );
            return;
        }

        // Loop variable is immutable and local to the loop
        self.symbols.push_scope();
        let symbol = Symbol::Variable {
            name: var.to_string(),
            ty: Type::int(),
            mutable: false,
            initialized: true,
            span,
        };
        self.symbols.declare(symbol).unwrap();

        let old_in_loop = self.in_loop;
        self.in_loop = true;
        self.check_block(body);
        self.in_loop = old_in_loop;

        self.symbols.pop_scope();
    }

    fn check_break(&mut self, span: Span) {
        if !self.in_loop {
            self.error("break outside of loop".to_string(), span);
        }
    }

    fn check_continue(&mut self, span: Span) {
        if !self.in_loop {
            self.error("continue outside of loop".to_string(), span);
        }
    }

    fn check_block(&mut self, block: &mut Block) {
        self.symbols.push_scope();
        for stmt in &mut block.stmts {
            self.check_stmt(stmt);
        }
        self.symbols.pop_scope();
    }

    fn check_expr(&mut self, expr: &mut Expr) -> Option<Type> {
        match expr {
            Expr::IntLit(_, _) => Some(Type::int()),
            Expr::FloatLit(_, _) => Some(Type::float()),
            Expr::StrLit(_, _) => Some(Type::str()),
            Expr::BoolLit(_, _) => Some(Type::bool()),
            Expr::UnitLit(_) => Some(Type::unit()),
            Expr::Ident(name, span) => self.check_ident(name, *span),
            Expr::Binary(left, op, right, span) => self.check_binary(left, *op, right, *span),
            Expr::Unary(op, operand, span) => self.check_unary(*op, operand, *span),
            Expr::Call { callee, args, span } => self.check_call(callee, args, *span),
            Expr::Group(inner, _) => self.check_expr(inner),
        }
    }

    fn check_ident(&mut self, name: &str, span: Span) -> Option<Type> {
        // Check for built-in functions that can be called directly
        if name == "print" || name == "str" || name == "int" || name == "float" || name == "range" {
            if let Some(sym) = self.symbols.resolve(name) {
                return Some(sym.ty().clone());
            }
        }

        // Extract type info first to avoid borrow issues
        let result = if let Some(symbol) = self.symbols.resolve(name) {
            match symbol {
                Symbol::Variable {
                    ty, initialized, ..
                } => {
                    let ty = ty.clone();
                    let uninit = !initialized;
                    if uninit {
                        self.warning(
                            format!("variable '{}' used before initialization", name),
                            span,
                        );
                    }
                    Some(ty)
                }
                Symbol::Function { ty, .. } => Some(ty.clone()),
                Symbol::Builtin { ty, .. } => Some(ty.clone()),
            }
        } else {
            self.error(format!("undefined variable '{}'", name), span);
            None
        };
        result
    }

    fn check_binary(
        &mut self,
        left: &mut Expr,
        op: BinOp,
        right: &mut Expr,
        span: Span,
    ) -> Option<Type> {
        let left_type = self.check_expr(left);
        let right_type = self.check_expr(right);

        match (left_type, right_type) {
            (Some(lt), Some(rt)) => {
                let result = self.check_binary_op(&lt, op, &rt, span);
                if result.is_none() {
                    self.error(
                        format!("invalid operands for '{}': {} and {}", op, lt, rt),
                        span,
                    );
                }
                result
            }
            _ => None,
        }
    }

    fn check_binary_op(&self, left: &Type, op: BinOp, right: &Type, span: Span) -> Option<Type> {
        use BinOp::*;

        match (left, right) {
            (Type::Primitive(PT::Int), Type::Primitive(PT::Int)) => {
                match op {
                    Add | Sub | Mul | Div | Rem => Some(Type::int()),
                    Eq | Ne | Lt | Le | Gt | Ge => Some(Type::bool()),
                    And | Or => None, // bool only
                }
            }
            (Type::Primitive(PT::Float), Type::Primitive(PT::Float)) => match op {
                Add | Sub | Mul | Div | Rem => Some(Type::float()),
                Eq | Ne | Lt | Le | Gt | Ge => Some(Type::bool()),
                And | Or => None,
            },
            (Type::Primitive(PT::Int), Type::Primitive(PT::Float))
            | (Type::Primitive(PT::Float), Type::Primitive(PT::Int)) => {
                // No implicit int->float conversion in v0.1
                None
            }
            (Type::Primitive(PT::Str), Type::Primitive(PT::Str)) => {
                match op {
                    Add => Some(Type::str()), // String concatenation
                    Eq | Ne => Some(Type::bool()),
                    _ => None,
                }
            }
            (Type::Primitive(PT::Bool), Type::Primitive(PT::Bool)) => match op {
                And | Or => Some(Type::bool()),
                Eq | Ne => Some(Type::bool()),
                _ => None,
            },
            (Type::Primitive(t), Type::Primitive(u)) if t == u => match op {
                Eq | Ne => Some(Type::bool()),
                _ => None,
            },
            _ => None,
        }
    }

    fn check_unary(&mut self, op: UnOp, operand: &mut Expr, span: Span) -> Option<Type> {
        let operand_type = self.check_expr(operand);

        match (op, operand_type) {
            (UnOp::Neg, Some(Type::Primitive(PrimitiveType::Int))) => Some(Type::int()),
            (UnOp::Neg, Some(Type::Primitive(PrimitiveType::Float))) => Some(Type::float()),
            (UnOp::Not, Some(Type::Primitive(PrimitiveType::Bool))) => Some(Type::bool()),
            (UnOp::Neg, Some(t)) => {
                self.error(format!("unary '-' not supported for type {}", t), span);
                None
            }
            (UnOp::Not, Some(t)) => {
                self.error(format!("'not' not supported for type {}", t), span);
                None
            }
            _ => None,
        }
    }

    fn check_call(&mut self, callee: &mut Expr, args: &mut [Expr], span: Span) -> Option<Type> {
        // First check if callee is a direct identifier for built-ins
        if let Expr::Ident(name, _) = &*callee {
            if name == "print" {
                return self.check_print_call(args, span);
            }
            if name == "str" {
                return self.check_str_call(args, span);
            }
            if name == "int" {
                return self.check_int_call(args, span);
            }
            if name == "float" {
                return self.check_float_call(args, span);
            }
            if name == "range" {
                return self.check_range_call(args, span);
            }
        }

        // General function call
        let callee_type = self.check_expr(callee);

        if let Some(Type::Function {
            params,
            return_type,
        }) = callee_type
        {
            if args.len() != params.len() {
                self.error(
                    format!(
                        "function takes {} arguments, but {} were provided",
                        params.len(),
                        args.len()
                    ),
                    span,
                );
                return Some(*return_type);
            }

            for (i, (arg, param_type)) in args.iter_mut().zip(params.iter()).enumerate() {
                let arg_type = self.check_expr(arg);
                if let Some(at) = arg_type {
                    if !types_compatible(param_type, &at) {
                        self.error(
                            format!("argument {}: expected {}, found {}", i + 1, param_type, at),
                            arg.span(),
                        );
                    }
                }
            }

            Some(*return_type)
        } else if callee_type.is_some() {
            self.error("value is not callable".to_string(), callee.span());
            None
        } else {
            None
        }
    }

    fn check_print_call(&mut self, args: &mut [Expr], span: Span) -> Option<Type> {
        for arg in args {
            let arg_type = self.check_expr(arg);
            if let Some(at) = arg_type {
                if !at.is_printable() {
                    self.error(format!("cannot print value of type {}", at), arg.span());
                }
            }
        }
        Some(Type::unit())
    }

    fn check_str_call(&mut self, args: &mut [Expr], span: Span) -> Option<Type> {
        if args.len() != 1 {
            self.error("str() takes exactly 1 argument".to_string(), span);
            return Some(Type::str());
        }
        let arg_type = self.check_expr(&mut args[0]);
        if let Some(at) = arg_type {
            // In v0.1, str() only accepts int
            if at != Type::int() {
                self.error(
                    format!("str() argument must be int, found {}", at),
                    args[0].span(),
                );
            }
        }
        Some(Type::str())
    }

    fn check_int_call(&mut self, args: &mut [Expr], span: Span) -> Option<Type> {
        if args.len() != 1 {
            self.error("int() takes exactly 1 argument".to_string(), span);
            return Some(Type::int());
        }
        let arg_type = self.check_expr(&mut args[0]);
        if let Some(at) = arg_type {
            if at != Type::float() {
                self.error(
                    format!("int() argument must be float, found {}", at),
                    args[0].span(),
                );
            }
        }
        Some(Type::int())
    }

    fn check_float_call(&mut self, args: &mut [Expr], span: Span) -> Option<Type> {
        if args.len() != 1 {
            self.error("float() takes exactly 1 argument".to_string(), span);
            return Some(Type::float());
        }
        let arg_type = self.check_expr(&mut args[0]);
        if let Some(at) = arg_type {
            if at != Type::int() {
                self.error(
                    format!("float() argument must be int, found {}", at),
                    args[0].span(),
                );
            }
        }
        Some(Type::float())
    }

    fn check_range_call(&mut self, args: &mut [Expr], span: Span) -> Option<Type> {
        if args.len() != 1 && args.len() != 2 {
            self.error("range() takes 1 or 2 arguments".to_string(), span);
            return Some(Type::Primitive(PrimitiveType::Int));
        }
        for arg in args {
            let arg_type = self.check_expr(arg);
            if let Some(at) = arg_type {
                if at != Type::int() {
                    self.error(
                        format!("range() arguments must be int, found {}", at),
                        arg.span(),
                    );
                }
            }
        }
        // range returns an iterator-like thing, but for v0.1 we just use it in for loops
        Some(Type::Primitive(PrimitiveType::Int))
    }

    fn error(&mut self, message: String, span: Span) {
        let diag = Diagnostic::error(message).with_label(Label::primary(span, "here"));
        self.diagnostics.push(diag);
    }

    fn warning(&mut self, message: String, span: Span) {
        let diag = Diagnostic::warning(message).with_label(Label::primary(span, "here"));
        self.diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::span::SourceFile;

    fn check_source(content: &str) -> Diagnostics {
        let source = SourceFile::new("<test>".to_string(), content.to_string());
        let (tokens, lexer_diags) = Lexer::new(&source).lex();
        let (program, parser_diags) = Parser::new(tokens).parse();
        let mut all_diags = lexer_diags;
        all_diags.extend(parser_diags);
        if !all_diags.has_errors() {
            let (_, checker_diags) = Checker::new(program).check();
            all_diags.extend(checker_diags);
        }
        all_diags
    }

    #[test]
    fn test_let_type_inference() {
        let diags = check_source("let x = 42\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_mut_type_annotation() {
        let diags = check_source("mut x: int = 0\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_type_mismatch() {
        let diags = check_source("let x: int = 3.14\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_assignment_to_immutable() {
        let diags = check_source("let x = 1\nx = 2\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_assignment_type_mismatch() {
        let diags = check_source("mut x: int = 0\nx = 3.14\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_undefined_variable() {
        let diags = check_source("x = 1\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_function_type_checking() {
        let diags = check_source("def add(a: int, b: int) -> int:\n    ret a + b\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_function_return_type_mismatch() {
        let diags = check_source("def f() -> int:\n    ret 3.14\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_missing_return() {
        let diags = check_source("def f() -> int:\n    let x = 1\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_unit_return() {
        let diags = check_source("def f() -> unit:\n    ret\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_if_condition_type() {
        let diags = check_source("if 1:\n    print(1)\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_while_condition_type() {
        let diags = check_source("while 1:\n    break\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_break_outside_loop() {
        let diags = check_source("break\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_continue_outside_loop() {
        let diags = check_source("continue\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_operator_type_checking() {
        let diags = check_source("let x = 1 + 2\n");
        assert!(!diags.has_errors());

        let diags = check_source(r#"let x = "a" + "b""#);
        assert!(!diags.has_errors());

        let diags = check_source("let x = 1 + 3.14\n");
        assert!(diags.has_errors()); // No implicit int->float
    }

    #[test]
    fn test_string_concat() {
        let diags = check_source(r#"let x = "hello" + "world""#);
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_bool_ops() {
        let diags = check_source("let x = true and false\n");
        assert!(!diags.has_errors());

        let diags = check_source("let x = 1 and 2\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_comparison() {
        let diags = check_source("let x = 1 == 2\n");
        assert!(!diags.has_errors());

        let diags = check_source("let x = 1 < 2\n");
        assert!(!diags.has_errors());

        let diags = check_source("let x = 1 < 3.14\n");
        assert!(diags.has_errors());
    }

    #[test]
    fn test_shadowing() {
        let diags = check_source("let x = 1\nlet x = 2\n");
        assert!(diags.has_errors()); // Same scope

        let diags = check_source("let x = 1\nif true:\n    let x = 2\n");
        assert!(!diags.has_errors()); // Nested scope
    }

    #[test]
    fn test_print_args() {
        let diags = check_source("print(1, 2, 3)\n");
        assert!(!diags.has_errors());

        let diags = check_source("print()\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_str_conversion() {
        let diags = check_source("let s = str(42)\n");
        assert!(!diags.has_errors());

        let diags = check_source("let s = str(3.14)\n");
        // str() only takes int in v0.1
        assert!(diags.has_errors());
    }

    #[test]
    fn test_int_conversion() {
        let diags = check_source("let i = int(3.14)\n");
        assert!(!diags.has_errors());

        let diags = check_source("let i = int(42)\n");
        assert!(diags.has_errors()); // int() only takes float
    }

    #[test]
    fn test_float_conversion() {
        let diags = check_source("let f = float(42)\n");
        assert!(!diags.has_errors());

        let diags = check_source("let f = float(3.14)\n");
        assert!(diags.has_errors()); // float() only takes int
    }

    #[test]
    fn test_range() {
        let diags = check_source("for i in range(3):\n    print(i)\n");
        assert!(!diags.has_errors());

        let diags = check_source("for i in range(1, 3):\n    print(i)\n");
        assert!(!diags.has_errors());

        let diags = check_source("for i in range(1.5):\n    print(i)\n");
        assert!(diags.has_errors());
    }
}
