# Fuzzing trame

## Prerequisites

```bash
cargo install cargo-afl
```

## Running AFL++

Build the fuzz targets with AFL instrumentation:

```bash
cd fuzz
cargo afl build --release
```

Run a fuzz target:

```bash
# Parser fuzzing (recommended first target)
cargo afl fuzz -i corpus/x12 -o out/parse_segments target/release/fuzz_parse_segments

# Interchange parsing
cargo afl fuzz -i corpus/x12 -o out/parse_interchange target/release/fuzz_parse_interchange

# Delimiter detection
cargo afl fuzz -i corpus/delimiters -o out/delimiters target/release/fuzz_delimiters

# Round-trip verification
cargo afl fuzz -i corpus/x12 -o out/roundtrip target/release/fuzz_roundtrip

# Schema walker
cargo afl fuzz -i corpus/x12 -o out/schema_walk target/release/fuzz_schema_walk

# Element access
cargo afl fuzz -i corpus/x12 -o out/element_access target/release/fuzz_element_access
```

## Parallel fuzzing

Run multiple AFL instances for better coverage:

```bash
# Primary instance
cargo afl fuzz -i corpus/x12 -o out/parse_segments -M fuzzer01 target/release/fuzz_parse_segments

# Secondary instances (in other terminals)
cargo afl fuzz -i corpus/x12 -o out/parse_segments -S fuzzer02 target/release/fuzz_parse_segments
cargo afl fuzz -i corpus/x12 -o out/parse_segments -S fuzzer03 target/release/fuzz_parse_segments
```

## Reproducing crashes

```bash
cargo afl run target/release/fuzz_parse_segments < out/parse_segments/crashes/id:000000,*
```

## Targets

| Target | What it tests |
|--------|--------------|
| `fuzz_parse_segments` | Core segment parser -- all element access paths |
| `fuzz_parse_interchange` | Full ISA/GS/ST hierarchy parsing |
| `fuzz_delimiters` | ISA header delimiter detection |
| `fuzz_roundtrip` | Parse -> write -> re-parse consistency |
| `fuzz_schema_walk` | Schema walker state machine |
| `fuzz_element_access` | Exhaustive element/sub-element access at all indices |
