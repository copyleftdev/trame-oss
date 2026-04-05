//! Delimiter detection from X12 ISA headers.
//!
//! The ISA segment is the only fixed-width segment in X12. Its layout reveals
//! the delimiters used throughout the interchange:
//!
//! - Position 3: element separator (typically `*`)
//! - Position 104: sub-element (component) separator (typically `:`)
//! - Position 105: segment terminator (typically `~`)
//! - ISA11 (positions 82..84 of the data): repetition separator (version >= 00402)

use crate::error::{ParseError, ParseErrorKind};

/// The minimum length of an ISA segment including the segment terminator.
/// ISA has 16 elements, fixed width = 106 characters including the terminator.
pub const ISA_MIN_LEN: usize = 106;

/// Delimiters used in an X12 interchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delimiters {
    /// Element separator (typically `*`).
    pub element: u8,
    /// Sub-element (component) separator (typically `:`).
    pub sub_element: u8,
    /// Segment terminator (typically `~`).
    pub segment: u8,
    /// Repetition separator (ISA11 in version >= 00402). `None` for older versions.
    pub repetition: Option<u8>,
}

impl Default for Delimiters {
    /// Standard X12 delimiters: `*` element, `:` sub-element, `~` segment.
    fn default() -> Self {
        Self {
            element: b'*',
            sub_element: b':',
            segment: b'~',
            repetition: None,
        }
    }
}

impl Delimiters {
    /// Auto-detect delimiters from the beginning of an X12 input buffer.
    ///
    /// The input must start with an ISA segment. This function reads the
    /// fixed-position bytes to determine all delimiters.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the input is too short, doesn't start with "ISA",
    /// or contains invalid delimiters.
    pub fn detect(input: &[u8]) -> Result<Self, ParseError> {
        // We need at least 106 bytes for a complete ISA segment.
        // But we also need to handle trailing CR/LF after the terminator,
        // so scan forward if needed.
        if input.len() < ISA_MIN_LEN {
            return Err(ParseError::new(
                ParseErrorKind::InvalidIsa,
                0,
                format!(
                    "input too short for ISA segment: need at least {} bytes, got {}",
                    ISA_MIN_LEN,
                    input.len()
                ),
            ));
        }

        // Verify the segment starts with "ISA"
        if &input[..3] != b"ISA" {
            return Err(ParseError::new(
                ParseErrorKind::InvalidIsa,
                0,
                format!(
                    "expected ISA at start, found {:?}",
                    String::from_utf8_lossy(&input[..3.min(input.len())])
                ),
            ));
        }

        // Position 3 is the element separator
        let element = input[3];

        // The sub-element separator is at position 104 (ISA16, the component element separator)
        let sub_element = input[104];

        // The segment terminator is at position 105
        let segment = input[105];

        // Validate delimiters are not alphanumeric or whitespace (common mistake detection)
        validate_delimiter(element, 3, "element separator")?;
        validate_delimiter(sub_element, 104, "sub-element separator")?;
        validate_segment_terminator(segment, 105)?;

        // All three must be distinct
        if element == sub_element || element == segment || sub_element == segment {
            return Err(ParseError::new(
                ParseErrorKind::InvalidDelimiter,
                3,
                "element, sub-element, and segment delimiters must all be distinct",
            ));
        }

        // ISA11 is the repetition separator in versions >= 00402.
        // ISA11 starts after ISA01..ISA10 elements.
        // In the fixed-width ISA: positions are:
        //   ISA*AA*...*ISA11*ISA12*...
        // We can find ISA11 by counting element separators.
        let repetition = detect_repetition_separator(input, element);

        Ok(Self {
            element,
            sub_element,
            segment,
            repetition,
        })
    }

    /// Returns `true` if the given byte is any of the active delimiters.
    pub fn is_delimiter(&self, b: u8) -> bool {
        b == self.element
            || b == self.sub_element
            || b == self.segment
            || self.repetition == Some(b)
    }
}

/// Validate that a delimiter byte is not alphanumeric.
fn validate_delimiter(b: u8, offset: usize, name: &str) -> Result<(), ParseError> {
    if b.is_ascii_alphanumeric() {
        return Err(ParseError::new(
            ParseErrorKind::InvalidDelimiter,
            offset,
            format!(
                "{name} cannot be alphanumeric: found {b:#04x} ({:?})",
                b as char
            ),
        ));
    }
    Ok(())
}

/// Validate the segment terminator — same rules as other delimiters but allow
/// newline characters since some EDI uses `\n` as segment terminator.
fn validate_segment_terminator(b: u8, offset: usize) -> Result<(), ParseError> {
    if b.is_ascii_alphanumeric() || b == b' ' {
        return Err(ParseError::new(
            ParseErrorKind::InvalidDelimiter,
            offset,
            format!(
                "segment terminator cannot be alphanumeric or space: found {b:#04x} ({:?})",
                b as char
            ),
        ));
    }
    Ok(())
}

