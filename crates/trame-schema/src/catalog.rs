//! Built-in transaction set definitions and schema registry.
//!
//! Provides static schema definitions for common X12 transaction sets
//! and a [`Registry`] for looking them up by ID, version, or implementation
//! guide reference.

use crate::types::{LoopDef, QualifierMatch, SegmentRef, TransactionSetDef, Usage};

// ---------------------------------------------------------------------------
// 997 — Functional Acknowledgment
// ---------------------------------------------------------------------------

static TS997_HEADER_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "ST",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "AK1",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
];

static TS997_AK3_CHILDREN: &[LoopDef] = &[];

static TS997_AK3_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "AK3",
        usage: Usage::Situational,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "AK4",
        usage: Usage::Situational,
        max_use: 99,
        position: 20,
    },
];

static TS997_AK2_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "AK3",
    name: "Data Segment Note",
    usage: Usage::Situational,
    repeat_count: 999_999,
    trigger_segment: "AK3",
    qualifier: None,
    segments: TS997_AK3_SEGMENTS,
    children: TS997_AK3_CHILDREN,
}];

static TS997_AK2_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "AK2",
        usage: Usage::Situational,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "AK5",
        usage: Usage::Required,
        max_use: 1,
        position: 40,
    },
];

static TS997_DETAIL_LOOPS: &[LoopDef] = &[LoopDef {
    id: "AK2",
    name: "Transaction Set Response Header",
    usage: Usage::Situational,
    repeat_count: 999_999,
    trigger_segment: "AK2",
    qualifier: None,
    segments: TS997_AK2_SEGMENTS,
    children: TS997_AK2_CHILDREN,
}];

static TS997_TRAILER_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "AK9",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "SE",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
];

static TS997: TransactionSetDef = TransactionSetDef {
    id: "997",
    name: "Functional Acknowledgment",
    functional_group_id: "FA",
    version: "005010",
    implementation_ref: None,
    header_segments: TS997_HEADER_SEGMENTS,
    header_loops: &[],
    detail_loops: TS997_DETAIL_LOOPS,
    summary_loops: &[],
    trailer_segments: TS997_TRAILER_SEGMENTS,
};

// ---------------------------------------------------------------------------
// 270 — Eligibility, Coverage, or Benefit Inquiry — 005010X279A1
// ---------------------------------------------------------------------------

static TS270_HEADER_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "ST",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "BHT",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
];

// Loop 2100A — Information Source Name
static TS270_2100A_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "NM1",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

// Loop 2100B — Information Receiver Name
static TS270_2100B_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 9,
        position: 20,
    },
];

// Loop 2110C — Eligibility/Benefit Inquiry
static TS270_2110C_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "EQ",
        usage: Usage::Situational,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "DTP",
        usage: Usage::Situational,
        max_use: 2,
        position: 30,
    },
];

// Loop 2100C — Subscriber Name
static TS270_2100C_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 9,
        position: 20,
    },
    SegmentRef {
        segment_id: "DMG",
        usage: Usage::Situational,
        max_use: 1,
        position: 40,
    },
    SegmentRef {
        segment_id: "DTP",
        usage: Usage::Situational,
        max_use: 1,
        position: 50,
    },
];

// Loop 2000A — Information Source
static TS270_2000A_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "HL",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS270_2000A_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "2100A",
    name: "Information Source Name",
    usage: Usage::Required,
    repeat_count: 1,
    trigger_segment: "NM1",
    qualifier: None,
    segments: TS270_2100A_SEGMENTS,
    children: &[],
}];

// Loop 2000B — Information Receiver
static TS270_2000B_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "HL",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS270_2000B_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "2100B",
    name: "Information Receiver Name",
    usage: Usage::Required,
    repeat_count: 1,
    trigger_segment: "NM1",
    qualifier: None,
    segments: TS270_2100B_SEGMENTS,
    children: &[],
}];

// Loop 2000C — Subscriber
static TS270_2000C_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "HL",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "TRN",
        usage: Usage::Situational,
        max_use: 9,
        position: 20,
    },
];

