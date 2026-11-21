use std::collections::HashMap;
use std::fmt;

use crate::ast::{PrimitiveType, TypeAnnotation};
use crate::span::Span;

/// A type in KantoLang.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
}

impl Type {
    pub fn int() -> Self {
        Type::Primitive(PrimitiveType::Int)
    }

    pub fn float() -> Self {
        Type::Primitive(PrimitiveType::Float)
    }

    pub fn bool() -> Self {
        Type::Primitive(PrimitiveType::Bool)
    }

    pub fn str() -> Self {
        Type::Primitive(PrimitiveType::Str)
    }

    pub fn unit() -> Self {
        Type::Primitive(PrimitiveType::Unit)
    }

    pub fn function(params: Vec<Type>, return_type: Type) -> Self {
        Type::Function {
            params,
            return_type: Box::new(return_type),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Primitive(PrimitiveType::Int) | Type::Primitive(PrimitiveType::Float)
        )
    }

    pub fn is_printable(&self) -> bool {
        matches!(
            self,
            Type::Primitive(PrimitiveType::Int)
                | Type::Primitive(PrimitiveType::Float)
                | Type::Primitive(PrimitiveType::Bool)
                | Type::Primitive(PrimitiveType::Str)
                | Type::Primitive(PrimitiveType::Unit)
        )
    }

    pub fn from_annotation(ann: &TypeAnnotation) -> Self {
        Type::Primitive(ann.primitive)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Primitive(p) => write!(f, "{}", p),
            Type::Function {
                params,
                return_type,
            } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
        }
    }
}

/// A symbol in the symbol table.
#[derive(Debug, Clone)]
pub enum Symbol {
    Variable {
        name: String,
        ty: Type,
        mutable: bool,
        initialized: bool,
        span: Span,
    },
    Function {
        name: String,
        ty: Type,
        params: Vec<(String, Type)>,
        return_type: Type,
        span: Span,
        defined: bool,
    },
    Builtin {
        name: String,
        ty: Type,
    },
}

impl Symbol {
    pub fn name(&self) -> &str {
        match self {
            Symbol::Variable { name, .. } => name,
            Symbol::Function { name, .. } => name,
            Symbol::Builtin { name, .. } => name,
        }
    }

    pub fn ty(&self) -> &Type {
        match self {
            Symbol::Variable { ty, .. } => ty,
            Symbol::Function { ty, .. } => ty,
            Symbol::Builtin { ty, .. } => ty,
        }
    }
}

/// Symbol table with lexical scoping.
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = SymbolTable::default();
        table.push_scope();
        table
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, symbol: Symbol) -> Result<(), String> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(symbol.name()) {
            return Err(format!(
                "'{}' already declared in this scope",
                symbol.name()
            ));
        }
        scope.insert(symbol.name().to_string(), symbol);
        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn resolve_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(sym) = scope.get_mut(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn is_defined_in_current_scope(&self, name: &str) -> bool {
        self.scopes.last().unwrap().contains_key(name)
    }

    pub fn current_scope_depth(&self) -> usize {
        self.scopes.len()
    }
}

/// Built-in function signatures.
pub fn builtin_types() -> HashMap<String, Type> {
    let mut map = HashMap::new();
    // print(...)
    map.insert("print".to_string(), Type::function(vec![], Type::unit()));
    // str(value)
    map.insert(
        "str".to_string(),
        Type::function(vec![Type::Primitive(PrimitiveType::Int)], Type::str()),
    );
    // int(number)
    map.insert(
        "int".to_string(),
        Type::function(vec![Type::Primitive(PrimitiveType::Float)], Type::int()),
    );
    // float(number)
    map.insert(
        "float".to_string(),
        Type::function(vec![Type::Primitive(PrimitiveType::Int)], Type::float()),
    );
    // range(stop) and range(start, stop)
    map.insert(
        "range".to_string(),
        Type::function(
            vec![Type::int(), Type::int()],
            Type::Primitive(PrimitiveType::Int), // placeholder, actually returns iterator
        ),
    );
    map
}

/// Check if a type is compatible with another (for assignment, etc.)
pub fn types_compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual
}

/// Check if a value of type `from` can be implicitly converted to type `to`.
/// In v0.1, no implicit conversions are allowed.
pub fn can_implicitly_convert(from: &Type, to: &Type) -> bool {
    from == to
}
