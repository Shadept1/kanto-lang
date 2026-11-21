use std::fmt;
use thiserror::Error;

use crate::span::{SourceFile, SourcePos, Span};

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
        }
    }
}

/// A labeled span for diagnostic output.
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

impl Label {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Label {
            span,
            message: message.into(),
        }
    }

    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Label::new(span, message)
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Label::new(span, message)
    }
}

/// A diagnostic with source context.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn note(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Note,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_labels(mut self, labels: Vec<Label>) -> Self {
        self.labels.extend(labels);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes.extend(notes);
        self
    }

    /// Render the diagnostic to a string using the given source file.
    pub fn render(&self, source: &SourceFile) -> String {
        let mut out = String::new();
        let code_str = self
            .code
            .as_deref()
            .map(|c| format!("[{}] ", c))
            .unwrap_or_default();

        // Header
        if let Some(label) = self.labels.first() {
            let (start_pos, _) = source.span_to_range(label.span);
            out.push_str(&format!(
                "{}:{}: {}: {}{}\n",
                source.name, start_pos, self.severity, code_str, self.message
            ));
        } else {
            out.push_str(&format!(
                "{}: {}: {}{}\n",
                source.name, self.severity, code_str, self.message
            ));
        }

        // Source lines with labels
        for label in &self.labels {
            let (start_pos, end_pos) = source.span_to_range(label.span);
            if let Some(line_content) = source.line_content(start_pos.line) {
                out.push_str(&format!("  --> {}:{}\n", source.name, start_pos.line));
                out.push_str(&format!("   |\n"));
                out.push_str(&format!("{} | {}\n", start_pos.line, line_content));
                out.push_str("   | ");

                let start_col = start_pos.column.saturating_sub(1);
                let end_col = if end_pos.line == start_pos.line {
                    end_pos.column.saturating_sub(1)
                } else {
                    line_content.len()
                };
                let len = end_col.saturating_sub(start_col);

                out.push_str(&" ".repeat(start_col));
                out.push_str(&"^".repeat(len.max(1)));
                out.push_str(&format!(" {}\n", label.message));
            }
        }

        // Notes
        for note in &self.notes {
            out.push_str(&format!("   = note: {}\n", note));
        }

        out
    }
}

/// A collection of diagnostics.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics { items: Vec::new() }
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.items.push(diag);
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    }

    pub fn render_all(&self, source: &SourceFile) -> String {
        self.items
            .iter()
            .map(|d| d.render(source))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This requires a source file, so we just show count
        write!(f, "{} diagnostics", self.items.len())
    }
}

/// Result type for operations that can produce diagnostics.
pub type DiagResult<T> = Result<T, Diagnostics>;

/// Trait for converting errors to diagnostics.
pub trait ToDiagnostics {
    fn to_diagnostics(self) -> Diagnostics;
}

impl ToDiagnostics for Diagnostics {
    fn to_diagnostics(self) -> Diagnostics {
        self
    }
}

impl<T, E: Into<Diagnostic>> ToDiagnostics for Result<T, E> {
    fn to_diagnostics(self) -> Diagnostics {
        match self {
            Ok(_) => Diagnostics::new(),
            Err(e) => {
                let mut diags = Diagnostics::new();
                diags.push(e.into());
                diags
            }
        }
    }
}
