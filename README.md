<img width="6912" height="3456" alt="KANTOLANG" src="https://github.com/user-attachments/assets/a957e496-fb29-4fb1-af75-1106ea1b2a6e" />
# ⚡ KantoLang

> *Write clean, expressive code with the readability of abbreviated English and the safety & performance spirit of Rust.*

[![Status](https://img.shields.io/badge/status-early%20development-orange?style=for-the-badge)]()
[![Built With](https://img.shields.io/badge/built%20with-Rust-dea584?style=for-the-badge&logo=rust)]()
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)]()

---

## 🌟 About KantoLang

**KantoLang** is an experimental, modern programming language currently being built **from scratch in Rust**. Its core philosophy is to bridge **human-friendly, concise syntax** (inspired by *abbreviated English* and Python-like clarity) with **strong safety, predictable behavior, and high performance** inspired by Rust.

Whether you are prototyping ideas quickly or building reliable software systems, KantoLang aims to keep code *clean, readable, and intentional* without unnecessary boilerplate.

---

## 🎯 Core Pillars

| Pillar | Philosophy | What It Means in Practice |
| :--- | :--- | :--- |
| 📖 **Readability** | *Abbreviated English* | Short, intuitive keywords and clean structure that read naturally. |
| 🛡️ **Safety First** | *Rust-Inspired Discipline* | Clear mutability rules, explicit types, and helpful error messages. |
| ⚡ **Performance** | *Systems Mindset* | Designed with an efficient execution pipeline built on **Rust**. |
| 🧩 **Simplicity** | *Low Friction* | Easy to pick up without drowning in cryptic symbols or ceremony. |

---

## ✨ Syntax Preview *(Draft / Work in Progress)*

Here is a glimpse of how **KantoLang** is designed to look as the language evolves:

```text
mut count = 0
mut step = 5

def add(a: int, b: int) -> int:
    ret a + b

count = add(count, step)

if count >= 5:
    print("Ready to go!")
```

> **Note:** Syntax details are actively evolving as Phase 1 of the interpreter/compiler takes shape.


## 🛠️ Project Architecture

The implementation in Rust is organized into focused, modular components:

| Module | Responsibility |
| :--- | :--- |
| `src/token.rs` | Defines token types, keywords, literals, and operators. |
| `src/lexer.rs` | Scans raw source code into a stream of tokens. |
| `src/ast.rs` | Represents program structure via an Abstract Syntax Tree. |
| `src/parser.rs` | Parses tokens into AST nodes and enforces operator precedence. |
| `src/evaluator.rs` | Executes statements/expressions and manages runtime state/errors. |
| `src/main.rs` | Entry point for running source files and tests. |

---

## 🚀 Getting Started

### Prerequisites

Make sure you have **Rust** (`rustc` and `cargo`) and **Git** installed on your machine.

### Clone & Run

```bash
git clone [https://github.com/Shadept1/kanto-lang.git](https://github.com/Shadept1/kanto-lang.git)
cd kanto-lang
cargo run
```

### Run Tests

```bash
cargo test
```

---

## 🤝 Contributing & Feedback

KantoLang is in its early stages and growing step by step. Ideas, syntax suggestions, bug reports, and experiments are all welcome as the language takes shape!

---

## 📄 License

Distributed under the **MIT License**. See [`LICENSE`](./LICENSE) for more information.
