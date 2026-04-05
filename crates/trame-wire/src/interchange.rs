//! Full interchange reader.
//!
//! Groups raw segments into the X12 envelope hierarchy:
//! `Interchange` (ISA...IEA) -> `FunctionalGroup` (GS...GE) -> `TransactionSet` (ST...SE).

use crate::envelope::{Gs, Isa, St};
use crate::error::{ParseError, ParseErrorKind};
use crate::parser::Parser;
use crate::segment::Segment;

/// A complete X12 interchange (ISA...IEA).
#[derive(Debug)]
pub struct Interchange<'a> {
    /// The parsed ISA header.
    pub isa: Isa<'a>,
    /// Functional groups contained in this interchange.
    pub groups: Vec<FunctionalGroup<'a>>,
    /// IEA01 — Number of included functional groups.
    pub iea_group_count: &'a [u8],
    /// IEA02 — Interchange control number (should match ISA13).
    pub iea_control_number: &'a [u8],
}

/// A functional group (GS...GE) within an interchange.
#[derive(Debug)]
pub struct FunctionalGroup<'a> {
    /// The parsed GS header.
    pub gs: Gs<'a>,
    /// Transaction sets contained in this group.
    pub transaction_sets: Vec<TransactionSet<'a>>,
    /// GE02 — Group control number (should match GS06).
    pub ge_control_number: &'a [u8],
    /// GE01 — Number of transaction sets included.
    pub ge_tx_count: &'a [u8],
}

/// A transaction set (ST...SE) within a functional group.
#[derive(Debug)]
pub struct TransactionSet<'a> {
    /// The parsed ST header.
    pub st: St<'a>,
    /// All segments between ST and SE (exclusive of ST and SE themselves).
    pub segments: Vec<Segment<'a>>,
    /// SE01 — Number of included segments (including ST and SE).
    pub se_segment_count: &'a [u8],
    /// SE02 — Transaction set control number (should match ST02).
    pub se_control_number: &'a [u8],
}

/// Parse all interchanges from an X12 byte buffer.
///
/// This is a convenience wrapper around [`Interchange::parse`].
pub fn parse_interchanges(input: &[u8]) -> Result<Vec<Interchange<'_>>, ParseError> {
    Interchange::parse(input)
}

