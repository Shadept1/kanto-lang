use std::fmt;

use crate::span::Span;

/// A unique identifier for AST nodes (for debugging and analysis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub fn new(id: usize) -> Self {
        NodeId(id)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Primitive types in KantoLang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Int,
    Float,
    Bool,
    Str,
    Unit,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::Float => write!(f, "float"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Str => write!(f, "str"),
            PrimitiveType::Unit => write!(f, "unit"),
        }
    }
}

/// Type annotation in the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAnnotation {
    pub primitive: PrimitiveType,
    pub span: Span,
}

impl TypeAnnotation {
    pub fn new(primitive: PrimitiveType, span: Span) -> Self {
        TypeAnnotation { primitive, span }
    }
}

impl fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.primitive)
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Rem => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Ne => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Le => write!(f, "<="),
            BinOp::Gt => write!(f, ">"),
            BinOp::Ge => write!(f, ">="),
            BinOp::And => write!(f, "and"),
            BinOp::Or => write!(f, "or"),
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnOp::Neg => write!(f, "-"),
            UnOp::Not => write!(f, "not"),
        }
    }
}

/// Expressions in KantoLang.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal
    IntLit(i64, Span),
    /// Floating-point literal
    FloatLit(f64, Span),
    /// String literal
    StrLit(String, Span),
    /// Boolean literal
    BoolLit(bool, Span),
    /// Unit literal
    UnitLit(Span),
    /// Variable reference
    Ident(String, Span),
    /// Binary operation
    Binary(Box<Expr>, BinOp, Box<Expr>, Span),
    /// Unary operation
    Unary(UnOp, Box<Expr>, Span),
    /// Function call
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Parenthesized expression (for parsing)
    Group(Box<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s) => *s,
            Expr::FloatLit(_, s) => *s,
            Expr::StrLit(_, s) => *s,
            Expr::BoolLit(_, s) => *s,
            Expr::UnitLit(s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::Binary(_, _, _, s) => *s,
            Expr::Unary(_, _, s) => *s,
            Expr::Call { span, .. } => *span,
            Expr::Group(_, s) => *s,
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::IntLit(n, _) => write!(f, "{}", n),
            Expr::FloatLit(n, _) => write!(f, "{}", n),
            Expr::StrLit(s, _) => write!(f, "\"{}\"", s),
            Expr::BoolLit(b, _) => write!(f, "{}", b),
            Expr::UnitLit(_) => write!(f, "()"),
            Expr::Ident(name, _) => write!(f, "{}", name),
            Expr::Binary(l, op, r, _) => write!(f, "({} {} {})", l, op, r),
            Expr::Unary(op, e, _) => write!(f, "({}{})", op, e),
            Expr::Call { callee, args, .. } => {
                write!(f, "{}(", callee)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expr::Group(e, _) => write!(f, "({})", e),
        }
    }
}

/// Statements in KantoLang.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Immutable binding: let name: type = expr
    Let {
        name: String,
        type_ann: Option<TypeAnnotation>,
        initializer: Expr,
        span: Span,
    },
    /// Mutable binding: mut name: type = expr
    Mut {
        name: String,
        type_ann: Option<TypeAnnotation>,
        initializer: Expr,
        span: Span,
    },
    /// Assignment: name = expr
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    /// Function definition
    Def {
        name: String,
        params: Vec<Param>,
        return_type: TypeAnnotation,
        body: Block,
        span: Span,
    },
    /// Return statement
    Ret { value: Option<Expr>, span: Span },
    /// If statement
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    /// While loop
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    /// For loop
    For {
        var: String,
        iter: Expr, // Must be range(...) call
        body: Block,
        span: Span,
    },
    /// Break statement
    Break(Span),
    /// Continue statement
    Continue(Span),
    /// Expression statement (for top-level expressions)
    Expr(Expr, Span),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Mut { span, .. } => *span,
            Stmt::Assign { span, .. } => *span,
            Stmt::Def { span, .. } => *span,
            Stmt::Ret { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Break(s) => *s,
            Stmt::Continue(s) => *s,
            Stmt::Expr(_, s) => *s,
        }
    }
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Let {
                name,
                type_ann,
                initializer,
                ..
            } => {
                write!(f, "let {}", name)?;
                if let Some(ann) = type_ann {
                    write!(f, ": {}", ann)?;
                }
                write!(f, " = {}", initializer)
            }
            Stmt::Mut {
                name,
                type_ann,
                initializer,
                ..
            } => {
                write!(f, "mut {}", name)?;
                if let Some(ann) = type_ann {
                    write!(f, ": {}", ann)?;
                }
                write!(f, " = {}", initializer)
            }
            Stmt::Assign { name, value, .. } => write!(f, "{} = {}", name, value),
            Stmt::Def {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                write!(f, "def {}({}", name, return_type)?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}: {}", return_type, body)
            }
            Stmt::Ret { value, .. } => {
                if let Some(v) = value {
                    write!(f, "ret {}", v)
                } else {
                    write!(f, "ret")
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                write!(f, "if {}: {}", condition, then_branch)?;
                if let Some(else_b) = else_branch {
                    write!(f, " else: {}", else_b)?;
                }
                Ok(())
            }
            Stmt::While {
                condition, body, ..
            } => write!(f, "while {}: {}", condition, body),
            Stmt::For {
                var, iter, body, ..
            } => write!(f, "for {} in {}: {}", var, iter, body),
            Stmt::Break(_) => write!(f, "break"),
            Stmt::Continue(_) => write!(f, "continue"),
            Stmt::Expr(e, _) => write!(f, "{}", e),
        }
    }
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.type_ann)
    }
}

/// A block of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>, span: Span) -> Self {
        Block { stmts, span }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt in &self.stmts {
            write!(f, "{}", stmt)?;
        }
        Ok(())
    }
}

/// Top-level program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

impl Program {
    pub fn new(stmts: Vec<Stmt>, span: Span) -> Self {
        Program { stmts, span }
    }
}