static TS270_2000C_CHILDREN: &[LoopDef] = &[
    LoopDef {
        id: "2100C",
        name: "Subscriber Name",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: None,
        segments: TS270_2100C_SEGMENTS,
        children: &[],
    },
    LoopDef {
        id: "2110C",
        name: "Eligibility/Benefit Inquiry",
        usage: Usage::Situational,
        repeat_count: 99,
        trigger_segment: "EQ",
        qualifier: None,
        segments: TS270_2110C_SEGMENTS,
        children: &[],
    },
];

static TS270_DETAIL_LOOPS: &[LoopDef] = &[
    LoopDef {
        id: "2000A",
        name: "Information Source",
        usage: Usage::Required,
        repeat_count: u32::MAX,
        trigger_segment: "HL",
        qualifier: Some(QualifierMatch {
            element_position: 3,
            values: &["20"],
        }),
        segments: TS270_2000A_SEGMENTS,
        children: TS270_2000A_CHILDREN,
    },
    LoopDef {
        id: "2000B",
        name: "Information Receiver",
        usage: Usage::Required,
        repeat_count: u32::MAX,
        trigger_segment: "HL",
        qualifier: Some(QualifierMatch {
            element_position: 3,
            values: &["21"],
        }),
        segments: TS270_2000B_SEGMENTS,
        children: TS270_2000B_CHILDREN,
    },
    LoopDef {
        id: "2000C",
        name: "Subscriber",
        usage: Usage::Situational,
        repeat_count: u32::MAX,
        trigger_segment: "HL",
        qualifier: Some(QualifierMatch {
            element_position: 3,
            values: &["22"],
        }),
        segments: TS270_2000C_SEGMENTS,
        children: TS270_2000C_CHILDREN,
    },
];

static TS270_TRAILER_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "SE",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS270: TransactionSetDef = TransactionSetDef {
    id: "270",
    name: "Eligibility, Coverage, or Benefit Inquiry",
    functional_group_id: "HS",
    version: "005010",
    implementation_ref: Some("005010X279A1"),
    header_segments: TS270_HEADER_SEGMENTS,
    header_loops: &[],
    detail_loops: TS270_DETAIL_LOOPS,
    summary_loops: &[],
    trailer_segments: TS270_TRAILER_SEGMENTS,
};

// ---------------------------------------------------------------------------
// 837P — Professional Claim — 005010X222A1 (abbreviated but structural)
// ---------------------------------------------------------------------------

static TS837P_HEADER_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "ST",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "BHT",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
];

// Loop 1000A — Submitter Name
static TS837P_1000A_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "PER",
        usage: Usage::Required,
        max_use: 2,
        position: 20,
    },
];

// Loop 1000B — Receiver Name
static TS837P_1000B_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "NM1",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS837P_HEADER_LOOPS: &[LoopDef] = &[
    LoopDef {
        id: "1000A",
        name: "Submitter Name",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["41"],
        }),
        segments: TS837P_1000A_SEGMENTS,
        children: &[],
    },
    LoopDef {
        id: "1000B",
        name: "Receiver Name",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["40"],
        }),
        segments: TS837P_1000B_SEGMENTS,
        children: &[],
    },
];

// Loop 2010AA — Billing Provider Name
static TS837P_2010AA_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Required,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 2,
        position: 40,
    },
];

// Loop 2010AB — Pay-To Address Name
static TS837P_2010AB_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Situational,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Required,
        max_use: 1,
        position: 30,
    },
];

// Loop 2000A — Billing Provider
static TS837P_2000A_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "HL",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "PRV",
        usage: Usage::Situational,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "CUR",
        usage: Usage::Situational,
        max_use: 1,
        position: 30,
    },
];

static TS837P_2000A_CHILDREN: &[LoopDef] = &[
    LoopDef {
        id: "2010AA",
        name: "Billing Provider Name",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["85"],
        }),
        segments: TS837P_2010AA_SEGMENTS,
        children: &[],
    },
    LoopDef {
        id: "2010AB",
        name: "Pay-To Address Name",
        usage: Usage::Situational,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["87"],
        }),
        segments: TS837P_2010AB_SEGMENTS,
        children: &[],
    },
];

// Loop 2010BA — Subscriber Name
static TS837P_2010BA_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Situational,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Situational,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "DMG",
        usage: Usage::Situational,
        max_use: 1,
        position: 40,
    },
];

