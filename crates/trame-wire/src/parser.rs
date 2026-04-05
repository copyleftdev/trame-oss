//! Streaming X12 segment iterator.
//!
//! The [`Parser`] reads a byte slice and yields [`Segment`] references one at
//! a time.  It auto-detects delimiters from the ISA header (or accepts them
//! explicitly) and handles CR/LF whitespace between segments.

use crate::delimiters::Delimiters;
use crate::error::ParseError;
use crate::segment::Segment;

/// Streaming X12 parser.  Yields segments one at a time from a byte slice.
pub struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    delimiters: Delimiters,
}

impl<'a> Parser<'a> {
    /// Create a new parser that auto-detects delimiters from the ISA header.
    ///
    /// # Errors
    ///
    /// Returns a `ParseError` if the input does not start with a valid ISA
    /// segment or the delimiters cannot be detected.
    pub fn new(input: &'a [u8]) -> Result<Self, ParseError> {
        let delimiters = Delimiters::detect(input)?;
        Ok(Self {
            input,
            pos: 0,
            delimiters,
        })
    }

    /// Create a parser with explicitly provided delimiters.
    ///
    /// No ISA header validation is performed.
    pub fn with_delimiters(input: &'a [u8], delimiters: Delimiters) -> Self {
        Self {
            input,
            pos: 0,
            delimiters,
        }
    }

    /// The delimiters detected (or provided) for this parser.
    pub fn delimiters(&self) -> Delimiters {
        self.delimiters
    }

    /// The current byte offset in the input.
    pub fn offset(&self) -> usize {
        self.pos
    }

    /// Skip whitespace characters (CR, LF, space) that may appear between segments.
    fn skip_inter_segment_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b'\r' | b'\n' => self.pos += 1,
                _ => break,
            }
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Segment<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_inter_segment_whitespace();

        if self.pos >= self.input.len() {
            return None;
        }

        let start = self.pos;
        let terminator = self.delimiters.segment;

        // Scan for the segment terminator.
        while self.pos < self.input.len() && self.input[self.pos] != terminator {
            self.pos += 1;
        }

        if self.pos >= self.input.len() {
            // Reached end of input without finding a terminator.
            // If we have content, yield it as a final segment (lenient mode).
            let raw = &self.input[start..self.pos];
            if raw.is_empty() {
                return None;
            }
            // Strip trailing CR/LF from the segment
            let raw = strip_trailing_crlf(raw);
            if raw.is_empty() {
                return None;
            }
            return Some(Ok(Segment::new(
                raw,
                self.delimiters.element,
                self.delimiters.sub_element,
            )));
        }

        // We found the terminator at self.pos.
        let raw = &self.input[start..self.pos];

        // Advance past the terminator.
        self.pos += 1;

        // Strip trailing CR/LF that may precede the terminator position.
        // (Some EDI has `segment\r\n~` or `segment\r~`.)
        let raw = strip_trailing_crlf(raw);

        if raw.is_empty() {
            // Empty segment between terminators — skip and try next.
            return self.next();
        }

        Some(Ok(Segment::new(
            raw,
            self.delimiters.element,
            self.delimiters.sub_element,
        )))
    }
}

/// Strip trailing CR and LF bytes from a slice.
fn strip_trailing_crlf(mut s: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = s.split_last() {
        if last == b'\r' || last == b'\n' {
            s = rest;
        } else {
            break;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid X12 interchange for testing.
    fn minimal_x12() -> String {
        // ISA must be exactly 106 bytes including the ~ terminator.
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        assert_eq!(isa.len(), 106);
        format!(
            "{isa}\
             GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1~\
             ST*837*0001~\
             BHT*0019*00*12345*20210901*1234*CH~\
             SE*3*0001~\
             GE*1*1~\
             IEA*1*000000001~"
        )
    }

    #[test]
    fn parse_minimal_interchange() {
        let input = minimal_x12();
        let parser = Parser::new(input.as_bytes()).unwrap();
        let segments: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(segments.len(), 7);
        assert_eq!(segments[0].id_str().unwrap(), "ISA");
        assert_eq!(segments[1].id_str().unwrap(), "GS");
        assert_eq!(segments[2].id_str().unwrap(), "ST");
        assert_eq!(segments[3].id_str().unwrap(), "BHT");
        assert_eq!(segments[4].id_str().unwrap(), "SE");
        assert_eq!(segments[5].id_str().unwrap(), "GE");
        assert_eq!(segments[6].id_str().unwrap(), "IEA");
    }

    #[test]
    fn handles_crlf_between_segments() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\r\nGS*HP*S*R*20210901*1234*1*X*005010~\r\nGE*1*1~\r\nIEA*1*000000001~\r\n"
        );
        let parser = Parser::new(input.as_bytes()).unwrap();
        let segments: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].id_str().unwrap(), "ISA");
        assert_eq!(segments[1].id_str().unwrap(), "GS");
        assert_eq!(segments[2].id_str().unwrap(), "GE");
        assert_eq!(segments[3].id_str().unwrap(), "IEA");
    }

    #[test]
    fn handles_lf_between_segments() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!("{isa}\nGS*HP*S*R*20210901*1234*1*X*005010~\nIEA*1*000000001~\n");
        let parser = Parser::new(input.as_bytes()).unwrap();
        let segments: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(segments.len(), 3);
    }

    #[test]
    fn explicit_delimiters() {
        let input = b"CLM|12345|100|||11:B:1~SE|2|0001~";
        let delimiters = Delimiters {
            element: b'|',
            sub_element: b':',
            segment: b'~',
            repetition: None,
        };
        let parser = Parser::with_delimiters(input, delimiters);
        let segments: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].id(), b"CLM");
        assert_eq!(segments[1].id(), b"SE");
    }

    #[test]
    fn empty_input_with_explicit_delimiters() {
        let parser = Parser::with_delimiters(b"", Delimiters::default());
        let segments: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn segment_without_trailing_terminator() {
        let input = b"CLM*12345*100~SE*2*0001";
        let parser = Parser::with_delimiters(input, Delimiters::default());
        let segments: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].id_str().unwrap(), "SE");
    }

    #[test]
    fn delimiters_accessor() {
        let input = minimal_x12();
        let parser = Parser::new(input.as_bytes()).unwrap();
        let d = parser.delimiters();
        assert_eq!(d.element, b'*');
    }

    #[test]
    fn offset_tracking() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!("{isa}IEA*1*000000001~");
        let mut parser = Parser::new(input.as_bytes()).unwrap();
        assert_eq!(parser.offset(), 0);
        let _ = parser.next(); // ISA
        assert!(parser.offset() > 0);
    }
}
