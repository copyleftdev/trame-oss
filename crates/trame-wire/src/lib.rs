//! `trame-wire` — Zero-copy, streaming X12 EDI parser.
//!
//! This crate provides the foundational parsing layer for the trame project.
//! It reads raw X12 bytes and produces zero-copy segment references without
//! any heap allocation during parsing.
//!
//! # Architecture
//!
//! The parser operates at three levels of abstraction:
//!
//! 1. **Segment level** ([`parser::Parser`]) — Yields individual [`segment::Segment`]
//!    references from a byte slice. This is the lowest level and is suitable for
//!    streaming use cases.
//!
//! 2. **Envelope level** ([`envelope`]) — Typed structs for ISA, GS, and ST headers
//!    that can be parsed from individual segments.
//!
//! 3. **Interchange level** ([`interchange::Interchange`]) — Groups segments into
//!    the full X12 envelope hierarchy (ISA/GS/ST/SE/GE/IEA).
//!
//! # Quick Start
//!
//! ```rust
//! use trame_wire::{Parser, Interchange, parse_interchanges};
//!
//! // Low-level: iterate segments
//! let input = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~GS*HP*S*R*20210901*1234*1*X*005010~ST*837*0001~SE*1*0001~GE*1*1~IEA*1*000000001~";
//! let parser = Parser::new(input).unwrap();
//! for segment in parser {
//!     let seg = segment.unwrap();
//!     println!("{}", seg.id_str().unwrap_or("???"));
//! }
//!
//! // High-level: parse full interchange structure
//! let interchanges = parse_interchanges(input).unwrap();
//! assert_eq!(interchanges.len(), 1);
//! ```

#![forbid(unsafe_code)]

pub mod delimiters;
pub mod envelope;
pub mod error;
pub mod interchange;
pub mod parser;
pub mod segment;
pub mod writer;

// Re-exports for convenience.
pub use delimiters::Delimiters;
pub use envelope::{Gs, Isa, St};
pub use error::{ParseError, ParseErrorKind};
pub use interchange::{parse_interchanges, FunctionalGroup, Interchange, TransactionSet};
pub use parser::Parser;
pub use segment::{ElementIter, Segment, SubElementIter};
pub use writer::Writer;