// Loop 2010BB — Payer Name
static TS837P_2010BB_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Situational,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Situational,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 5,
        position: 40,
    },
];

// Loop 2000B — Subscriber
static TS837P_2000B_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "HL",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "SBR",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
];

static TS837P_2000B_CHILDREN: &[LoopDef] = &[
    LoopDef {
        id: "2010BA",
        name: "Subscriber Name",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["IL"],
        }),
        segments: TS837P_2010BA_SEGMENTS,
        children: &[],
    },
    LoopDef {
        id: "2010BB",
        name: "Payer Name",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "NM1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["PR"],
        }),
        segments: TS837P_2010BB_SEGMENTS,
        children: &[],
    },
];

// Loop 2010CA — Patient Name
static TS837P_2010CA_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Required,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "DMG",
        usage: Usage::Required,
        max_use: 1,
        position: 40,
    },
];

// Loop 2000C — Patient
static TS837P_2000C_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "HL",
        usage: Usage::Situational,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "PAT",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
];

static TS837P_2000C_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "2010CA",
    name: "Patient Name",
    usage: Usage::Required,
    repeat_count: 1,
    trigger_segment: "NM1",
    qualifier: Some(QualifierMatch {
        element_position: 1,
        values: &["QC"],
    }),
    segments: TS837P_2010CA_SEGMENTS,
    children: &[],
}];

// Loop 2400 — Service Line
static TS837P_2400_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "LX",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "SV1",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "DTP",
        usage: Usage::Required,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 5,
        position: 40,
    },
];

// Loop 2300 — Claim Information
static TS837P_2300_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "CLM",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "DTP",
        usage: Usage::Situational,
        max_use: 4,
        position: 20,
    },
    SegmentRef {
        segment_id: "AMT",
        usage: Usage::Situational,
        max_use: 2,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 14,
        position: 40,
    },
    SegmentRef {
        segment_id: "HI",
        usage: Usage::Required,
        max_use: 1,
        position: 80,
    },
];

static TS837P_2300_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "2400",
    name: "Service Line",
    usage: Usage::Required,
    repeat_count: 50,
    trigger_segment: "LX",
    qualifier: None,
    segments: TS837P_2400_SEGMENTS,
    children: &[],
}];

static TS837P_DETAIL_LOOPS: &[LoopDef] = &[
    LoopDef {
        id: "2000A",
        name: "Billing Provider",
        usage: Usage::Required,
        repeat_count: u32::MAX,
        trigger_segment: "HL",
        qualifier: Some(QualifierMatch {
            element_position: 3,
            values: &["20"],
        }),
        segments: TS837P_2000A_SEGMENTS,
        children: TS837P_2000A_CHILDREN,
    },
    LoopDef {
        id: "2000B",
        name: "Subscriber",
        usage: Usage::Required,
        repeat_count: u32::MAX,
        trigger_segment: "HL",
        qualifier: Some(QualifierMatch {
            element_position: 3,
            values: &["22"],
        }),
        segments: TS837P_2000B_SEGMENTS,
        children: TS837P_2000B_CHILDREN,
    },
    LoopDef {
        id: "2000C",
        name: "Patient",
        usage: Usage::Situational,
        repeat_count: u32::MAX,
        trigger_segment: "HL",
        qualifier: Some(QualifierMatch {
            element_position: 3,
            values: &["23"],
        }),
        segments: TS837P_2000C_SEGMENTS,
        children: TS837P_2000C_CHILDREN,
    },
    LoopDef {
        id: "2300",
        name: "Claim Information",
        usage: Usage::Required,
        repeat_count: 100,
        trigger_segment: "CLM",
        qualifier: None,
        segments: TS837P_2300_SEGMENTS,
        children: TS837P_2300_CHILDREN,
    },
];

static TS837P_TRAILER_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "SE",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS837P: TransactionSetDef = TransactionSetDef {
    id: "837",
    name: "Health Care Claim: Professional",
    functional_group_id: "HC",
    version: "005010",
    implementation_ref: Some("005010X222A1"),
    header_segments: TS837P_HEADER_SEGMENTS,
    header_loops: TS837P_HEADER_LOOPS,
    detail_loops: TS837P_DETAIL_LOOPS,
    summary_loops: &[],
    trailer_segments: TS837P_TRAILER_SEGMENTS,
};

