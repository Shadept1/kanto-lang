# KantoLang

A statically-typed, indentation-based interpreted programming language implemented in Rust.

## Features

- **Indentation-based syntax** (4 spaces, no tabs)
- **Static type checking** with type inference
- **Tree-walk interpreter** with runtime error handling
- **Built-in functions**: `print`, `str`, `int`, `float`, `range`
- **Control flow**: `if/else`, `while`, `for`, `break`, `continue`
- **Functions** with parameters and return types
- **Arithmetic** with overflow checking
- **VS Code extension** with syntax highlighting, snippets, and tasks

## Quick Start

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))

### Building

```bash
cargo build --release
```

The binary will be at `target/release/kantolang` (or `target/release/kantolang.exe` on Windows).

### Running a Program

```bash
# Run a KantoLang file
kantolang examples/hello.knt

# Or use the run subcommand
kantolang run examples/hello.knt

# Check syntax and types without executing
kantolang check examples/hello.knt
```

## Language Overview

### Hello World

```kantolang
print("Hello, World!")
```

### Variables

```kantolang
# Immutable (type inferred)
let x = 42

# Immutable with explicit type
let y: int = 10

# Mutable
mut z = 0
z = z + 1  # OK

# Reassignment only for mut variables
let a = 1
a = 2  # Error: cannot assign to immutable variable
```

### Functions

```kantolang
def add(a: int, b: int) -> int:
    ret a + b

def greet(name: str) -> unit:
    print("Hello, " + name)

# Call functions
let result = add(2, 3)
greet("World")
```

### Control Flow

```kantolang
# If-else
if x > 0:
    print("positive")
else:
    print("non-positive")

# While loop
mut i = 0
while i < 10:
    print(i)
    i = i + 1

# For loop
for i in range(5):        # 0, 1, 2, 3, 4
    print(i)

for i in range(2, 5):     # 2, 3, 4
    print(i)

# Break/continue
while true:
    if i == 5:
        break
    if i % 2 == 0:
        continue
    print(i)
```

### Types

```kantolang
# Primitive types
let i: int = 42
let f: float = 3.14
let b: bool = true
let s: str = "hello"
let u: unit = ()

# Type conversions (explicit, no implicit conversions)
let i2 = int(3.14)    # float -> int (truncates)
let f2 = float(42)    # int -> float
let s2 = str(42)      # int -> string
```

### Built-in Functions

| Function | Description |
|----------|-------------|
| `print(...)` | Print values to stdout (any number of args) |
| `str(int)` | Convert integer to string |
| `int(float)` | Convert float to integer (truncates) |
| `float(int)` | Convert integer to float |
| `range(stop)` | Create range 0..stop |
| `range(start, stop)` | Create range start..stop |

## Examples

See the `examples/` directory for more complete programs:

- `hello.knt` - Hello World
- `factorial.knt` - Recursive factorial
- `fibonacci.knt` - Iterative Fibonacci
- `guessing_game.knt` - Number guessing game

Run them with:
```bash
kantolang examples/factorial.knt
```

## VS Code Extension

The `editors/vscode/` directory contains a VS Code extension with:

- Syntax highlighting
- Code snippets (type `def`, `if`, `for`, `let`, etc.)
- Tasks for Run (Ctrl+F5) and Check
- Bracket matching and auto-indentation

To install:
1. Open the `editors/vscode` folder in VS Code
2. Press F5 to launch Extension Development Host
3. Open a `.knt` file

Or package and install:
```bash
cd editors/vscode
npm install -g vsce
vsce package
code --install-extension kantolang-0.1.0.vsix
```

## Architecture

KantoLang uses a multi-pass compilation pipeline:

```
Source → Lexer → Parser → Type Checker → Evaluator
```

- **Lexer**: Indentation-aware tokenization with INDENT/DEDENT
- **Parser**: Pratt parser (precedence climbing) for expressions
- **Type Checker**: Two-pass (signatures first, then bodies) with lexical scoping
- **Evaluator**: Tree-walk interpreter with checked arithmetic

See `docs/architecture.md` for details.

## License

MIT License - see LICENSE file for details.