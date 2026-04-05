//! ISA/GS/ST envelope parsing.
//!
//! Provides typed, zero-copy structures for the three X12 envelope headers
//! (ISA, GS, ST) and their corresponding trailers (IEA, GE, SE).

use crate::error::{ParseError, ParseErrorKind};
use crate::segment::Segment;

/// Parsed ISA interchange control header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isa<'a> {
    /// ISA01 — Authorization Information Qualifier.
    pub auth_qualifier: &'a [u8],
    /// ISA02 — Authorization Information.
    pub auth_info: &'a [u8],
    /// ISA03 — Security Information Qualifier.
    pub security_qualifier: &'a [u8],
    /// ISA04 — Security Information.
    pub security_info: &'a [u8],
    /// ISA05 — Interchange ID Qualifier (sender).
    pub sender_qualifier: &'a [u8],
    /// ISA06 — Interchange Sender ID.
    pub sender_id: &'a [u8],
    /// ISA07 — Interchange ID Qualifier (receiver).
    pub receiver_qualifier: &'a [u8],
    /// ISA08 — Interchange Receiver ID.
    pub receiver_id: &'a [u8],
    /// ISA09 — Interchange Date (YYMMDD).
    pub date: &'a [u8],
    /// ISA10 — Interchange Time (HHMM).
    pub time: &'a [u8],
    /// ISA11 — Repetition Separator (or Interchange Standards Identifier).
    pub repetition_sep: &'a [u8],
    /// ISA12 — Interchange Control Version Number.
    pub version: &'a [u8],
    /// ISA13 — Interchange Control Number.
    pub control_number: &'a [u8],
    /// ISA14 — Acknowledgment Requested.
    pub ack_requested: &'a [u8],
    /// ISA15 — Usage Indicator (P = Production, T = Test).
    pub usage_indicator: &'a [u8],
    /// ISA16 — Component Element Separator.
    pub component_sep: &'a [u8],
}

impl<'a> Isa<'a> {
    /// Parse an ISA segment into a typed struct.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the segment is not an ISA or has too few elements.
    pub fn parse(segment: &Segment<'a>) -> Result<Self, ParseError> {
        if segment.id() != b"ISA" {
            return Err(ParseError::new(
                ParseErrorKind::InvalidEnvelope,
                0,
                format!("expected ISA segment, got {:?}", segment.id_str().unwrap_or("<non-utf8>")),
            ));
        }

        // ISA has exactly 16 data elements (+ the segment ID = 17 total).
        let count = segment.element_count();
        if count < 17 {
            return Err(ParseError::new(
                ParseErrorKind::InvalidIsa,
                0,
                format!("ISA requires 17 elements (ID + 16 fields), got {count}"),
            ));
        }

        Ok(Self {
            auth_qualifier: segment.element(1).unwrap_or_default(),
            auth_info: segment.element(2).unwrap_or_default(),
            security_qualifier: segment.element(3).unwrap_or_default(),
            security_info: segment.element(4).unwrap_or_default(),
            sender_qualifier: segment.element(5).unwrap_or_default(),
            sender_id: segment.element(6).unwrap_or_default(),
            receiver_qualifier: segment.element(7).unwrap_or_default(),
            receiver_id: segment.element(8).unwrap_or_default(),
            date: segment.element(9).unwrap_or_default(),
            time: segment.element(10).unwrap_or_default(),
            repetition_sep: segment.element(11).unwrap_or_default(),
            version: segment.element(12).unwrap_or_default(),
            control_number: segment.element(13).unwrap_or_default(),
            ack_requested: segment.element(14).unwrap_or_default(),
            usage_indicator: segment.element(15).unwrap_or_default(),
            component_sep: segment.element(16).unwrap_or_default(),
        })
    }
}

/// Parsed GS functional group header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gs<'a> {
    /// GS01 — Functional Identifier Code.
    pub functional_id: &'a [u8],
    /// GS02 — Application Sender's Code.
    pub sender_code: &'a [u8],
    /// GS03 — Application Receiver's Code.
    pub receiver_code: &'a [u8],
    /// GS04 — Date (CCYYMMDD).
    pub date: &'a [u8],
    /// GS05 — Time (HHMM or HHMMSS or HHMMSSD).
    pub time: &'a [u8],
    /// GS06 — Group Control Number.
    pub control_number: &'a [u8],
    /// GS07 — Responsible Agency Code.
    pub responsible_agency: &'a [u8],
    /// GS08 — Version / Release / Industry Identifier Code.
    pub version: &'a [u8],
}

impl<'a> Gs<'a> {
    /// Parse a GS segment into a typed struct.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the segment is not a GS or has too few elements.
    pub fn parse(segment: &Segment<'a>) -> Result<Self, ParseError> {
        if segment.id() != b"GS" {
            return Err(ParseError::new(
                ParseErrorKind::InvalidEnvelope,
                0,
                format!("expected GS segment, got {:?}", segment.id_str().unwrap_or("<non-utf8>")),
            ));
        }

        let count = segment.element_count();
        if count < 9 {
            return Err(ParseError::new(
                ParseErrorKind::InvalidEnvelope,
                0,
                format!("GS requires 9 elements (ID + 8 fields), got {count}"),
            ));
        }

        Ok(Self {
            functional_id: segment.element(1).unwrap_or_default(),
            sender_code: segment.element(2).unwrap_or_default(),
            receiver_code: segment.element(3).unwrap_or_default(),
            date: segment.element(4).unwrap_or_default(),
            time: segment.element(5).unwrap_or_default(),
            control_number: segment.element(6).unwrap_or_default(),
            responsible_agency: segment.element(7).unwrap_or_default(),
            version: segment.element(8).unwrap_or_default(),
        })
    }
}

