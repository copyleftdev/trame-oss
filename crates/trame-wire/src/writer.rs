//! X12 segment serializer.
//!
//! The [`Writer`] builds X12 output byte-by-byte using the specified delimiters.
//! It can serialize raw segments, or higher-level ISA/GS/ST envelope structures.

use crate::delimiters::Delimiters;
use crate::envelope::{Gs, Isa, St};

/// An X12 segment writer that builds output into an internal buffer.
pub struct Writer {
    buf: Vec<u8>,
    delimiters: Delimiters,
}

impl Writer {
    /// Create a new writer using the given delimiters.
    pub fn new(delimiters: Delimiters) -> Self {
        Self {
            buf: Vec::new(),
            delimiters,
        }
    }

    /// Write a segment given an ID and a slice of element values.
    ///
    /// The segment ID is written first, then each element separated by the
    /// element separator, followed by the segment terminator.
    pub fn write_segment(&mut self, id: &[u8], elements: &[&[u8]]) {
        self.buf.extend_from_slice(id);
        for elem in elements {
            self.buf.push(self.delimiters.element);
            self.buf.extend_from_slice(elem);
        }
        self.buf.push(self.delimiters.segment);
    }

    /// Write an ISA segment from a parsed [`Isa`] struct.
    pub fn write_isa(&mut self, isa: &Isa<'_>) {
        self.write_segment(
            b"ISA",
            &[
                isa.auth_qualifier,
                isa.auth_info,
                isa.security_qualifier,
                isa.security_info,
                isa.sender_qualifier,
                isa.sender_id,
                isa.receiver_qualifier,
                isa.receiver_id,
                isa.date,
                isa.time,
                isa.repetition_sep,
                isa.version,
                isa.control_number,
                isa.ack_requested,
                isa.usage_indicator,
                isa.component_sep,
            ],
        );
    }

    /// Write a GS segment from a parsed [`Gs`] struct.
    pub fn write_gs(&mut self, gs: &Gs<'_>) {
        self.write_segment(
            b"GS",
            &[
                gs.functional_id,
                gs.sender_code,
                gs.receiver_code,
                gs.date,
                gs.time,
                gs.control_number,
                gs.responsible_agency,
                gs.version,
            ],
        );
    }

    /// Write an ST segment from a parsed [`St`] struct.
    pub fn write_st(&mut self, st: &St<'_>) {
        let mut elements: Vec<&[u8]> = vec![st.transaction_set_id, st.control_number];
        if let Some(impl_ref) = st.implementation_ref {
            elements.push(impl_ref);
        }
        self.write_segment(b"ST", &elements);
    }

    /// Write an SE (transaction set trailer) segment.
    pub fn write_se(&mut self, segment_count: &[u8], control_number: &[u8]) {
        self.write_segment(b"SE", &[segment_count, control_number]);
    }

    /// Write a GE (functional group trailer) segment.
    pub fn write_ge(&mut self, tx_count: &[u8], control_number: &[u8]) {
        self.write_segment(b"GE", &[tx_count, control_number]);
    }

    /// Write an IEA (interchange control trailer) segment.
    pub fn write_iea(&mut self, group_count: &[u8], control_number: &[u8]) {
        self.write_segment(b"IEA", &[group_count, control_number]);
    }

    /// Consume the writer and return the output buffer.
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    /// Borrow the output buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{Gs, Isa, St};
    use crate::interchange::Interchange;
    use crate::parser::Parser;
    use crate::segment::Segment;

    #[test]
    fn write_simple_segment() {
        let mut w = Writer::new(Delimiters::default());
        w.write_segment(b"CLM", &[b"12345", b"100", b"", b"", b"11:B:1"]);
        assert_eq!(w.as_bytes(), b"CLM*12345*100***11:B:1~");
    }

    #[test]
    fn write_isa_segment() {
        let isa = Isa {
            auth_qualifier: b"00",
            auth_info: b"          ",
            security_qualifier: b"00",
            security_info: b"          ",
            sender_qualifier: b"ZZ",
            sender_id: b"SENDER         ",
            receiver_qualifier: b"ZZ",
            receiver_id: b"RECEIVER       ",
            date: b"210901",
            time: b"1234",
            repetition_sep: b"U",
            version: b"00401",
            control_number: b"000000001",
            ack_requested: b"0",
            usage_indicator: b"P",
            component_sep: b":",
        };
        let mut w = Writer::new(Delimiters::default());
        w.write_isa(&isa);
        let output = w.finish();
        assert!(output.starts_with(b"ISA*00*"));
        assert!(output.ends_with(b"*:~"));
    }

    #[test]
    fn write_gs_segment() {
        let gs = Gs {
            functional_id: b"HP",
            sender_code: b"SENDER",
            receiver_code: b"RECEIVER",
            date: b"20210901",
            time: b"1234",
            control_number: b"1",
            responsible_agency: b"X",
            version: b"005010X222A1",
        };
        let mut w = Writer::new(Delimiters::default());
        w.write_gs(&gs);
        assert_eq!(
            w.as_bytes(),
            b"GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1~"
        );
    }

