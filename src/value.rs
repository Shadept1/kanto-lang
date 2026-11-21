use std::fmt;

/// Runtime values in KantoLang.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    Function {
        name: String,
        param_names: Vec<String>,
        // We'll store the body as an index or reference
    },
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "str",
            Value::Unit => "unit",
            Value::Function { .. } => "function",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => false, // No implicit truthiness
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => {
                // Format floats nicely: avoid scientific notation for simple values
                if n.fract() == 0.0 && *n >= -9007199254740992.0 && *n <= 9007199254740992.0 {
                    write!(f, "{}.0", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Str(s) => write!(f, "{}", s),
            Value::Unit => write!(f, "()"),
            Value::Function { name, .. } => write!(f, "<function {}>", name),
        }
    }
}

/// Arithmetic operations with overflow checking.
pub mod arithmetic {
    use super::Value;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ArithError {
        Overflow,
        DivisionByZero,
        RemainderByZero,
        InvalidFloatResult,
    }

    pub fn add_int(a: i64, b: i64) -> Result<i64, ArithError> {
        a.checked_add(b).ok_or(ArithError::Overflow)
    }

    pub fn sub_int(a: i64, b: i64) -> Result<i64, ArithError> {
        a.checked_sub(b).ok_or(ArithError::Overflow)
    }

    pub fn mul_int(a: i64, b: i64) -> Result<i64, ArithError> {
        a.checked_mul(b).ok_or(ArithError::Overflow)
    }

    pub fn div_int(a: i64, b: i64) -> Result<i64, ArithError> {
        if b == 0 {
            return Err(ArithError::DivisionByZero);
        }
        // Check for MIN / -1 overflow
        if a == i64::MIN && b == -1 {
            return Err(ArithError::Overflow);
        }
        Ok(a / b)
    }

    pub fn rem_int(a: i64, b: i64) -> Result<i64, ArithError> {
        if b == 0 {
            return Err(ArithError::RemainderByZero);
        }
        // Check for MIN % -1 overflow (though remainder is 0, some languages panic)
        if a == i64::MIN && b == -1 {
            return Err(ArithError::Overflow);
        }
        Ok(a % b)
    }

    pub fn neg_int(a: i64) -> Result<i64, ArithError> {
        if a == i64::MIN {
            return Err(ArithError::Overflow);
        }
        Ok(-a)
    }

    pub fn add_float(a: f64, b: f64) -> Result<f64, ArithError> {
        let result = a + b;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ArithError::InvalidFloatResult)
        }
    }

    pub fn sub_float(a: f64, b: f64) -> Result<f64, ArithError> {
        let result = a - b;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ArithError::InvalidFloatResult)
        }
    }

    pub fn mul_float(a: f64, b: f64) -> Result<f64, ArithError> {
        let result = a * b;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ArithError::InvalidFloatResult)
        }
    }

    pub fn div_float(a: f64, b: f64) -> Result<f64, ArithError> {
        if b == 0.0 {
            return Err(ArithError::DivisionByZero);
        }
        let result = a / b;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ArithError::InvalidFloatResult)
        }
    }

    pub fn rem_float(a: f64, b: f64) -> Result<f64, ArithError> {
        if b == 0.0 {
            return Err(ArithError::RemainderByZero);
        }
        let result = a % b;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ArithError::InvalidFloatResult)
        }
    }

    pub fn neg_float(a: f64) -> Result<f64, ArithError> {
        let result = -a;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ArithError::InvalidFloatResult)
        }
    }

    pub fn int_to_float(i: i64) -> f64 {
        i as f64
    }

    pub fn float_to_int(f: f64) -> Result<i64, ArithError> {
        if !f.is_finite() {
            return Err(ArithError::InvalidFloatResult);
        }
        if f > i64::MAX as f64 || f < i64::MIN as f64 {
            return Err(ArithError::Overflow);
        }
        // Truncate toward zero
        Ok(f as i64)
    }

    pub fn try_int_to_float_exact(i: i64) -> Option<f64> {
        // Check if the integer can be exactly represented as float
        let f = i as f64;
        if f as i64 == i { Some(f) } else { None }
    }
}

/// Conversion operations.
pub mod conversion {
    use super::Value;
    use crate::value::arithmetic::ArithError;

    pub fn to_str(v: &Value) -> String {
        match v {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 && *f >= -9007199254740992.0 && *f <= 9007199254740992.0 {
                    format!("{}.0", f)
                } else {
                    f.to_string()
                }
            }
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Str(s) => s.clone(),
            Value::Unit => "()".to_string(),
            Value::Function { name, .. } => format!("<function {}>", name),
        }
    }

    pub fn to_int(v: &Value) -> Result<i64, ArithError> {
        match v {
            Value::Float(f) => crate::value::arithmetic::float_to_int(*f),
            _ => Err(ArithError::InvalidFloatResult), // Only float->int allowed in v0.1
        }
    }

    pub fn to_float(v: &Value) -> Result<f64, ArithError> {
        match v {
            Value::Int(i) => Ok(crate::value::arithmetic::int_to_float(*i)),
            _ => Err(ArithError::InvalidFloatResult), // Only int->float allowed in v0.1
        }
    }
}