/// Detect the repetition separator from ISA11.
///
/// ISA11 is the 12th element (0-indexed: 11). In versions < 00402, ISA11 is
/// the "Interchange Standards Identifier" (usually `U`). In >= 00402 it is
/// repurposed as the repetition separator.
///
/// We extract ISA11 by counting element separators. If ISA12 (version) starts
/// with `004` or higher and ISA11 is a single non-alphanumeric byte, we treat
/// it as the repetition separator.
fn detect_repetition_separator(input: &[u8], element_sep: u8) -> Option<u8> {
    // Count element separators to find ISA11 and ISA12 values.
    let mut sep_positions = Vec::new();
    for (i, &b) in input.iter().enumerate() {
        if b == element_sep {
            sep_positions.push(i);
        }
        // Stop after we have enough separators (we need 12 to get ISA12)
        if sep_positions.len() >= 13 {
            break;
        }
    }

    // We need at least 12 separators to access ISA11 and ISA12.
    if sep_positions.len() < 13 {
        return None;
    }

    // ISA11 is between separator 10 and separator 11
    let isa11_start = sep_positions[10] + 1;
    let isa11_end = sep_positions[11];
    let isa11 = &input[isa11_start..isa11_end];

    // ISA12 is between separator 11 and separator 12
    let isa12_start = sep_positions[11] + 1;
    let isa12_end = sep_positions[12];
    let isa12 = &input[isa12_start..isa12_end];

    // Trim ISA12 to check version
    let version_str = std::str::from_utf8(isa12).ok()?;
    let trimmed_version = version_str.trim();

    // If version >= 00402, ISA11 is the repetition separator
    if trimmed_version >= "00402" && isa11.len() == 1 && !isa11[0].is_ascii_alphanumeric() {
        Some(isa11[0])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a standard ISA segment for testing.
    fn standard_isa() -> Vec<u8> {
        // Standard 106-byte ISA segment
        let s = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        assert_eq!(s.len(), 106, "test ISA must be exactly 106 bytes");
        s.as_bytes().to_vec()
    }

    /// Build an ISA with version 00501 and repetition separator `^`.
    fn isa_with_repetition() -> Vec<u8> {
        let s = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*^*00501*000000001*0*P*:~";
        assert_eq!(s.len(), 106);
        s.as_bytes().to_vec()
    }

    #[test]
    fn detect_standard_delimiters() {
        let input = standard_isa();
        let d = Delimiters::detect(&input).unwrap();
        assert_eq!(d.element, b'*');
        assert_eq!(d.sub_element, b':');
        assert_eq!(d.segment, b'~');
        assert_eq!(d.repetition, None);
    }

    #[test]
    fn detect_repetition_separator() {
        let input = isa_with_repetition();
        let d = Delimiters::detect(&input).unwrap();
        assert_eq!(d.element, b'*');
        assert_eq!(d.sub_element, b':');
        assert_eq!(d.segment, b'~');
        assert_eq!(d.repetition, Some(b'^'));
    }

    #[test]
    fn detect_nonstandard_delimiters() {
        // Use | as element, + as sub-element, \n as segment terminator
        let s = "ISA|00|          |00|          |ZZ|SENDER         |ZZ|RECEIVER       |210901|1234|U|00401|000000001|0|P|+\n";
        assert_eq!(s.len(), 106);
        let d = Delimiters::detect(s.as_bytes()).unwrap();
        assert_eq!(d.element, b'|');
        assert_eq!(d.sub_element, b'+');
        assert_eq!(d.segment, b'\n');
    }

    #[test]
    fn too_short_input() {
        let err = Delimiters::detect(b"ISA*00*").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidIsa);
    }

    #[test]
    fn not_isa() {
        let input = vec![b'G'; 106];
        let err = Delimiters::detect(&input).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidIsa);
    }

    #[test]
    fn default_delimiters() {
        let d = Delimiters::default();
        assert_eq!(d.element, b'*');
        assert_eq!(d.sub_element, b':');
        assert_eq!(d.segment, b'~');
        assert_eq!(d.repetition, None);
    }

    #[test]
    fn duplicate_delimiters_rejected() {
        // sub-element same as element
        let mut input = standard_isa();
        input[104] = b'*'; // sub-element = element
        let err = Delimiters::detect(&input).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidDelimiter);
    }

    #[test]
    fn alphanumeric_element_separator_rejected() {
        let mut input = standard_isa();
        input[3] = b'A'; // element separator is alphanumeric
        let err = Delimiters::detect(&input).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidDelimiter);
    }

    #[test]
    fn is_delimiter() {
        let d = Delimiters {
            element: b'*',
            sub_element: b':',
            segment: b'~',
            repetition: Some(b'^'),
        };
        assert!(d.is_delimiter(b'*'));
        assert!(d.is_delimiter(b':'));
        assert!(d.is_delimiter(b'~'));
        assert!(d.is_delimiter(b'^'));
        assert!(!d.is_delimiter(b'A'));
    }
}
