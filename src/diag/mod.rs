//! Diagnostics and source spans.

use core::fmt;

/// A source span with byte offsets and a one based line and column.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    /// Byte offset of the first byte.
    pub start: usize,
    /// Byte offset one past the last byte.
    pub end: usize,
    /// One based line number.
    pub line: usize,
    /// One based column number.
    pub column: usize,
}

/// The broad category of a diagnostic.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// A source reading failure.
    Parse,
    /// An evaluation failure.
    Eval,
    /// A numeric or FFI failure.
    Numeric,
    /// An internal invariant failure.
    Internal,
}

/// A stable diagnostic code.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    /// Whitespace inside a statement.
    ParseWhitespace,
    /// A comment inside a statement.
    ParseInlineComment,
    /// An unexpected character.
    ParseUnexpectedCharacter,
    /// An unexpected token.
    ParseUnexpectedToken,
    /// An unclosed delimiter.
    ParseUnclosedDelimiter,
    /// A malformed number.
    ParseInvalidNumber,
    /// Indexing syntax used before it is implemented.
    ParseIndexingUnsupported,
    /// An unclosed comment block.
    ParseUnclosedComment,
    /// A call head is not callable.
    EvalNotCallable,
    /// A protected symbol was redefined.
    EvalProtected,
    /// The evaluation budget was exhausted.
    EvalBudget,
    /// A call has the wrong number of arguments.
    EvalArity,
    /// A definition target is invalid.
    EvalDefinition,
    /// An expression handle is no longer live.
    EvalNotLive,
    /// A numeric operation failed.
    NumericFailure,
    /// An internal invariant failed.
    InternalInvariant,
}

impl ErrorCode {
    /// The stable text form of the code.
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::ParseWhitespace => "TGN-PARSE-0001",
            ErrorCode::ParseInlineComment => "TGN-PARSE-0002",
            ErrorCode::ParseUnexpectedCharacter => "TGN-PARSE-0003",
            ErrorCode::ParseUnexpectedToken => "TGN-PARSE-0004",
            ErrorCode::ParseUnclosedDelimiter => "TGN-PARSE-0005",
            ErrorCode::ParseInvalidNumber => "TGN-PARSE-0006",
            ErrorCode::ParseIndexingUnsupported => "TGN-PARSE-0007",
            ErrorCode::ParseUnclosedComment => "TGN-PARSE-0008",
            ErrorCode::EvalNotCallable => "TGN-EVAL-0010",
            ErrorCode::EvalProtected => "TGN-EVAL-0011",
            ErrorCode::EvalBudget => "TGN-EVAL-0012",
            ErrorCode::EvalArity => "TGN-EVAL-0013",
            ErrorCode::EvalDefinition => "TGN-EVAL-0014",
            ErrorCode::EvalNotLive => "TGN-EVAL-0015",
            ErrorCode::NumericFailure => "TGN-NUM-0001",
            ErrorCode::InternalInvariant => "TGN-INT-0001",
        }
    }

    /// The broad category of the code.
    pub const fn kind(self) -> ErrorKind {
        match self {
            ErrorCode::ParseWhitespace
            | ErrorCode::ParseInlineComment
            | ErrorCode::ParseUnexpectedCharacter
            | ErrorCode::ParseUnexpectedToken
            | ErrorCode::ParseUnclosedDelimiter
            | ErrorCode::ParseInvalidNumber
            | ErrorCode::ParseIndexingUnsupported
            | ErrorCode::ParseUnclosedComment => ErrorKind::Parse,
            ErrorCode::EvalNotCallable
            | ErrorCode::EvalProtected
            | ErrorCode::EvalBudget
            | ErrorCode::EvalArity
            | ErrorCode::EvalDefinition
            | ErrorCode::EvalNotLive => ErrorKind::Eval,
            ErrorCode::NumericFailure => ErrorKind::Numeric,
            ErrorCode::InternalInvariant => ErrorKind::Internal,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A frame in an evaluation trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    /// The head symbol, when known.
    pub head: Option<String>,
    /// The expression at this frame.
    pub expression: String,
    /// The source span, when known.
    pub span: Option<Span>,
}

/// A structured diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// The stable code.
    pub code: ErrorCode,
    /// The broad category.
    pub kind: ErrorKind,
    /// A deterministic message.
    pub message: String,
    /// The source span, when known.
    pub span: Option<Span>,
    /// The evaluation trace.
    pub trace: Vec<Frame>,
    /// Additional notes.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Build a parse diagnostic.
    pub fn parse(code: ErrorCode, message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            code,
            kind: ErrorKind::Parse,
            message: message.into(),
            span: Some(span),
            trace: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Build an evaluation diagnostic.
    pub fn eval(code: ErrorCode, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            kind: ErrorKind::Eval,
            message: message.into(),
            span: None,
            trace: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Add a note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add a trace frame.
    pub fn with_frame(mut self, frame: Frame) -> Self {
        self.trace.push(frame);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "error {}: {}", self.code, self.message)?;
        if let Some(span) = self.span {
            writeln!(f, "  --> {}:{}", span.line, span.column)?;
        }
        for note in &self.notes {
            writeln!(f, "  = note: {note}")?;
        }
        if !self.trace.is_empty() {
            writeln!(f, "  = trace:")?;
            for (index, frame) in self.trace.iter().enumerate() {
                writeln!(f, "      {}. {}", index + 1, frame.expression)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}
