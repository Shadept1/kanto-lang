# KantoLang Language Specification v0.1

## Overview

KantoLang is a statically-typed, indentation-based interpreted programming language. It features a clean syntax inspired by Python, with explicit type annotations and static type checking before execution.

## Lexical Structure

### Indentation

- Indentation is significant and uses **4 spaces** (no tabs allowed)
- Blocks are defined by indentation level
- `INDENT` and `DEDENT` tokens are generated automatically by the lexer
- Empty lines and comment-only lines are ignored for indentation purposes

### Comments

- Single-line comments start with `#` and continue to end of line
- Comments are ignored by the parser

### Identifiers

- Must start with a letter (a-z, A-Z) or underscore (_)
- Can contain letters, digits (0-9), and underscores
- Case-sensitive
- Cannot be keywords

### Keywords

The following are reserved keywords and cannot be used as identifiers:

```
let, mut, def, ret, if, else, while, for, in, break, continue,
true, false, and, or, not, int, float, bool, str, unit,
print, range
```

### Literals

- **Integer**: Decimal integers (e.g., `42`, `-10`)
- **Float**: Decimal numbers with a decimal point (e.g., `3.14`, `-2.5`)
- **String**: Double-quoted strings with escape sequences (e.g., `"hello\nworld"`)
- **Boolean**: `true` or `false`
- **Unit**: `()` (empty tuple)

### Operators

```
+  -  *  /  %     Arithmetic
== != <  <= > >=  Comparison
and or not        Logical
=                 Assignment
:                 Type annotation separator
->                Function return type
```

## Types

### Primitive Types

| Type | Description | Literal Example |
|------|-------------|-----------------|
| `int` | 64-bit signed integer | `42` |
| `float` | 64-bit floating point | `3.14` |
| `bool` | Boolean value | `true`, `false` |
| `str` | UTF-8 string | `"hello"` |
| `unit` | No value (like void) | `()` |

### Type Compatibility

- No implicit conversions between types in v0.1
- Explicit conversion functions: `int()`, `float()`, `str()`
- `int()` accepts only `float`
- `float()` accepts only `int`
- `str()` accepts only `int` (in v0.1)

## Declarations

### Immutable Binding (`let`)

```kantolang
let name: type = expression
let name = expression  # Type inferred from expression
```

- Variable cannot be reassigned
- Type annotation is optional if initializer is present
- Must have either type annotation or initializer

### Mutable Binding (`mut`)

```kantolang
mut name: type = expression
mut name = expression  # Type inferred
```

- Variable can be reassigned using `=`
- Same type rules as `let`

### Assignment

```kantolang
name = expression
```

- Only allowed for `mut` variables
- Type must match the variable's declared type

## Functions

### Function Definition

```kantolang
def name(param1: type1, param2: type2, ...) -> return_type:
    # function body
    ret expression
```

- Parameters are immutable
- Return type is required
- `ret` with expression returns a value
- `ret` without expression returns `unit` (only valid for unit functions)
- All paths must return a value for non-unit functions (checked statically)

### Function Call

```kantolang
name(arg1, arg2, ...)
```

- Arguments are evaluated left-to-right
- Number and types of arguments must match parameters

### Built-in Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `print` | `print(...values: any) -> unit` | Print values to stdout |
| `str` | `str(value: int) -> str` | Convert int to string |
| `int` | `int(value: float) -> int` | Convert float to int (truncates) |
| `float` | `float(value: int) -> float` | Convert int to float |
| `range` | `range(stop: int) -> iter` / `range(start: int, stop: int) -> iter` | Create range for for-loops |

## Control Flow

### If Statement

```kantolang
if condition:
    # then branch
else:
    # else branch (optional)
```

- Condition must be `bool`
- Both branches create new scopes

### While Loop

```kantolang
while condition:
    # body
```

- Condition must be `bool`
- Creates new scope for body
- `break` and `continue` supported

### For Loop

```kantolang
for variable in range(stop):
    # body

for variable in range(start, stop):
    # body
```

- Loop variable is immutable and scoped to the loop
- Iterates from `start` (inclusive) to `stop` (exclusive)
- Default `start` is 0
- Creates new scope for each iteration

### Break and Continue

```kantolang
break   # Exit nearest loop
continue  # Skip to next iteration
```

- Only allowed inside `while` or `for` loops

## Expressions

### Arithmetic

- `+`, `-`, `*`, `/`, `%` for `int` and `float`
- String concatenation: `"a" + "b"`
- Overflow checking for integers (runtime error on overflow)
- Division by zero checking (runtime error)

### Comparison

- `==`, `!=`, `<`, `<=`, `>`, `>=` for `int`, `float`, `str`, `bool`
- Returns `bool`

### Logical

- `and`, `or` for `bool` (short-circuit evaluation)
- `not` for `bool`

### Unary

- `-` for `int` and `float` (negation)
- `not` for `bool`

### Grouping

- Parentheses for grouping: `(expression)`

## Error Handling

### Static Errors (Compile-time)

- Syntax errors
- Type mismatches
- Undefined variables
- Duplicate declarations in same scope
- Missing return in non-unit functions
- Break/continue outside loops
- Wrong number of function arguments
- Invalid operands for operators

### Runtime Errors

- Integer overflow
- Division by zero
- Remainder by zero
- Invalid float results (NaN, infinity)
- Stack overflow (max call depth exceeded)
- Failed type conversions (e.g., `int(infinity)`)

## Program Structure

- A program is a sequence of statements
- Statements can be declarations, assignments, function definitions, control flow, or expressions
- Top-level expressions are allowed but their values are discarded
- Execution starts at the first statement and proceeds sequentially

## Example Program

```kantolang
# Calculate Fibonacci numbers
def fib(n: int) -> int:
    if n <= 1:
        ret n
    ret fib(n - 1) + fib(n - 2)

mut i = 0
while i < 10:
    print("fib(" + str(i) + ") = " + str(fib(i)))
    i = i + 1
```