// ---------------------------------------------------------------------------
// 835 — Health Care Claim Payment/Remittance Advice — 005010X221A1
// ---------------------------------------------------------------------------

static TS835_HEADER_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "ST",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "BPR",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "TRN",
        usage: Usage::Required,
        max_use: 1,
        position: 30,
    },
];

// Loop 1000A — Payer Identification
static TS835_1000A_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "N1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Required,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 4,
        position: 40,
    },
    SegmentRef {
        segment_id: "PER",
        usage: Usage::Situational,
        max_use: 3,
        position: 50,
    },
];

// Loop 1000B — Payee Identification
static TS835_1000B_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "N1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Situational,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Situational,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 4,
        position: 40,
    },
];

static TS835_HEADER_LOOPS: &[LoopDef] = &[
    LoopDef {
        id: "1000A",
        name: "Payer Identification",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "N1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["PR"],
        }),
        segments: TS835_1000A_SEGMENTS,
        children: &[],
    },
    LoopDef {
        id: "1000B",
        name: "Payee Identification",
        usage: Usage::Required,
        repeat_count: 1,
        trigger_segment: "N1",
        qualifier: Some(QualifierMatch {
            element_position: 1,
            values: &["PE"],
        }),
        segments: TS835_1000B_SEGMENTS,
        children: &[],
    },
];

// Loop 2110 — Service Payment Information
static TS835_2110_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "SVC",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "DTM",
        usage: Usage::Situational,
        max_use: 3,
        position: 20,
    },
    SegmentRef {
        segment_id: "CAS",
        usage: Usage::Situational,
        max_use: 99,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 24,
        position: 40,
    },
    SegmentRef {
        segment_id: "AMT",
        usage: Usage::Situational,
        max_use: 12,
        position: 50,
    },
    SegmentRef {
        segment_id: "QTY",
        usage: Usage::Situational,
        max_use: 6,
        position: 60,
    },
    SegmentRef {
        segment_id: "LQ",
        usage: Usage::Situational,
        max_use: 99,
        position: 70,
    },
];

// Loop 2100 — Claim Payment Information
static TS835_2100_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "CLP",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "CAS",
        usage: Usage::Situational,
        max_use: 99,
        position: 20,
    },
    SegmentRef {
        segment_id: "NM1",
        usage: Usage::Situational,
        max_use: 9,
        position: 30,
    },
    SegmentRef {
        segment_id: "MIA",
        usage: Usage::Situational,
        max_use: 1,
        position: 40,
    },
    SegmentRef {
        segment_id: "MOA",
        usage: Usage::Situational,
        max_use: 1,
        position: 50,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 14,
        position: 60,
    },
    SegmentRef {
        segment_id: "DTM",
        usage: Usage::Situational,
        max_use: 4,
        position: 70,
    },
    SegmentRef {
        segment_id: "PER",
        usage: Usage::Situational,
        max_use: 3,
        position: 80,
    },
    SegmentRef {
        segment_id: "AMT",
        usage: Usage::Situational,
        max_use: 14,
        position: 90,
    },
    SegmentRef {
        segment_id: "QTY",
        usage: Usage::Situational,
        max_use: 14,
        position: 100,
    },
];

static TS835_2100_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "2110",
    name: "Service Payment Information",
    usage: Usage::Situational,
    repeat_count: 999,
    trigger_segment: "SVC",
    qualifier: None,
    segments: TS835_2110_SEGMENTS,
    children: &[],
}];

// Loop 2000 — Header Number
static TS835_2000_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "LX",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "TS3",
        usage: Usage::Situational,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "TS2",
        usage: Usage::Situational,
        max_use: 1,
        position: 30,
    },
];

static TS835_2000_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "2100",
    name: "Claim Payment Information",
    usage: Usage::Required,
    repeat_count: u32::MAX,
    trigger_segment: "CLP",
    qualifier: None,
    segments: TS835_2100_SEGMENTS,
    children: TS835_2100_CHILDREN,
}];

