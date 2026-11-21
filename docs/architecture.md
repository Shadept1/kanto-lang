# KantoLang Architecture

## Overview

KantoLang is implemented in Rust as a tree-walk interpreter with a multi-pass compilation pipeline:

```
Source Code → Lexer → Parser → Type Checker → Evaluator → Output
```

Each pass produces diagnostics (errors/warnings) that can halt the pipeline.

## Components

### 1. Lexer (`src/lexer.rs`)

**Responsibilities:**
- Convert source text into a stream of tokens
- Handle indentation (generate `INDENT`/`DEDENT` tokens)
- Process string escape sequences
- Track source positions (line, column, byte offset)
- Reject tabs in indentation

**Key Features:**
- Single-pass, stateful lexer
- Maintains indentation stack for `INDENT`/`DEDENT` generation
- Handles pending dedents at EOF
- Produces tokens with `Span` for error reporting

**Token Types:**
- Keywords (`Let`, `Mut`, `Def`, `If`, `Else`, `While`, `For`, `In`, `Break`, `Continue`, `Ret`, `True`, `False`, `And`, `Or`, `Not`)
- Types (`IntType`, `FloatType`, `BoolType`, `StrType`, `UnitType`)
- Built-ins (`Print`, `Range`, `IntBuiltin`, `FloatBuiltin`, `StrBuiltin`)
- Literals (`IntLit(i64)`, `FloatLit(f64)`, `StrLit(String)`, `BoolLit(bool)`)
- Identifiers (`Ident(String)`)
- Operators (`Plus`, `Minus`, `Star`, `Slash`, `Percent`, `Eq`, `EqEq`, `NotEq`, `Lt`, `LtEq`, `Gt`, `GtEq`, `Assign`, `Colon`, `Arrow`, `Comma`)
- Structure (`LParen`, `RParen`, `Newline`, `Indent`, `Dedent`, `Eof`)

### 2. Parser (`src/parser.rs`)

**Responsibilities:**
- Convert token stream into Abstract Syntax Tree (AST)
- Handle indentation-based block structure
- Implement Pratt parser (precedence climbing) for expressions
- Produce diagnostics for syntax errors

**Key Features:**
- Recursive descent for statements
- Pratt parser for expressions with correct precedence:
  1. `or` (lowest)
  2. `and`
  3. `== != < <= > >=`
  4. `+ -`
  5. `* / %` (highest)
- Left-associative operators
- Handles function calls, unary operators, grouping
- Error recovery: skips invalid tokens and continues parsing

**AST Nodes:**
- `Program` - top-level container
- `Stmt::Let`, `Stmt::Mut`, `Stmt::Assign`, `Stmt::Def`, `Stmt::Ret`, `Stmt::If`, `Stmt::While`, `Stmt::For`, `Stmt::Break`, `Stmt::Continue`, `Stmt::Expr`
- `Expr::IntLit`, `Expr::FloatLit`, `Expr::StrLit`, `Expr::BoolLit`, `Expr::UnitLit`, `Expr::Ident`, `Expr::Binary`, `Expr::Unary`, `Expr::Call`, `Expr::Group`
- `Block` - sequence of statements with span
- `Param` - function parameter with name and type annotation

### 3. Type Checker (`src/checker.rs`)

**Responsibilities:**
- Two-pass semantic analysis
- Static type checking
- Symbol table management with lexical scoping
- Return path analysis for non-unit functions

**Pass 1: Function Signature Collection**
- Scans all top-level `Def` statements
- Registers function signatures in symbol table
- Enables forward references (mutual recursion)

**Pass 2: Full Type Checking**
- Checks variable declarations, assignments, expressions
- Validates function calls against signatures
- Ensures type compatibility
- Checks control flow (break/continue in loops, return paths)
- Verifies built-in function arguments

**Type System:**
- Primitive types: `Int`, `Float`, `Bool`, `Str`, `Unit`
- Function types: `Function { params: Vec<Type>, return_type: Box<Type> }`
- Type compatibility: exact match required (no implicit conversions)

