use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use kantolang::{check_file, run_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    if args.len() < 2 {
        print_usage(&args[0], &mut stderr);
        std::process::exit(2);
    }

    match args[1].as_str() {
        "--help" | "-h" => {
            print_usage(&args[0], &mut stdout);
            std::process::exit(0);
        }
        "--version" | "-v" => {
            println!("kantolang {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        "check" => {
            if args.len() != 3 {
                eprintln!("Usage: {} check <file.knt>", args[0]);
                std::process::exit(2);
            }
            let path = PathBuf::from(&args[2]);
            let (_, diags) = check_file(&path);
            if diags.items.is_empty() {
                println!("OK");
                std::process::exit(0);
            } else {
                let source = std::fs::read_to_string(&path).unwrap_or_default();
                let source_file = kantolang::span::SourceFile::new(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    source,
                );
                for diag in &diags.items {
                    eprint!("{}", diag.render(&source_file));
                }
                std::process::exit(1);
            }
        }
        "run" => {
            if args.len() != 3 {
                eprintln!("Usage: {} run <file.knt>", args[0]);
                std::process::exit(2);
            }
            let path = PathBuf::from(&args[2]);
            let (_, diags) = run_file(&path, Box::new(io::stdout()));
            if diags.has_errors() {
                let source = std::fs::read_to_string(&path).unwrap_or_default();
                let source_file = kantolang::span::SourceFile::new(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    source,
                );
                for diag in &diags.items {
                    eprint!("{}", diag.render(&source_file));
                }
                std::process::exit(1);
            }
            std::process::exit(0);
        }
        arg if arg.starts_with('-') => {
            eprintln!("Unknown option: {}", arg);
            print_usage(&args[0], &mut stderr);
            std::process::exit(2);
        }
        _ => {
            // Bare file path - run the program
            let path = PathBuf::from(&args[1]);
            let (_, diags) = run_file(&path, Box::new(io::stdout()));
            if diags.has_errors() {
                let source = std::fs::read_to_string(&path).unwrap_or_default();
                let source_file = kantolang::span::SourceFile::new(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    source,
                );
                for diag in &diags.items {
                    eprint!("{}", diag.render(&source_file));
                }
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }
}

fn print_usage(program: &str, out: &mut dyn Write) {
    writeln!(out, "KantoLang v{}", env!("CARGO_PKG_VERSION")).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "Usage:").unwrap();
    writeln!(
        out,
        "  {} <file.knt>          Run a KantoLang program",
        program
    )
    .unwrap();
    writeln!(
        out,
        "  {} run <file.knt>      Run a KantoLang program",
        program
    )
    .unwrap();
    writeln!(
        out,
        "  {} check <file.knt>    Check a KantoLang program (no execution)",
        program
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(out, "Options:").unwrap();
    writeln!(out, "  -h, --help     Show this help message").unwrap();
    writeln!(out, "  -v, --version  Show version information").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "Exit codes:").unwrap();
    writeln!(out, "  0  Success").unwrap();
    writeln!(out, "  1  Compilation or runtime error").unwrap();
    writeln!(out, "  2  Invalid usage or file access error").unwrap();
}
