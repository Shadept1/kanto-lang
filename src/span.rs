use std::fmt;

/// A byte offset in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteOffset(pub usize);

impl ByteOffset {
    pub const fn new(offset: usize) -> Self {
        ByteOffset(offset)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl From<usize> for ByteOffset {
    fn from(offset: usize) -> Self {
        ByteOffset(offset)
    }
}

impl From<ByteOffset> for usize {
    fn from(offset: ByteOffset) -> Self {
        offset.0
    }
}

impl fmt::Display for ByteOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A span in a source file, represented as a byte range [start, end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: ByteOffset,
    pub end: ByteOffset,
}

impl Span {
    pub const fn new(start: ByteOffset, end: ByteOffset) -> Self {
        Span { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.as_usize() - self.start.as_usize()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn merge(&self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A source location with line and column (1-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourcePos {
    pub line: usize,
    pub column: usize,
}

impl SourcePos {
    pub fn new(line: usize, column: usize) -> Self {
        SourcePos { line, column }
    }
}

impl fmt::Display for SourcePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A source file with content and precomputed line starts for fast position lookup.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub content: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: String, content: String) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in content.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        SourceFile {
            name,
            content,
            line_starts,
        }
    }

    pub fn span_to_pos(&self, offset: ByteOffset) -> SourcePos {
        let offset = offset.as_usize().min(self.content.len());
        let line_idx = self
            .line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i - 1);
        let line_start = self.line_starts[line_idx];
        SourcePos::new(line_idx + 1, offset - line_start + 1)
    }

    pub fn span_to_range(&self, span: Span) -> (SourcePos, SourcePos) {
        (self.span_to_pos(span.start), self.span_to_pos(span.end))
    }

    pub fn line_content(&self, line: usize) -> Option<&str> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[line - 1];
        let end = if line < self.line_starts.len() {
            self.line_starts[line].saturating_sub(1)
        } else {
            self.content.len()
        };
        Some(&self.content[start..end])
    }
}
