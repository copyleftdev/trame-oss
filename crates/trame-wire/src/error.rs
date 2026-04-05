//! Error types for X12 parsing.

use std::fmt;

/// The kind of parse error encountered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// The ISA segment is malformed or too short.
    InvalidIsa,
    /// A detected delimiter is invalid (e.g., alphanumeric).
    InvalidDelimiter,
    /// Input ended before a complete structure was found.
    UnexpectedEndOfInput,
    /// A segment has no identifier.
    MissingSegmentId,
    /// Envelope structure (ISA/GS/ST/SE/GE/IEA) is malformed.
    InvalidEnvelope,
    /// A closing control number does not match its opening.
    MismatchedControlNumber,
    /// The segment or group count in a trailer is wrong.
    InvalidSegmentCount,
    /// The functional group count in IEA is wrong.
    InvalidGroupCount,
}

/// An error encountered while parsing X12 data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// What kind of error this is.
    pub kind: ParseErrorKind,
    /// Byte offset in the input where the error was detected.
    pub offset: usize,
    /// Human-readable description.
    pub message: String,
}

impl ParseError {
    /// Create a new `ParseError`.
    pub fn new(kind: ParseErrorKind, offset: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            offset,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "X12 parse error at byte {}: {:?}: {}",
            self.offset, self.kind, self.message
        )
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format() {
        let err = ParseError::new(ParseErrorKind::InvalidIsa, 0, "ISA too short");
        let s = err.to_string();
        assert!(s.contains("byte 0"));
        assert!(s.contains("InvalidIsa"));
        assert!(s.contains("ISA too short"));
    }

    #[test]
    fn error_trait() {
        let err = ParseError::new(ParseErrorKind::UnexpectedEndOfInput, 42, "truncated");
        // Ensure it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn equality() {
        let a = ParseError::new(ParseErrorKind::InvalidDelimiter, 3, "bad");
        let b = ParseError::new(ParseErrorKind::InvalidDelimiter, 3, "bad");
        assert_eq!(a, b);
    }
}