static TS835_DETAIL_LOOPS: &[LoopDef] = &[LoopDef {
    id: "2000",
    name: "Header Number",
    usage: Usage::Required,
    repeat_count: u32::MAX,
    trigger_segment: "LX",
    qualifier: None,
    segments: TS835_2000_SEGMENTS,
    children: TS835_2000_CHILDREN,
}];

static TS835_TRAILER_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "SE",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS835: TransactionSetDef = TransactionSetDef {
    id: "835",
    name: "Health Care Claim Payment/Remittance Advice",
    functional_group_id: "HP",
    version: "005010",
    implementation_ref: Some("005010X221A1"),
    header_segments: TS835_HEADER_SEGMENTS,
    header_loops: TS835_HEADER_LOOPS,
    detail_loops: TS835_DETAIL_LOOPS,
    summary_loops: &[],
    trailer_segments: TS835_TRAILER_SEGMENTS,
};

// ---------------------------------------------------------------------------
// 850 — Purchase Order — 004010
// ---------------------------------------------------------------------------

static TS850_HEADER_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "ST",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "BEG",
        usage: Usage::Required,
        max_use: 1,
        position: 20,
    },
    SegmentRef {
        segment_id: "CUR",
        usage: Usage::Situational,
        max_use: 1,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 12,
        position: 40,
    },
    SegmentRef {
        segment_id: "PER",
        usage: Usage::Situational,
        max_use: 3,
        position: 50,
    },
    SegmentRef {
        segment_id: "DTM",
        usage: Usage::Situational,
        max_use: 10,
        position: 60,
    },
];

// Loop N1 (header-level) — Party Identification
static TS850_N1_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "N1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "N2",
        usage: Usage::Situational,
        max_use: 2,
        position: 20,
    },
    SegmentRef {
        segment_id: "N3",
        usage: Usage::Situational,
        max_use: 2,
        position: 30,
    },
    SegmentRef {
        segment_id: "N4",
        usage: Usage::Situational,
        max_use: 1,
        position: 40,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 12,
        position: 50,
    },
    SegmentRef {
        segment_id: "PER",
        usage: Usage::Situational,
        max_use: 3,
        position: 60,
    },
];

// Line-level N1 loop inside PO1
static TS850_PO1_N1_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "N1",
    usage: Usage::Situational,
    max_use: 1,
    position: 10,
}];

// Loop PO1 — Line Item
static TS850_PO1_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "PO1",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "PID",
        usage: Usage::Situational,
        max_use: 1000,
        position: 20,
    },
    SegmentRef {
        segment_id: "MEA",
        usage: Usage::Situational,
        max_use: 40,
        position: 30,
    },
    SegmentRef {
        segment_id: "REF",
        usage: Usage::Situational,
        max_use: 12,
        position: 40,
    },
    SegmentRef {
        segment_id: "DTM",
        usage: Usage::Situational,
        max_use: 10,
        position: 50,
    },
    SegmentRef {
        segment_id: "SAC",
        usage: Usage::Situational,
        max_use: 25,
        position: 60,
    },
];

static TS850_PO1_CHILDREN: &[LoopDef] = &[LoopDef {
    id: "N1",
    name: "Line-Level Party Identification",
    usage: Usage::Situational,
    repeat_count: 200,
    trigger_segment: "N1",
    qualifier: None,
    segments: TS850_PO1_N1_SEGMENTS,
    children: &[],
}];

// Loop CTT — Transaction Totals
static TS850_CTT_SEGMENTS: &[SegmentRef] = &[
    SegmentRef {
        segment_id: "CTT",
        usage: Usage::Required,
        max_use: 1,
        position: 10,
    },
    SegmentRef {
        segment_id: "AMT",
        usage: Usage::Situational,
        max_use: 1,
        position: 20,
    },
];

static TS850_HEADER_LOOPS: &[LoopDef] = &[LoopDef {
    id: "N1",
    name: "Party Identification",
    usage: Usage::Situational,
    repeat_count: 200,
    trigger_segment: "N1",
    qualifier: None,
    segments: TS850_N1_SEGMENTS,
    children: &[],
}];

