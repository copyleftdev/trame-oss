# trame-schema

X12 transaction set grammars and schema-driven document walker.

## Overview

`trame-schema` provides static, zero-heap-allocation schema definitions for X12 transaction sets and a `SchemaWalker` state machine that maps a stream of segments to their position in the grammar. Loop disambiguation uses qualifier-based matching (e.g., HL-03 values to distinguish 2000A/2000B/2000C hierarchies).

## Built-in Schemas

| Transaction Set | Name | Implementation Guide |
|----------------|------|---------------------|
| 837P | Professional Claim | 005010X222A1 |
| 835 | Remittance Advice | 005010X221A1 |
| 270 | Eligibility Inquiry | 005010X279A1 |
| 997 | Functional Acknowledgment | 005010 |
| 850 | Purchase Order | 004010 |

## Quick Start

```rust
use trame_schema::{Registry, SchemaWalker, WalkEvent};

let reg = Registry::new();
let schema = reg.lookup("997", "005010").unwrap();
let mut walker = SchemaWalker::new(schema);

let event = walker.feed(b"ST", None);
// => WalkEvent::SegmentMatch { loop_id: None, .. }

let event = walker.feed(b"AK1", None);
// => WalkEvent::SegmentMatch { loop_id: None, .. }

let event = walker.feed(b"AK2", None);
// => WalkEvent::LoopStart { loop_id: "AK2", iteration: 1 }

let event = walker.feed(b"AK5", None);
// => WalkEvent::SegmentMatch { loop_id: Some("AK2"), .. }
```

For HL-based hierarchies, pass the qualifier to disambiguate loops:

```rust
let schema = reg.lookup("270", "005010").unwrap();
let mut walker = SchemaWalker::new(schema);

walker.feed(b"ST", None);
walker.feed(b"BHT", None);

let event = walker.feed(b"HL", Some(b"20"));
// => WalkEvent::LoopStart { loop_id: "2000A", .. }

let event = walker.feed(b"HL", Some(b"22"));
// => WalkEvent::LoopStart { loop_id: "2000C", .. }
```

## Core Types

| Type | Description |
|------|-------------|
| `TransactionSetDef` | Complete transaction set grammar (header, detail, summary, trailer) |
| `LoopDef` | Loop with trigger segment, qualifier match, child loops, and segment list |
| `SegmentRef` | Reference to a segment within a loop, with usage and max repeat |
| `QualifierMatch` | Element position + allowed values for loop disambiguation |
| `ElementDef` | Element within a segment: data type, length, usage, code values |
| `DataType` | AN, Nn, R, ID, DT, TM, B |
| `Usage` | Required, Situational, NotUsed |

All schema types use `&'static` references so definitions can live as `const`/`static` data with zero heap allocation.

## Part of trame

`trame-schema` is part of the [trame](https://github.com/copyleftdev/trame) workspace. It depends on `trame-wire` for the underlying segment types.

## License

Apache-2.0
