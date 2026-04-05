//! Core type definitions for X12 schema modeling.
//!
//! All types use `&'static` references so that schema definitions can be
//! `const`/`static` data with zero heap allocation.

/// X12 element data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// AN — Alphanumeric string.
    Alphanumeric,
    /// Nn — Numeric with n implied decimal places (0..6).
    Numeric { decimal_places: u8 },
    /// R — Decimal number (explicit decimal point).
    Decimal,
    /// ID — Identifier from a code set.
    Identifier,
    /// DT — Date (CCYYMMDD or YYMMDD).
    Date,
    /// TM — Time (HHMM, HHMMSS, or HHMMSSDD).
    Time,
    /// B — Binary data.
    Binary,
}

/// Usage indicator for segments and elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    /// Must be present.
    Required,
    /// Must be present when condition is met.
    Situational,
    /// Must not be present.
    NotUsed,
}

/// Syntax rule type (relational conditions between elements within a segment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxRuleKind {
    /// P — If any in set is present, ALL must be present.
    Paired,
    /// R — At least one in set must be present.
    AtLeastOne,
    /// E — No more than one in set may be present.
    Exclusive,
    /// C — If first is present, all others must be present.
    Conditional,
    /// L — If first is present, at least one other must be present.
    ListConditional,
}

/// A syntax rule referencing element positions within a segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRule {
    pub kind: SyntaxRuleKind,
    /// 1-indexed element positions within the segment.
    pub positions: Vec<u16>,
}

/// Definition of a component within a composite element.
#[derive(Debug, Clone)]
pub struct ComponentDef {
    /// Position within the composite (1-indexed).
    pub position: u16,
    /// Data dictionary element reference (e.g., "235", "234").
    pub element_ref: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    pub data_type: DataType,
    pub min_length: u16,
    pub max_length: u16,
    pub usage: Usage,
}

/// Definition of a composite element (e.g., C003, C022).
#[derive(Debug, Clone)]
pub struct CompositeDef {
    /// Composite ID (e.g., "C003").
    pub id: &'static str,
    pub name: &'static str,
    pub components: &'static [ComponentDef],
}

/// Definition of an element within a segment.
#[derive(Debug, Clone)]
pub struct ElementDef {
    /// Position within segment (1-indexed; 0 is always the segment ID).
    pub position: u16,
    /// Data dictionary element reference (e.g., "98", "1035").
    pub element_ref: &'static str,
    pub name: &'static str,
    pub data_type: DataType,
    pub min_length: u16,
    pub max_length: u16,
    pub usage: Usage,
    /// If this element is a composite.
    pub composite: Option<&'static CompositeDef>,
    /// Valid code values for Identifier type elements. Empty slice if unrestricted.
    pub code_values: &'static [(&'static str, &'static str)],
}

/// Definition of a segment within the X12 data dictionary.
#[derive(Debug, Clone)]
pub struct SegmentDef {
    /// Segment identifier (e.g., "NM1", "CLM", "SV1").
    pub id: &'static str,
    pub name: &'static str,
    pub elements: &'static [ElementDef],
    pub syntax_rules: &'static [SyntaxRule],
}

/// Reference to a segment within a loop, with usage overrides from an IG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentRef {
    /// Segment identifier.
    pub segment_id: &'static str,
    pub usage: Usage,
    /// Maximum number of times this segment can appear at this position.
    pub max_use: u32,
    /// Ordinal position within the loop.
    pub position: u32,
}

/// Qualifier match for disambiguating loops sharing the same trigger segment.
#[derive(Debug, Clone)]
pub struct QualifierMatch {
    /// Which element to check (1-indexed).
    pub element_position: u16,
    /// Valid values that identify this loop.
    pub values: &'static [&'static str],
}

/// Definition of a loop within a transaction set.
#[derive(Debug, Clone)]
pub struct LoopDef {
    /// Loop identifier (e.g., "2000A", "2300", "2400").
    pub id: &'static str,
    pub name: &'static str,
    pub usage: Usage,
    /// Maximum repeat count (`u32::MAX` for unbounded).
    pub repeat_count: u32,
    /// The segment ID that triggers a new loop iteration.
    pub trigger_segment: &'static str,
    /// Optional qualifier match to disambiguate loops with the same trigger.
    pub qualifier: Option<QualifierMatch>,
    /// Segments in this loop (in order).
    pub segments: &'static [SegmentRef],
    /// Nested child loops (in order).
    pub children: &'static [LoopDef],
}

/// Definition of a complete transaction set.
#[derive(Debug, Clone)]
pub struct TransactionSetDef {
    /// Transaction set ID (e.g., "837", "835", "270").
    pub id: &'static str,
    pub name: &'static str,
    /// Functional group ID for GS01 (e.g., "HC", "HP", "HB").
    pub functional_group_id: &'static str,
    /// X12 version (e.g., "005010").
    pub version: &'static str,
    /// Implementation guide reference (e.g., "005010X222A1").
    pub implementation_ref: Option<&'static str>,
    /// Top-level segments before any loops (e.g., ST, BHT).
    pub header_segments: &'static [SegmentRef],
    /// Header table loops.
    pub header_loops: &'static [LoopDef],
    /// Detail table loops.
    pub detail_loops: &'static [LoopDef],
    /// Summary table loops.
    pub summary_loops: &'static [LoopDef],
    /// Trailer segments (SE).
    pub trailer_segments: &'static [SegmentRef],
}
