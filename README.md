# trame

A complete X12 EDI toolkit in Rust. Parse, validate, generate, transform, store, and transport — from raw bytes to database in one pipeline.

```
trame fake 837p --seed 42 | trame fmt
```

```
ISA*00*          *00*          *ZZ*METRO HEALTH NE*ZZ*PACIFIC CARE SY*260519*0007*U*00501*000000001*0*P*:~
  GS*HC*METRO HEALTH NETWORK*PACIFIC CARE SYSTEMS*20260519*0007*1*X*005010X222A1~
    ST*837*0001~
      BHT*0019*00*1703935056*20230414*0007*CH~
      NM1*41*2*METRO HEALTH NETWORK*****46*0281189718~
      PER*IC*METRO HEALTH NETWORK*TE*5551234567~
      NM1*40*2*PACIFIC CARE SYSTEMS*****46*345825536~
      HL*1**20*1~
      NM1*85*2*ACME HEALTH SERVICES*****XX*0281189718~
      N3*123 MAIN ST~
      N4*ANYTOWN*CA*90210~
      REF*EI*811414032~
      HL*2*1*22*0~
      SBR*P*18*******345825536~
      NM1*IL*1*HERNANDEZ*CHARLES****MI*ID93674426~
      CLM*007731762767*3405.97***22:B:1*Y*A*Y*I~
      HI*ABK:Z00.00~
      SV1*HC:99281*2671.01*UN*1***1~
      DTP*472*D8*20230414~
    SE*27*0001~
  GE*1*1~
IEA*1*000000001~
```

## Why this exists

Every X12 tool you can find today is either:

- **A SaaS product** that holds your data hostage behind an API
- **A single-purpose library** in one language that handles parsing but not validation, or validation but not generation
- **Abandoned** — last commit 2019, broken on modern runtimes
- **Proprietary** — $50K/year for an "EDI translator" that runs on a Windows VM

There is no comprehensive, open, fast, testable X12 toolkit. Trame is that toolkit.

## What it does

```bash
# Pretty-print any X12 file
trame fmt claim.edi

# Inspect interchange structure
trame info claim.edi

# Show available commands
trame help

# Show version
trame version
```

Additional commands (validate, generate, transform, store, transport) are available in [trame-pro](#trame-pro).

## Architecture

Four crates. Each useful standalone. All composable.

```
trame-cli           CLI: fmt, info, help, version
trame-schema        Grammar definitions for 837P, 835, 270, 997, 850
trame-wire          Zero-copy streaming parser, delimiter autodetect, writer
trame-dst           VOPR deterministic simulation testing framework
```

### trame-wire: Zero-copy parser

Parses X12 directly from `&[u8]`. No heap allocation during parsing. Delimiter autodetection from the ISA header.

```rust
use trame_wire::{Parser, parse_interchanges};

// Low-level: stream segments
let parser = Parser::new(raw_bytes)?;
for segment in parser {
    let seg = segment?;
    println!("{}: {} elements", seg.id_str().unwrap_or("?"), seg.element_count());
}

// High-level: full interchange tree
let interchanges = parse_interchanges(raw_bytes)?;
for ix in &interchanges {
    for group in &ix.groups {
        for txn in &group.transaction_sets {
            println!("{} — {} segments", txn.st.transaction_set_id, txn.segments.len());
        }
    }
}
```

### trame-schema: Grammar engine

Static schema definitions — zero allocation at lookup time. SchemaWalker state machine with qualifier-based loop disambiguation.

```rust
use trame_schema::{Registry, SchemaWalker};

let registry = Registry::new();
let schema = registry.lookup("837", "005010").unwrap();
let mut walker = SchemaWalker::new(schema);

// Feed segments, get structured events
for segment in parsed_segments {
    match walker.feed(segment.id(), qualifier) {
        WalkEvent::LoopStart { loop_id, .. } => println!("entering {loop_id}"),
        WalkEvent::SegmentMatch { .. } => { /* valid */ },
        WalkEvent::SegmentUnexpected { .. } => { /* error */ },
        WalkEvent::LoopEnd { .. } => { /* closing */ },
    }
}
```

Built-in schemas: **837P** (005010X222A1), **835** (005010X221A1), **270** (005010X279A1), **997**, **850** (004010).

### trame-dst: Deterministic simulation testing

Every parser bug is reproducible with a single `u64` seed.

```rust
use trame_dst::prng::SplitMix64;
use trame_dst::fault::{FaultInjector, FaultProfile};

let mut rng = SplitMix64::new(seed);

// Generate random X12, mutate it, verify the parser never panics
// 100 seeds x 50 docs = 5,000 documents, 40,000 mutations, 0 panics
```

The VOPR framework provides:
- **SplitMix64 PRNG** with fork hierarchy for independent streams
- **32 fault types** across storage, network, process, clock, and composite categories
- **9-phase simulation loop** with configurable fault probability
- **Shrinking** — minimize failing inputs to the smallest reproduction
- **CI tiers** — Commit (10K ticks) through PreRelease (100M ticks)

### trame-cli

```bash
trame fmt claim.edi       # pretty-print with indentation
trame info claim.edi      # show interchange metadata
```

## Testing

224 tests. Zero failures.

```
trame-dst           130 tests    VOPR framework, PRNG, fault injection, simulation
trame-wire           69 tests    Parser, segments, envelopes, interchange, writer, VOPR fuzz
trame-schema         21 tests    Registry, walker, loop disambiguation
```

The VOPR fuzz suite generates 5,000 random X12 documents, applies 40,000 mutations (bit flips, truncation, delimiter swaps, byte insertion/deletion), and verifies six properties hold for every input:

1. **No panic** — the parser never crashes on any input
2. **Round-trip** — parse then write produces identical bytes
3. **Determinism** — same input always produces same output
4. **Delimiter consistency** — detection is stable
5. **Segment bound** — can't produce more segments than input bytes
6. **Zero-copy correctness** — all references point within the source buffer

## Building

```bash
cargo build --workspace            # debug
cargo build --workspace --release  # optimized
cargo test --workspace             # run all 224 tests
cargo run --bin trame -- help      # run the CLI
```

Requires Rust 1.85.0+ (edition 2024).

## Dependencies

The runtime dependency tree:

```
trame-wire          0 deps    (only std)
trame-schema        0 deps    (only std + trame-wire)
trame-dst           0 deps    (only std)
trame-cli           0 deps    (only std + workspace crates)
```

Zero external dependencies.

## License

Apache-2.0

See [LICENSE](LICENSE) for full terms.

## Status

v0.1.0. API not yet stable.

## trame-pro

The commercial companion adds validation, generation, transformation, storage, and transport:

- `trame-validate` — Schema-driven validation, 997/999 acknowledgment generation
- `trame-generate` — Fluent document builders, deterministic fake data
- `trame-transform` — X12 to JSON (zero-dep, byte-identical round-trip)
- `trame-store` — SQLite persistence, lifecycle tracking, audit trail
- `trame-transport` — File-based and in-memory transport

Contact: [GitHub Issues](https://github.com/copyleftdev/trame/issues)
