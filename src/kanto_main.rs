use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use kantolang::{check_file, run_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    let program_name = args[0].clone();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    // Get current directory for main.knt lookup
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match args.get(1).map(|s| s.as_str()) {
        Some("--help") | Some("-h") => {
            print_usage(&program_name, &mut stdout, &current_dir);
            std::process::exit(0);
        }
        Some("--version") | Some("-v") => {
            println!("kanto {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        Some("check") => {
            if args.len() != 3 {
                eprintln!("Usage: {} check <file.knt>", program_name);
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
        Some("run") => {
            if args.len() != 3 {
                eprintln!("Usage: {} run <file.knt>", program_name);
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
        Some(arg) if arg.starts_with('-') => {
            eprintln!("Unknown option: {}", arg);
            print_usage(&program_name, &mut stderr, &current_dir);
            std::process::exit(2);
        }
        Some(file_arg) => {
            // Explicit file argument provided
            let path = PathBuf::from(file_arg);
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
        None => {
            // No arguments - try to run main.knt in current directory
            let main_path = current_dir.join("main.knt");
            if main_path.exists() {
                let (_, diags) = run_file(&main_path, Box::new(io::stdout()));
                if diags.has_errors() {
                    let source = std::fs::read_to_string(&main_path).unwrap_or_default();
                    let source_file =
                        kantolang::span::SourceFile::new("main.knt".to_string(), source);
                    for diag in &diags.items {
                        eprint!("{}", diag.render(&source_file));
                    }
                    std::process::exit(1);
                }
                std::process::exit(0);
            } else {
                eprintln!("No main.knt found in the current directory.");
                eprintln!("Searched in: {}", current_dir.display());
                eprintln!();
                eprintln!("Create a main.knt file or run with an explicit file:");
                eprintln!("  kanto <file.knt>");
                eprintln!("  kanto run <file.knt>");
                std::process::exit(2);
            }
        }
    }
}

fn print_usage(program: &str, out: &mut dyn Write, current_dir: &std::path::Path) {
    writeln!(out, "KantoLang v{} (kanto)", env!("CARGO_PKG_VERSION")).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "Usage:").unwrap();
    writeln!(
        out,
        "  {}                 Run main.knt from current directory ({})",
        program,
        current_dir.display()
    )
    .unwrap();
    writeln!(out, "  {} <file.knt>      Run a KantoLang program", program).unwrap();
    writeln!(out, "  {} run <file.knt>  Run a KantoLang program", program).unwrap();
    writeln!(
        out,
        "  {} check <file.knt> Check a KantoLang program (no execution)",
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
