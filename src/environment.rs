use std::collections::HashMap;

use crate::value::Value;

/// Runtime environment for variable bindings.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Environment::default();
        env.push_scope();
        env
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: String, value: Value) {
        let scope = self.scopes.last_mut().unwrap();
        scope.insert(name, value);
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(format!("undefined variable '{}'", name))
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(v) = scope.get_mut(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn is_defined_in_current_scope(&self, name: &str) -> bool {
        self.scopes.last().unwrap().contains_key(name)
    }
}

/// Built-in functions that can be called directly.
pub mod builtins {
    use crate::value::Value;
    use crate::value::arithmetic::ArithError;
    use crate::value::conversion;

    pub fn print(args: &[Value], output: &mut dyn std::io::Write) -> Result<Value, String> {
        let mut first = true;
        for arg in args {
            if !first {
                write!(output, " ").map_err(|e| e.to_string())?;
            }
            first = false;
            write!(output, "{}", conversion::to_str(arg)).map_err(|e| e.to_string())?;
        }
        writeln!(output).map_err(|e| e.to_string())?;
        Ok(Value::Unit)
    }

    pub fn str_builtin(args: &[Value]) -> Result<Value, String> {
        if args.len() != 1 {
            return Err("str() takes exactly 1 argument".to_string());
        }
        Ok(Value::Str(conversion::to_str(&args[0])))
    }

    pub fn int_builtin(args: &[Value]) -> Result<Value, String> {
        if args.len() != 1 {
            return Err("int() takes exactly 1 argument".to_string());
        }
        let i = conversion::to_int(&args[0]).map_err(|e| match e {
            ArithError::Overflow => "int() argument out of range".to_string(),
            ArithError::InvalidFloatResult => "int() argument must be a finite float".to_string(),
            _ => "int() argument must be float".to_string(),
        })?;
        Ok(Value::Int(i))
    }

    pub fn float_builtin(args: &[Value]) -> Result<Value, String> {
        if args.len() != 1 {
            return Err("float() takes exactly 1 argument".to_string());
        }
        let f = conversion::to_float(&args[0]).map_err(|e| match e {
            ArithError::InvalidFloatResult => "float() argument must be int".to_string(),
            _ => "float() argument must be int".to_string(),
        })?;
        Ok(Value::Float(f))
    }

    pub fn range_builtin(args: &[Value]) -> Result<Value, String> {
        if args.len() != 1 && args.len() != 2 {
            return Err("range() takes 1 or 2 arguments".to_string());
        }

        let start = match args.get(0) {
            Some(Value::Int(n)) => *n,
            _ => return Err("range() arguments must be int".to_string()),
        };

        let end = match args.get(1) {
            Some(Value::Int(n)) => *n,
            None => start, // range(stop) -> start=0, end=stop
            _ => return Err("range() arguments must be int".to_string()),
        };

        let actual_start = if args.len() == 1 { 0 } else { start };

        // Return a special range iterator value
        // For simplicity, we'll return a tuple-like structure
        // The evaluator will handle this specially in for loops
        Ok(Value::Str(format!("range:{}:{}", actual_start, end)))
    }
}