static TS850_DETAIL_LOOPS: &[LoopDef] = &[LoopDef {
    id: "PO1",
    name: "Line Item",
    usage: Usage::Required,
    repeat_count: 100_000,
    trigger_segment: "PO1",
    qualifier: None,
    segments: TS850_PO1_SEGMENTS,
    children: TS850_PO1_CHILDREN,
}];

static TS850_SUMMARY_LOOPS: &[LoopDef] = &[LoopDef {
    id: "CTT",
    name: "Transaction Totals",
    usage: Usage::Situational,
    repeat_count: 1,
    trigger_segment: "CTT",
    qualifier: None,
    segments: TS850_CTT_SEGMENTS,
    children: &[],
}];

static TS850_TRAILER_SEGMENTS: &[SegmentRef] = &[SegmentRef {
    segment_id: "SE",
    usage: Usage::Required,
    max_use: 1,
    position: 10,
}];

static TS850: TransactionSetDef = TransactionSetDef {
    id: "850",
    name: "Purchase Order",
    functional_group_id: "PO",
    version: "004010",
    implementation_ref: None,
    header_segments: TS850_HEADER_SEGMENTS,
    header_loops: TS850_HEADER_LOOPS,
    detail_loops: TS850_DETAIL_LOOPS,
    summary_loops: TS850_SUMMARY_LOOPS,
    trailer_segments: TS850_TRAILER_SEGMENTS,
};

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Registry of built-in transaction set schemas.
///
/// The registry holds static references to all known transaction set definitions
/// and provides lookup by ID + version or implementation guide reference.
pub struct Registry {
    sets: Vec<&'static TransactionSetDef>,
}

impl Registry {
    /// Create a new registry populated with all built-in transaction set
    /// definitions.
    pub fn new() -> Self {
        Self {
            sets: vec![&TS997, &TS270, &TS837P, &TS835, &TS850],
        }
    }