    #[test]
    fn write_st_with_impl_ref() {
        let st = St {
            transaction_set_id: b"837",
            control_number: b"0001",
            implementation_ref: Some(b"005010X222A1"),
        };
        let mut w = Writer::new(Delimiters::default());
        w.write_st(&st);
        assert_eq!(w.as_bytes(), b"ST*837*0001*005010X222A1~");
    }

    #[test]
    fn write_st_without_impl_ref() {
        let st = St {
            transaction_set_id: b"837",
            control_number: b"0001",
            implementation_ref: None,
        };
        let mut w = Writer::new(Delimiters::default());
        w.write_st(&st);
        assert_eq!(w.as_bytes(), b"ST*837*0001~");
    }

    #[test]
    fn write_trailers() {
        let mut w = Writer::new(Delimiters::default());
        w.write_se(b"4", b"0001");
        w.write_ge(b"1", b"1");
        w.write_iea(b"1", b"000000001");
        assert_eq!(w.as_bytes(), b"SE*4*0001~GE*1*1~IEA*1*000000001~");
    }

    #[test]
    fn nonstandard_delimiters() {
        let delims = Delimiters {
            element: b'|',
            sub_element: b'+',
            segment: b'\n',
            repetition: None,
        };
        let mut w = Writer::new(delims);
        w.write_segment(b"CLM", &[b"12345", b"100"]);
        assert_eq!(w.as_bytes(), b"CLM|12345|100\n");
    }

    #[test]
    fn round_trip_parse_write_parse() {
        // Build a full interchange
        let isa_raw = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
        assert_eq!(isa_raw.len(), 106);
        let input = format!(
            "{isa_raw}\
             GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1~\
             ST*837*0001*005010X222A1~\
             BHT*0019*00*12345*20210901*1234*CH~\
             CLM*PATIENT1*100***11:B:1*Y*A*Y*I~\
             SE*4*0001~\
             GE*1*1~\
             IEA*1*000000001~"
        );

        // Parse
        let interchanges = Interchange::parse(input.as_bytes()).unwrap();
        assert_eq!(interchanges.len(), 1);
        let ic = &interchanges[0];

        // Write
        let mut w = Writer::new(Delimiters::default());
        w.write_isa(&ic.isa);
        for grp in &ic.groups {
            w.write_gs(&grp.gs);
            for txn in &grp.transaction_sets {
                w.write_st(&txn.st);
                for seg in &txn.segments {
                    w.write_segment(seg.id(), &seg.elements().skip(1).collect::<Vec<_>>());
                }
                w.write_se(txn.se_segment_count, txn.se_control_number);
            }
            w.write_ge(grp.ge_tx_count, grp.ge_control_number);
        }
        w.write_iea(ic.iea_group_count, ic.iea_control_number);

        let output = w.finish();

        // Re-parse the written output
        let interchanges2 = Interchange::parse(&output).unwrap();
        assert_eq!(interchanges2.len(), 1);

        let ic2 = &interchanges2[0];
        assert_eq!(ic.isa, ic2.isa);
        assert_eq!(ic.groups.len(), ic2.groups.len());
        assert_eq!(ic.groups[0].gs, ic2.groups[0].gs);
        assert_eq!(
            ic.groups[0].transaction_sets[0].st,
            ic2.groups[0].transaction_sets[0].st
        );
        assert_eq!(
            ic.groups[0].transaction_sets[0].segments.len(),
            ic2.groups[0].transaction_sets[0].segments.len(),
        );

        // Verify segment content matches
        let orig_segs = &ic.groups[0].transaction_sets[0].segments;
        let new_segs = &ic2.groups[0].transaction_sets[0].segments;
        for (a, b) in orig_segs.iter().zip(new_segs.iter()) {
            assert_eq!(a.id(), b.id());
            assert_eq!(a.element_count(), b.element_count());
        }
    }

    #[test]
    fn round_trip_segments_via_parser() {
        // Write segments, parse them back, verify identity.
        let delims = Delimiters::default();
        let mut w = Writer::new(delims);
        w.write_segment(
            b"NM1",
            &[b"IL", b"1", b"DOE", b"JOHN", b"", b"", b"", b"MI", b"12345"],
        );
        w.write_segment(b"DTP", &[b"472", b"D8", b"20210901"]);

        let output = w.finish();
        let parser = Parser::with_delimiters(&output, delims);
        let segs: Vec<Segment<'_>> = parser.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].id_str().unwrap(), "NM1");
        assert_eq!(segs[0].element_str(1), Some("IL"));
        assert_eq!(segs[0].element_str(3), Some("DOE"));
        assert_eq!(segs[0].element_str(4), Some("JOHN"));
        assert_eq!(segs[0].element_str(5), Some(""));
        assert_eq!(segs[0].element_str(8), Some("MI"));
        assert_eq!(segs[0].element_str(9), Some("12345"));

        assert_eq!(segs[1].id_str().unwrap(), "DTP");
        assert_eq!(segs[1].element_str(2), Some("D8"));
        assert_eq!(segs[1].element_str(3), Some("20210901"));
    }
}