/// Parsed ST transaction set header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct St<'a> {
    /// ST01 — Transaction Set Identifier Code (e.g., `837`, `835`, `270`).
    pub transaction_set_id: &'a [u8],
    /// ST02 — Transaction Set Control Number.
    pub control_number: &'a [u8],
    /// ST03 — Implementation Convention Reference (optional, 5010+).
    pub implementation_ref: Option<&'a [u8]>,
}

impl<'a> St<'a> {
    /// Parse an ST segment into a typed struct.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the segment is not an ST or has too few elements.
    pub fn parse(segment: &Segment<'a>) -> Result<Self, ParseError> {
        if segment.id() != b"ST" {
            return Err(ParseError::new(
                ParseErrorKind::InvalidEnvelope,
                0,
                format!("expected ST segment, got {:?}", segment.id_str().unwrap_or("<non-utf8>")),
            ));
        }

        let count = segment.element_count();
        if count < 3 {
            return Err(ParseError::new(
                ParseErrorKind::InvalidEnvelope,
                0,
                format!("ST requires at least 3 elements (ID + 2 fields), got {count}"),
            ));
        }

        let impl_ref = segment.element(3).and_then(|e| {
            if e.is_empty() {
                None
            } else {
                Some(e)
            }
        });

        Ok(Self {
            transaction_set_id: segment.element(1).unwrap_or_default(),
            control_number: segment.element(2).unwrap_or_default(),
            implementation_ref: impl_ref,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::Segment;

    fn seg(raw: &[u8]) -> Segment<'_> {
        Segment::new(raw, b'*', b':')
    }

    #[test]
    fn parse_isa() {
        let raw = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:";
        let s = seg(raw);
        let isa = Isa::parse(&s).unwrap();
        assert_eq!(isa.auth_qualifier, b"00");
        assert_eq!(isa.sender_qualifier, b"ZZ");
        assert_eq!(isa.sender_id, b"SENDER         ");
        assert_eq!(isa.receiver_id, b"RECEIVER       ");
        assert_eq!(isa.date, b"210901");
        assert_eq!(isa.time, b"1234");
        assert_eq!(isa.version, b"00401");
        assert_eq!(isa.control_number, b"000000001");
        assert_eq!(isa.usage_indicator, b"P");
        assert_eq!(isa.component_sep, b":");
    }

    #[test]
    fn parse_isa_wrong_segment() {
        let s = seg(b"GS*HP*SENDER*RECEIVER");
        let err = Isa::parse(&s).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidEnvelope);
    }

    #[test]
    fn parse_isa_too_few_elements() {
        let s = seg(b"ISA*00*          *00");
        let err = Isa::parse(&s).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidIsa);
    }

    #[test]
    fn parse_gs() {
        let s = seg(b"GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1");
        let gs = Gs::parse(&s).unwrap();
        assert_eq!(gs.functional_id, b"HP");
        assert_eq!(gs.sender_code, b"SENDER");
        assert_eq!(gs.receiver_code, b"RECEIVER");
        assert_eq!(gs.date, b"20210901");
        assert_eq!(gs.control_number, b"1");
        assert_eq!(gs.version, b"005010X222A1");
    }

    #[test]
    fn parse_gs_wrong_segment() {
        let s = seg(b"ISA*00*stuff");
        let err = Gs::parse(&s).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidEnvelope);
    }

    #[test]
    fn parse_gs_too_few_elements() {
        let s = seg(b"GS*HP*SENDER");
        let err = Gs::parse(&s).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidEnvelope);
    }

    #[test]
    fn parse_st() {
        let s = seg(b"ST*837*0001*005010X222A1");
        let st = St::parse(&s).unwrap();
        assert_eq!(st.transaction_set_id, b"837");
        assert_eq!(st.control_number, b"0001");
        assert_eq!(st.implementation_ref, Some(b"005010X222A1".as_ref()));
    }

    #[test]
    fn parse_st_no_impl_ref() {
        let s = seg(b"ST*837*0001");
        let st = St::parse(&s).unwrap();
        assert_eq!(st.transaction_set_id, b"837");
        assert_eq!(st.control_number, b"0001");
        assert_eq!(st.implementation_ref, None);
    }

    #[test]
    fn parse_st_empty_impl_ref() {
        let s = seg(b"ST*837*0001*");
        let st = St::parse(&s).unwrap();
        assert_eq!(st.implementation_ref, None);
    }

    #[test]
    fn parse_st_wrong_segment() {
        let s = seg(b"SE*5*0001");
        let err = St::parse(&s).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidEnvelope);
    }

    #[test]
    fn parse_st_too_few_elements() {
        let s = seg(b"ST*837");
        let err = St::parse(&s).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidEnvelope);
    }
}