    /// Look up a transaction set by its ID and version.
    ///
    /// # Examples
    /// ```
    /// use trame_schema::Registry;
    /// let reg = Registry::new();
    /// let ts = reg.lookup("997", "005010").unwrap();
    /// assert_eq!(ts.id, "997");
    /// ```
    pub fn lookup(&self, ts_id: &str, version: &str) -> Option<&'static TransactionSetDef> {
        self.sets
            .iter()
            .find(|s| s.id == ts_id && s.version == version)
            .copied()
    }

    /// Look up a transaction set by its implementation guide reference.
    ///
    /// # Examples
    /// ```
    /// use trame_schema::Registry;
    /// let reg = Registry::new();
    /// let ts = reg.lookup_by_impl_ref("005010X279A1").unwrap();
    /// assert_eq!(ts.id, "270");
    /// ```
    pub fn lookup_by_impl_ref(&self, impl_ref: &str) -> Option<&'static TransactionSetDef> {
        self.sets
            .iter()
            .find(|s| s.implementation_ref == Some(impl_ref))
            .copied()
    }

    /// Return all registered transaction set definitions.
    pub fn all(&self) -> &[&'static TransactionSetDef] {
        &self.sets
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lookup_997() {
        let reg = Registry::new();
        let ts = reg
            .lookup("997", "005010")
            .expect("997 should be registered");
        assert_eq!(ts.id, "997");
        assert_eq!(ts.name, "Functional Acknowledgment");
        assert_eq!(ts.functional_group_id, "FA");
    }

    #[test]
    fn registry_lookup_270() {
        let reg = Registry::new();
        let ts = reg
            .lookup("270", "005010")
            .expect("270 should be registered");
        assert_eq!(ts.id, "270");
        assert_eq!(ts.implementation_ref, Some("005010X279A1"));
    }

    #[test]
    fn registry_lookup_837p() {
        let reg = Registry::new();
        let ts = reg
            .lookup("837", "005010")
            .expect("837 should be registered");
        assert_eq!(ts.id, "837");
        assert_eq!(ts.implementation_ref, Some("005010X222A1"));
    }

    #[test]
    fn registry_lookup_by_impl_ref() {
        let reg = Registry::new();
        let ts = reg
            .lookup_by_impl_ref("005010X279A1")
            .expect("270 by impl ref");
        assert_eq!(ts.id, "270");

        let ts2 = reg
            .lookup_by_impl_ref("005010X222A1")
            .expect("837 by impl ref");
        assert_eq!(ts2.id, "837");
    }

    #[test]
    fn registry_lookup_missing() {
        let reg = Registry::new();
        assert!(reg.lookup("999", "005010").is_none());
        assert!(reg.lookup("997", "004010").is_none());
        assert!(reg.lookup_by_impl_ref("NOPE").is_none());
    }

    #[test]
    fn registry_all() {
        let reg = Registry::new();
        assert_eq!(reg.all().len(), 5);
    }

    #[test]
    fn ts997_structure() {
        let ts = &TS997;
        // Header: ST, AK1
        assert_eq!(ts.header_segments.len(), 2);
        assert_eq!(ts.header_segments[0].segment_id, "ST");
        assert_eq!(ts.header_segments[1].segment_id, "AK1");

        // Detail: one loop AK2 with child loop AK3
        assert_eq!(ts.detail_loops.len(), 1);
        let ak2_loop = &ts.detail_loops[0];
        assert_eq!(ak2_loop.id, "AK2");
        assert_eq!(ak2_loop.trigger_segment, "AK2");
        assert_eq!(ak2_loop.segments.len(), 2);
        assert_eq!(ak2_loop.segments[0].segment_id, "AK2");
        assert_eq!(ak2_loop.segments[1].segment_id, "AK5");

        // Child loop: AK3 with AK4
        assert_eq!(ak2_loop.children.len(), 1);
        let ak3_loop = &ak2_loop.children[0];
        assert_eq!(ak3_loop.id, "AK3");
        assert_eq!(ak3_loop.trigger_segment, "AK3");
        assert_eq!(ak3_loop.segments.len(), 2);
        assert_eq!(ak3_loop.segments[0].segment_id, "AK3");
        assert_eq!(ak3_loop.segments[1].segment_id, "AK4");

        // Trailer: AK9, SE
        assert_eq!(ts.trailer_segments.len(), 2);
        assert_eq!(ts.trailer_segments[0].segment_id, "AK9");
        assert_eq!(ts.trailer_segments[1].segment_id, "SE");
    }

    #[test]
    fn ts270_hl_qualifiers() {
        let ts = &TS270;
        assert_eq!(ts.detail_loops.len(), 3);

        let loop_a = &ts.detail_loops[0];
        assert_eq!(loop_a.id, "2000A");
        let qa = loop_a
            .qualifier
            .as_ref()
            .expect("2000A should have qualifier");
        assert_eq!(qa.element_position, 3);
        assert_eq!(qa.values, &["20"]);

        let loop_b = &ts.detail_loops[1];
        assert_eq!(loop_b.id, "2000B");
        let qb = loop_b
            .qualifier
            .as_ref()
            .expect("2000B should have qualifier");
        assert_eq!(qb.values, &["21"]);

        let loop_c = &ts.detail_loops[2];
        assert_eq!(loop_c.id, "2000C");
        let qc = loop_c
            .qualifier
            .as_ref()
            .expect("2000C should have qualifier");
        assert_eq!(qc.values, &["22"]);
    }

    #[test]
    fn ts837p_major_loops() {
        let ts = &TS837P;

        // Header loops: 1000A, 1000B
        assert_eq!(ts.header_loops.len(), 2);
        assert_eq!(ts.header_loops[0].id, "1000A");
        assert_eq!(ts.header_loops[1].id, "1000B");

        // Detail loops: 2000A, 2000B, 2000C, 2300
        assert_eq!(ts.detail_loops.len(), 4);
        assert_eq!(ts.detail_loops[0].id, "2000A");
        assert_eq!(ts.detail_loops[1].id, "2000B");
        assert_eq!(ts.detail_loops[2].id, "2000C");
        assert_eq!(ts.detail_loops[3].id, "2300");

        // 2000A children: 2010AA, 2010AB
        assert_eq!(ts.detail_loops[0].children.len(), 2);
        assert_eq!(ts.detail_loops[0].children[0].id, "2010AA");
        assert_eq!(ts.detail_loops[0].children[1].id, "2010AB");

        // 2300 children: 2400
        let claim_loop = &ts.detail_loops[3];
        assert_eq!(claim_loop.children.len(), 1);
        assert_eq!(claim_loop.children[0].id, "2400");
    }

    #[test]
    fn registry_lookup_835_by_impl_ref() {
        let reg = Registry::new();
        let ts = reg
            .lookup_by_impl_ref("005010X221A1")
            .expect("835 by impl ref");
        assert_eq!(ts.id, "835");
        assert_eq!(ts.name, "Health Care Claim Payment/Remittance Advice");
        assert_eq!(ts.functional_group_id, "HP");
    }

    #[test]
    fn registry_lookup_850_by_id_version() {
        let reg = Registry::new();
        let ts = reg
            .lookup("850", "004010")
            .expect("850 should be registered");
        assert_eq!(ts.id, "850");
        assert_eq!(ts.name, "Purchase Order");
        assert_eq!(ts.functional_group_id, "PO");
        assert_eq!(ts.implementation_ref, None);
    }

    #[test]
    fn ts835_loops() {
        let ts = &TS835;

        // Header segments: ST, BPR, TRN
        assert_eq!(ts.header_segments.len(), 3);
        assert_eq!(ts.header_segments[0].segment_id, "ST");
        assert_eq!(ts.header_segments[1].segment_id, "BPR");
        assert_eq!(ts.header_segments[2].segment_id, "TRN");

        // Header loops: 1000A (Payer), 1000B (Payee)
        assert_eq!(ts.header_loops.len(), 2);
        assert_eq!(ts.header_loops[0].id, "1000A");
        assert_eq!(ts.header_loops[0].name, "Payer Identification");
        assert_eq!(ts.header_loops[1].id, "1000B");
        assert_eq!(ts.header_loops[1].name, "Payee Identification");

        // Detail loops: 2000 (Header Number)
        assert_eq!(ts.detail_loops.len(), 1);
        let loop_2000 = &ts.detail_loops[0];
        assert_eq!(loop_2000.id, "2000");
        assert_eq!(loop_2000.trigger_segment, "LX");

        // 2000 child: 2100 (Claim Payment)
        assert_eq!(loop_2000.children.len(), 1);
        let loop_2100 = &loop_2000.children[0];
        assert_eq!(loop_2100.id, "2100");
        assert_eq!(loop_2100.trigger_segment, "CLP");

        // 2100 child: 2110 (Service Payment)
        assert_eq!(loop_2100.children.len(), 1);
        let loop_2110 = &loop_2100.children[0];
        assert_eq!(loop_2110.id, "2110");
        assert_eq!(loop_2110.trigger_segment, "SVC");

        // Trailer: SE
        assert_eq!(ts.trailer_segments.len(), 1);
        assert_eq!(ts.trailer_segments[0].segment_id, "SE");
    }

    #[test]
    fn ts850_loops() {
        let ts = &TS850;

        // Header segments: ST, BEG, CUR, REF, PER, DTM
        assert_eq!(ts.header_segments.len(), 6);
        assert_eq!(ts.header_segments[0].segment_id, "ST");
        assert_eq!(ts.header_segments[1].segment_id, "BEG");

        // Header loops: N1 (Party Identification)
        assert_eq!(ts.header_loops.len(), 1);
        assert_eq!(ts.header_loops[0].id, "N1");
        assert_eq!(ts.header_loops[0].name, "Party Identification");

        // Detail loops: PO1 (Line Item)
        assert_eq!(ts.detail_loops.len(), 1);
        let po1_loop = &ts.detail_loops[0];
        assert_eq!(po1_loop.id, "PO1");
        assert_eq!(po1_loop.trigger_segment, "PO1");

        // PO1 child: N1 (Line-Level)
        assert_eq!(po1_loop.children.len(), 1);
        assert_eq!(po1_loop.children[0].id, "N1");

        // Summary loops: CTT (Transaction Totals)
        assert_eq!(ts.summary_loops.len(), 1);
        assert_eq!(ts.summary_loops[0].id, "CTT");
        assert_eq!(ts.summary_loops[0].trigger_segment, "CTT");

        // Trailer: SE
        assert_eq!(ts.trailer_segments.len(), 1);
        assert_eq!(ts.trailer_segments[0].segment_id, "SE");
    }
}
