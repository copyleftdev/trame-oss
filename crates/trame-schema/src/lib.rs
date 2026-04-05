#![forbid(unsafe_code)]
//! X12 transaction set grammars and schema definitions.
//!
//! This crate provides:
//! - Type definitions for X12 schemas (segments, elements, loops, composites)
//! - A schema registry with built-in definitions for common transaction sets
//! - A walker/state machine for traversing X12 documents against a schema
//!
//! # Supported Transaction Sets
//!
//! Built-in schemas are provided for:
//! - 837P (Professional Claim) — 005010X222A1
//! - 835 (Remittance Advice) — 005010X221A1
//! - 850 (Purchase Order) — 004010
//! - 270 (Eligibility Inquiry) — 005010X279A1
//! - 997 (Functional Acknowledgment)

pub mod catalog;
pub mod types;
pub mod walker;

pub use catalog::Registry;
pub use types::*;
pub use walker::SchemaWalker;
