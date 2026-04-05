# trame-wire

Zero-copy, streaming X12 EDI parser and writer in Rust.

## Quick Start

```rust
use trame_wire::Parser;

let input = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~GS*HP*S*R*20210901*1234*1*X*005010~ST*837*0001~SE*1*0001~GE*1*1~IEA*1*000000001~";

let parser = Parser::new(input).unwrap();
for segment in parser {
    let seg = segment.unwrap();
    let id = seg.id_str().unwrap_or("???");
    let elem1 = seg.element_str(1).unwrap_or("");
    println!("{id}: {elem1}");
}
```

## Features

- **Zero-copy** -- `Segment` is a lightweight `&[u8]` view into the source buffer. No heap allocation during parsing.
- **Delimiter autodetect** -- Reads delimiters from the ISA header automatically; also supports explicit delimiters.
- **Streaming iterator** -- `Parser` implements `Iterator<Item = Result<Segment, ParseError>>`. Process files of any size.
- **Envelope parsing** -- Typed, zero-copy structs for ISA, GS, and ST headers (`Isa`, `Gs`, `St`).
- **Full interchange reader** -- `Interchange::parse` groups segments into the ISA/GS/ST/SE/GE/IEA hierarchy.
- **Writer / serializer** -- `Writer` builds X12 output segment by segment with configurable delimiters.
- **Sub-element access** -- `seg.sub_elements(n)` splits composite elements by the component separator.
- **Zero external dependencies** -- No runtime crate dependencies at all.

## High-Level API

Parse a complete interchange with full envelope structure:

```rust
use trame_wire::parse_interchanges;

let input = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~GS*HP*S*R*20210901*1234*1*X*005010~ST*837*0001~BHT*0019*00*12345*20210901*1234*CH~SE*2*0001~GE*1*1~IEA*1*000000001~";

let interchanges = parse_interchanges(input).unwrap();
let ic = &interchanges[0];
assert_eq!(ic.isa.sender_id, b"SENDER         ");

for group in &ic.groups {
    for txn in &group.transaction_sets {
        println!("TX {} with {} body segments",
            std::str::from_utf8(txn.st.transaction_set_id).unwrap(),
            txn.segments.len());
    }
}
```

## Writer

Build X12 output programmatically:

```rust
use trame_wire::{Writer, Delimiters};

let mut w = Writer::new(Delimiters::default());
w.write_segment(b"CLM", &[b"12345", b"100", b"", b"", b"11:B:1"]);
w.write_segment(b"NM1", &[b"IL", b"1", b"DOE", b"JOHN"]);

let output = w.finish();
assert_eq!(&output, b"CLM*12345*100***11:B:1~NM1*IL*1*DOE*JOHN~");
```

## Architecture

The parser operates at three levels:

| Level | Entry point | Description |
|-------|-------------|-------------|
| Segment | `Parser::new(input)` | Yields individual `Segment` references from a byte slice |
| Envelope | `Isa::parse`, `Gs::parse`, `St::parse` | Typed structs for the three X12 envelope headers |
| Interchange | `Interchange::parse(input)` | Groups segments into the full ISA/GS/ST hierarchy |

## Part of trame

`trame-wire` is the parsing foundation of the [trame](https://github.com/copyleftdev/trame) workspace, which includes schema validation (`trame-schema`), deterministic simulation testing (`trame-dst`), and a CLI (`trame-cli`).

## License

Apache-2.0