**Symbol Table:**
- Lexical scoping with `push_scope`/`pop_scope`
- Tracks: variables (mutable/immutable), functions, built-ins
- Shadowing allowed in nested scopes
- Detects duplicate declarations in same scope

### 4. Evaluator (`src/evaluator.rs`)

**Responsibilities:**
- Tree-walk interpretation of typed AST
- Runtime value management
- Control flow handling (return, break, continue)
- Built-in function execution
- Arithmetic with overflow checking

**Runtime Values:**
```rust
enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    // Function values not directly stored (called via AST)
}
```

**Arithmetic (`src/value.rs`):**
- Checked integer operations (`add_int`, `sub_int`, `mul_int`, `div_int`, `rem_int`, `neg_int`)
- Float operations with finite result validation
- Conversion functions (`int_to_float`, `float_to_int`, `try_int_to_float_exact`)

**Control Flow:**
```rust
enum ControlFlow {
    None,
    Return(Option<Value>),
    Break,
    Continue,
}
```

**Environment (`src/environment.rs`):**
- Stack of hash maps for variable storage
- `push_scope`/`pop_scope` for block scoping
- `define` for new variables, `assign` for mutation
- Shadowing supported

**Built-in Functions:**
- `print`: outputs values to stdout
- `str`, `int`, `float`: type conversions
- `range`: generates range for for-loops (encoded as special string)

### 5. Diagnostics (`src/diagnostic.rs`)

**Responsibilities:**
- Structured error/warning reporting
- Source code rendering with labels
- Colorized terminal output

**Features:**
- `Diagnostic` with severity (Error/Warning), message, labels, notes
- `Label` with span, message, style (primary/secondary)
- `Diagnostics` collection with `has_errors()` check
- Pretty printing with source context

### 6. Span Tracking (`src/span.rs`)

**Responsibilities:**
- Source position tracking (byte offset, line, column)
- `SourceFile` with content and line start offsets
- `Span` with start/end byte offsets
- `ByteOffset` newtype for type safety
- Conversion between byte offsets and line/column

## Pipeline Flow

```
run_file(path)
  → read file
  → run_source(content)
    → Lexer::lex() → (tokens, lexer_diags)
    → Parser::parse() → (program, parser_diags)
    → Checker::check() → (program, checker_diags)
    → Evaluator::evaluate() → (value, eval_diags)
    → print diagnostics, exit with code
```

## Error Handling Strategy

1. **Lexer errors**: Invalid characters, unterminated strings, invalid escapes, tabs in indentation
2. **Parser errors**: Unexpected tokens, missing delimiters, invalid structure
3. **Checker errors**: Type mismatches, undefined names, scope violations, missing returns
4. **Evaluator errors**: Arithmetic overflow, division by zero, runtime type errors

Each pass collects diagnostics. If any pass has errors, subsequent passes are skipped (except evaluator runs if checker passes).

## Testing Strategy

- Unit tests for each component (lexer, parser, checker, evaluator)
- Integration tests via `eval_source` and `check_source` helpers
- Test coverage for:
  - All language constructs
  - Type checking rules
  - Error conditions
  - Edge cases (overflow, division by zero, scoping)

## Extensibility Points

1. **New types**: Add to `PrimitiveType`, `Type`, update checker/evaluator
2. **New built-ins**: Add to `builtin_types()`, `lex_identifier_or_keyword`, parser, checker, evaluator
3. **New operators**: Add to `BinOp`/`UnOp`, update lexer, parser, checker, evaluator
4. **New statements**: Add to `Stmt`, update parser, checker, evaluator
5. **REPL mode**: Could be added by running pipeline incrementally

## Performance Considerations

- Tree-walk interpreter: simple but not optimized for heavy computation
- String interning not implemented (could optimize identifier storage)
- Function calls clone AST blocks (could use shared references with lifetime management)
- No bytecode compilation or JIT
- Suitable for scripting, education, small programs

## Future Improvements

1. **REPL**: Interactive mode
2. **Module system**: Import/export
3. **Data structures**: Arrays, tuples, structs
4. **Pattern matching**: Match expressions
5. **Closures**: First-class functions with capture
6. **Standard library**: File I/O, collections, math
7. **Optimization**: Bytecode interpreter, constant folding