/// Expect a specific segment ID at `idx`, returning an error if it doesn't match.
fn expect_segment_id(
    segments: &[Segment<'_>],
    idx: usize,
    expected: &str,
    context: &str,
) -> Result<(), ParseError> {
    if idx >= segments.len() {
        return Err(ParseError::new(
            ParseErrorKind::UnexpectedEndOfInput,
            0,
            format!("expected {expected} segment to close {context}, but reached end of input"),
        ));
    }
    let actual = segments[idx].id_str().unwrap_or("<non-utf8>");
    if actual != expected {
        return Err(ParseError::new(
            ParseErrorKind::InvalidEnvelope,
            0,
            format!("expected {expected} segment at position {idx}, found {actual:?}"),
        ));
    }
    Ok(())
}

/// Parse a single transaction set (ST...SE) starting at `idx`.
/// Returns the transaction set and the next index.
fn parse_transaction_set<'a>(
    segments: &[Segment<'a>],
    mut idx: usize,
) -> Result<(TransactionSet<'a>, usize), ParseError> {
    expect_segment_id(segments, idx, "ST", "functional group")?;
    let st = St::parse(&segments[idx])?;
    idx += 1;

    let mut body = Vec::new();
    while idx < segments.len() && segments[idx].id() != b"SE" {
        body.push(segments[idx]);
        idx += 1;
    }

    expect_segment_id(segments, idx, "SE", "transaction set")?;
    let trailer = &segments[idx];
    idx += 1;

    Ok((
        TransactionSet {
            st,
            segments: body,
            se_segment_count: trailer.element(1).unwrap_or_default(),
            se_control_number: trailer.element(2).unwrap_or_default(),
        },
        idx,
    ))
}

/// Parse a single functional group (GS...GE) starting at `idx`.
/// Returns the group and the next index.
fn parse_functional_group<'a>(
    segments: &[Segment<'a>],
    mut idx: usize,
) -> Result<(FunctionalGroup<'a>, usize), ParseError> {
    expect_segment_id(segments, idx, "GS", "interchange")?;
    let gs = Gs::parse(&segments[idx])?;
    idx += 1;

    let mut transaction_sets = Vec::new();
    while idx < segments.len() && segments[idx].id() != b"GE" {
        let (txn, next) = parse_transaction_set(segments, idx)?;
        transaction_sets.push(txn);
        idx = next;
    }

    expect_segment_id(segments, idx, "GE", "functional group")?;
    let trailer = &segments[idx];
    idx += 1;

    Ok((
        FunctionalGroup {
            gs,
            transaction_sets,
            ge_tx_count: trailer.element(1).unwrap_or_default(),
            ge_control_number: trailer.element(2).unwrap_or_default(),
        },
        idx,
    ))
}

impl<'a> Interchange<'a> {
    /// Parse all interchanges from the input.
    ///
    /// The input may contain multiple consecutive ISA...IEA interchanges.
    ///
    /// # Errors
    ///
    /// Returns a `ParseError` on structural problems: missing/mismatched
    /// envelopes, bad control numbers, etc.
    pub fn parse(input: &'a [u8]) -> Result<Vec<Self>, ParseError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let parser = Parser::new(input)?;
        let segments: Vec<Segment<'a>> = parser.collect::<Result<Vec<_>, _>>()?;

        let mut interchanges = Vec::new();
        let mut idx = 0;

        while idx < segments.len() {
            expect_segment_id(&segments, idx, "ISA", "input")?;
            let isa = Isa::parse(&segments[idx])?;
            idx += 1;

            let mut groups = Vec::new();
            while idx < segments.len() && segments[idx].id() != b"IEA" {
                let (group, next) = parse_functional_group(&segments, idx)?;
                groups.push(group);
                idx = next;
            }

            expect_segment_id(&segments, idx, "IEA", "interchange")?;
            let trailer = &segments[idx];
            idx += 1;

            interchanges.push(Interchange {
                isa,
                groups,
                iea_group_count: trailer.element(1).unwrap_or_default(),
                iea_control_number: trailer.element(2).unwrap_or_default(),
            });
        }

        Ok(interchanges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_x12() -> String {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        assert_eq!(isa.len(), 106);
        format!(
            "{isa}\
             GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1~\
             ST*837*0001*005010X222A1~\
             BHT*0019*00*12345*20210901*1234*CH~\
             CLM*PATIENT1*100***11:B:1*Y*A*Y*I~\
             SE*4*0001~\
             GE*1*1~\
             IEA*1*000000001~"
        )
    }

    #[test]
    fn parse_single_interchange() {
        let input = minimal_x12();
        let interchanges = Interchange::parse(input.as_bytes()).unwrap();

        assert_eq!(interchanges.len(), 1);
        let ic = &interchanges[0];
        assert_eq!(ic.isa.sender_qualifier, b"ZZ");
        assert_eq!(ic.isa.control_number, b"000000001");
        assert_eq!(ic.iea_control_number, b"000000001");
        assert_eq!(ic.iea_group_count, b"1");

        assert_eq!(ic.groups.len(), 1);
        let grp = &ic.groups[0];
        assert_eq!(grp.gs.functional_id, b"HP");
        assert_eq!(grp.gs.control_number, b"1");
        assert_eq!(grp.ge_control_number, b"1");
        assert_eq!(grp.ge_tx_count, b"1");

        assert_eq!(grp.transaction_sets.len(), 1);
        let txn = &grp.transaction_sets[0];
        assert_eq!(txn.st.transaction_set_id, b"837");
        assert_eq!(txn.st.control_number, b"0001");
        assert_eq!(txn.st.implementation_ref, Some(b"005010X222A1".as_ref()));
        assert_eq!(txn.segments.len(), 2); // BHT + CLM
        assert_eq!(txn.segments[0].id_str().unwrap(), "BHT");
        assert_eq!(txn.segments[1].id_str().unwrap(), "CLM");
        assert_eq!(txn.se_segment_count, b"4");
        assert_eq!(txn.se_control_number, b"0001");
    }

    #[test]
    fn parse_multiple_transaction_sets() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             BHT*0019*00*A~\
             SE*2*0001~\
             ST*837*0002~\
             BHT*0019*00*B~\
             SE*2*0002~\
             GE*2*1~\
             IEA*1*000000001~"
        );
        let interchanges = Interchange::parse(input.as_bytes()).unwrap();
        assert_eq!(interchanges.len(), 1);
        assert_eq!(interchanges[0].groups[0].transaction_sets.len(), 2);
        assert_eq!(
            interchanges[0].groups[0].transaction_sets[0]
                .st
                .control_number,
            b"0001"
        );
        assert_eq!(
            interchanges[0].groups[0].transaction_sets[1]
                .st
                .control_number,
            b"0002"
        );
    }

    #[test]
    fn parse_multiple_groups() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             SE*1*0001~\
             GE*1*1~\
             GS*FA*S*R*20210901*1234*2*X*005010~\
             ST*999*0001~\
             SE*1*0001~\
             GE*1*2~\
             IEA*2*000000001~"
        );
        let interchanges = Interchange::parse(input.as_bytes()).unwrap();
        assert_eq!(interchanges[0].groups.len(), 2);
        assert_eq!(interchanges[0].groups[0].gs.functional_id, b"HP");
        assert_eq!(interchanges[0].groups[1].gs.functional_id, b"FA");
    }

    #[test]
    fn parse_multiple_interchanges() {
        let isa1 = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let isa2 = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000002*0*P*:~";
        let input = format!(
            "{isa1}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             SE*1*0001~\
             GE*1*1~\
             IEA*1*000000001~\
             {isa2}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             SE*1*0001~\
             GE*1*1~\
             IEA*1*000000002~"
        );
        let interchanges = Interchange::parse(input.as_bytes()).unwrap();
        assert_eq!(interchanges.len(), 2);
        assert_eq!(interchanges[0].isa.control_number, b"000000001");
        assert_eq!(interchanges[1].isa.control_number, b"000000002");
    }

    #[test]
    fn missing_iea() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             SE*1*0001~\
             GE*1*1~"
        );
        let err = Interchange::parse(input.as_bytes()).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEndOfInput);
    }

    #[test]
    fn missing_ge() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             SE*1*0001~\
             IEA*1*000000001~"
        );
        // The parser sees IEA where it expects GE, so it's an envelope error
        let err = Interchange::parse(input.as_bytes()).unwrap_err();
        assert!(
            err.kind == ParseErrorKind::InvalidEnvelope
                || err.kind == ParseErrorKind::UnexpectedEndOfInput
        );
    }

    #[test]
    fn missing_se() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\
             GS*HP*S*R*20210901*1234*1*X*005010~\
             ST*837*0001~\
             BHT*0019*00*A~\
             GE*1*1~\
             IEA*1*000000001~"
        );
        // GE appears where SE is expected — this becomes an envelope error
        // because the parser expects either content segments or SE.
        // Actually, BHT will be collected, then GE is not SE, so we keep collecting.
        // Then IEA is not SE, so we keep collecting. Then we hit end of input.
        let err = Interchange::parse(input.as_bytes()).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEndOfInput);
    }

    #[test]
    fn empty_input() {
        let interchanges = Interchange::parse(b"").unwrap();
        assert!(interchanges.is_empty());
    }

    #[test]
    fn convenience_function() {
        let input = minimal_x12();
        let interchanges = parse_interchanges(input.as_bytes()).unwrap();
        assert_eq!(interchanges.len(), 1);
    }

    #[test]
    fn with_crlf_line_endings() {
        let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        let input = format!(
            "{isa}\r\n\
             GS*HP*S*R*20210901*1234*1*X*005010~\r\n\
             ST*837*0001~\r\n\
             BHT*0019*00*A~\r\n\
             SE*2*0001~\r\n\
             GE*1*1~\r\n\
             IEA*1*000000001~\r\n"
        );
        let interchanges = Interchange::parse(input.as_bytes()).unwrap();
        assert_eq!(interchanges.len(), 1);
        assert_eq!(
            interchanges[0].groups[0].transaction_sets[0].segments.len(),
            1
        );
    }
}
