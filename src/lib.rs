//! KantoLang - A readable, statically-typed interpreted language.

pub mod ast;
pub mod checker;
pub mod diagnostic;
pub mod environment;
pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;
pub mod types;
pub mod value;

use std::fs;
use std::io;
use std::path::Path;

use crate::ast::Program;
use crate::checker::Checker;
use crate::diagnostic::{Diagnostics, Severity};
use crate::evaluator::Evaluator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::span::SourceFile;

/// Run a KantoLang program from source code.
pub fn run_source(
    source: &str,
    name: &str,
    output: Box<dyn io::Write + Send>,
) -> (Option<crate::value::Value>, Diagnostics) {
    let source_file = SourceFile::new(name.to_string(), source.to_string());
    let (tokens, lexer_diags) = Lexer::new(&source_file).lex();
    let (program, parser_diags) = Parser::new(tokens).parse();

    let mut all_diags = lexer_diags;
    all_diags.extend(parser_diags);

    if all_diags.has_errors() {
        return (None, all_diags);
    }

    let (program, checker_diags) = Checker::new(program).check();
    all_diags.extend(checker_diags);

    if all_diags.has_errors() {
        return (None, all_diags);
    }

    let evaluator = Evaluator::new(program, output);
    let (value, eval_diags) = evaluator.evaluate();
    all_diags.extend(eval_diags);

    (Some(value), all_diags)
}

/// Run a KantoLang program from a file.
pub fn run_file<P: AsRef<Path>>(
    path: P,
    output: Box<dyn io::Write + Send>,
) -> (Option<crate::value::Value>, Diagnostics) {
    let path = path.as_ref();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            let mut diags = Diagnostics::new();
            diags.push(crate::diagnostic::Diagnostic::error(format!(
                "failed to read file: {}",
                e
            )));
            return (None, diags);
        }
    };

    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    run_source(&content, &name, output)
}

/// Check a KantoLang program from source code (without executing).
pub fn check_source(source: &str, name: &str) -> (Program, Diagnostics) {
    let source_file = SourceFile::new(name.to_string(), source.to_string());
    let (tokens, lexer_diags) = Lexer::new(&source_file).lex();
    let (program, parser_diags) = Parser::new(tokens).parse();

    let mut all_diags = lexer_diags;
    all_diags.extend(parser_diags);

    if all_diags.has_errors() {
        return (program, all_diags);
    }

    let (program, checker_diags) = Checker::new(program).check();
    all_diags.extend(checker_diags);

    (program, all_diags)
}

/// Check a KantoLang program from a file (without executing).
pub fn check_file<P: AsRef<Path>>(path: P) -> (Program, Diagnostics) {
    let path = path.as_ref();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            let mut diags = Diagnostics::new();
            diags.push(crate::diagnostic::Diagnostic::error(format!(
                "failed to read file: {}",
                e
            )));
            return (
                Program::new(
                    Vec::new(),
                    crate::span::Span::new(crate::span::ByteOffset(0), crate::span::ByteOffset(0)),
                ),
                diags,
            );
        }
    };

    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    check_source(&content, &name)
}
