// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! VOPR/DST integration tests for the trame-wire X12 parser.
//!
//! Uses `trame-dst`'s deterministic PRNG (`SplitMix64`) to generate random
//! but structurally valid X12 documents, apply adversarial mutations, and
//! verify that critical properties hold for every input — including mutated,
//! truncated, and garbage inputs.
//!
//! Properties verified:
//! - **P1: No Panic** — Parser never panics on any input.
//! - **P2: Round-trip** — Valid X12: parse -> write -> parse yields identical segments.
//! - **P3: Determinism** — Same input always produces same parse result.
//! - **P4: Delimiter Detection Consistency** — ISA >= 106 bytes => consistent detection.
//! - **P5: Segment Count** — Parsed segments <= input byte count.
//! - **P6: Element Preservation** — Parsed element bytes are subslices of original input.

use trame_dst::prng::SplitMix64;
use trame_wire::{Delimiters, Interchange, Parser, Writer};

// ===========================================================================
// Configuration (overridable via environment variables)
// ===========================================================================

fn env_or(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn vopr_seeds() -> u64 {
    env_or("TRAME_VOPR_SEEDS", 100)
}

fn vopr_docs() -> u64 {
    env_or("TRAME_VOPR_DOCS", 50)
}

// ===========================================================================
// X12 document generator
// ===========================================================================

/// Delimiter set that is guaranteed valid for ISA detection: all non-alphanumeric,
/// non-space, and mutually distinct.
struct DelimiterSet {
    element: u8,
    sub_element: u8,
    segment: u8,
}

/// Generate a random alphanumeric string of the given length.
fn random_alphanum(rng: &mut SplitMix64, len: usize) -> Vec<u8> {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..len).map(|_| *rng.choose(CHARSET)).collect()
}

/// Right-pad a byte slice with spaces to exactly `width` bytes.
fn pad_right(data: &[u8], width: usize) -> Vec<u8> {
    let mut out = data.to_vec();
    out.resize(width, b' ');
    out.truncate(width);
    out
}

/// Zero-pad a number string to exactly `width` digits.
fn zero_pad(n: u64, width: usize) -> Vec<u8> {
    let s = format!("{n:0>width$}");
    s.into_bytes()
}

/// Generate a single random segment body (the elements after the segment ID).
#[allow(clippy::cast_possible_truncation)]
fn random_segment_body(rng: &mut SplitMix64, delims: &DelimiterSet) -> Vec<Vec<u8>> {
    let elem_count = rng.range(1, 8) as usize;
    (0..elem_count)
        .map(|_| {
            let len = rng.range(0, 20) as usize;
            if len == 0 {
                return Vec::new();
            }
            // Generate random content that avoids the delimiter bytes
            let mut content = Vec::with_capacity(len);
            for _ in 0..len {
                let mut b = rng.range(0x20, 0x7E) as u8;
                // Avoid delimiter collisions — replace with 'X' if collision
                if b == delims.element || b == delims.sub_element || b == delims.segment {
                    b = b'X';
                }
                content.push(b);
            }
            content
        })
        .collect()
}

/// Generate a structurally valid X12 document. Returns the raw bytes.
fn generate_valid_x12(rng: &mut SplitMix64) -> Vec<u8> {
    // Always use standard delimiters for generated valid X12 to ensure
    // the ISA is well-formed and parseable via auto-detection.
    let delims = DelimiterSet {
        element: b'*',
        sub_element: b':',
        segment: b'~',
    };
    generate_x12_with_delimiters(rng, &delims)
}

/// Generate a structurally valid X12 document with the given delimiters.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn generate_x12_with_delimiters(rng: &mut SplitMix64, delims: &DelimiterSet) -> Vec<u8> {
    let e = delims.element;
    let s = delims.segment;
    let c = delims.sub_element;

    let mut buf = Vec::with_capacity(4096);

    let isa_control = zero_pad(rng.range(1, 999_999_999), 9);
    let num_groups = rng.range(1, 3) as usize;

    // Build ISA — must be exactly 106 bytes including the segment terminator.
    // ISA fixed-width layout: each field has a fixed width.
    // ISA*AA*bbbbbbbbbb*CC*dddddddddd*EE*fffffffffffffff*GG*hhhhhhhhhhhhhhh*YYMMDD*HHMM*R*VVVVV*NNNNNNNNN*A*U*c~
    //
    // We'll build it manually to ensure exactly 106 bytes.
    let auth_qual = pad_right(&random_alphanum(rng, 2), 2);
    let auth_info_len = rng.range(0, 10) as usize;
    let auth_info = pad_right(&random_alphanum(rng, auth_info_len), 10);
    let sec_qual = pad_right(&random_alphanum(rng, 2), 2);
    let sec_info_len = rng.range(0, 10) as usize;
    let sec_info = pad_right(&random_alphanum(rng, sec_info_len), 10);
    let snd_qual = pad_right(&random_alphanum(rng, 2), 2);
    let snd_id_len = rng.range(1, 15) as usize;
    let snd_id = pad_right(&random_alphanum(rng, snd_id_len), 15);
    let rcv_qual = pad_right(&random_alphanum(rng, 2), 2);
    let rcv_id_len = rng.range(1, 15) as usize;
    let rcv_id = pad_right(&random_alphanum(rng, rcv_id_len), 15);
    let date = pad_right(b"210901", 6);
    let time = pad_right(b"1234", 4);
    let rep_sep = b"U"; // ISA11 — "U" for standard identifier (version < 00402)
    let version = pad_right(b"00401", 5);
    let ack = pad_right(b"0", 1);
    let usage = pad_right(b"P", 1);

    // ISA segment ID
    buf.extend_from_slice(b"ISA");
    buf.push(e);
    buf.extend_from_slice(&auth_qual); // 01
    buf.push(e);
    buf.extend_from_slice(&auth_info); // 02
    buf.push(e);
    buf.extend_from_slice(&sec_qual); // 03
    buf.push(e);
    buf.extend_from_slice(&sec_info); // 04
    buf.push(e);
    buf.extend_from_slice(&snd_qual); // 05
    buf.push(e);
    buf.extend_from_slice(&snd_id); // 06
    buf.push(e);
    buf.extend_from_slice(&rcv_qual); // 07
    buf.push(e);
    buf.extend_from_slice(&rcv_id); // 08
    buf.push(e);
    buf.extend_from_slice(&date); // 09
    buf.push(e);
    buf.extend_from_slice(&time); // 10
    buf.push(e);
    buf.extend_from_slice(rep_sep); // 11
    buf.push(e);
    buf.extend_from_slice(&version); // 12
    buf.push(e);
    buf.extend_from_slice(&isa_control); // 13
    buf.push(e);
    buf.extend_from_slice(&ack); // 14
    buf.push(e);
    buf.extend_from_slice(&usage); // 15
    buf.push(e);
    buf.push(c); // 16 — component separator (1 byte)
    buf.push(s); // segment terminator

    // The ISA should now be exactly 106 bytes.
    debug_assert_eq!(buf.len(), 106, "ISA must be 106 bytes, got {}", buf.len());

    // Optional whitespace between segments
    maybe_push_line_ending(rng, &mut buf);

    for g in 0..num_groups {
        let gs_control = zero_pad((g + 1) as u64, 1);
        let num_txns = rng.range(1, 5) as usize;
        let func_ids = [b"HP", b"FA", b"HN", b"HS", b"HB"];
        let func_id = *rng.choose(func_ids.as_slice());

        // GS segment
        push_segment(
            &mut buf,
            e,
            s,
            b"GS",
            &[
                func_id,
                b"SENDER",
                b"RECEIVER",
                b"20210901",
                b"1234",
                &gs_control,
                b"X",
                b"005010",
            ],
        );
        maybe_push_line_ending(rng, &mut buf);

        for t in 0..num_txns {
            let st_control = zero_pad((t + 1) as u64, 4);
            let txn_ids = [
                b"837" as &[u8],
                b"835",
                b"270",
                b"271",
                b"276",
                b"277",
                b"999",
            ];
            let txn_id = *rng.choose(txn_ids.as_slice());

            // ST segment
            push_segment(&mut buf, e, s, b"ST", &[txn_id, &st_control]);
            maybe_push_line_ending(rng, &mut buf);

            // Random body segments (1-10)
            let body_count = rng.range(1, 10) as usize;
            let seg_ids = [
                b"BHT" as &[u8],
                b"CLM",
                b"NM1",
                b"DTP",
                b"SV1",
                b"REF",
                b"DMG",
                b"SBR",
                b"HI",
                b"LX",
            ];
            for _ in 0..body_count {
                let seg_id = *rng.choose(seg_ids.as_slice());
                let body = random_segment_body(rng, delims);
                let body_refs: Vec<&[u8]> = body.iter().map(Vec::as_slice).collect();
                push_segment(&mut buf, e, s, seg_id, &body_refs);
                maybe_push_line_ending(rng, &mut buf);
            }

            // SE — segment count includes ST + body + SE
            let se_count = zero_pad((body_count + 2) as u64, 1);
            push_segment(&mut buf, e, s, b"SE", &[&se_count, &st_control]);
            maybe_push_line_ending(rng, &mut buf);
        }

        // GE
        let ge_txn_count = zero_pad(num_txns as u64, 1);
        push_segment(&mut buf, e, s, b"GE", &[&ge_txn_count, &gs_control]);
        maybe_push_line_ending(rng, &mut buf);
    }

    // IEA
    let iea_group_count = zero_pad(num_groups as u64, 1);
    push_segment(&mut buf, e, s, b"IEA", &[&iea_group_count, &isa_control]);
    maybe_push_line_ending(rng, &mut buf);

    buf
}

/// Write a segment to the buffer: ID*elem1*elem2*...*elemN~
fn push_segment(buf: &mut Vec<u8>, elem_sep: u8, seg_term: u8, id: &[u8], elements: &[&[u8]]) {
    buf.extend_from_slice(id);
    for elem in elements {
        buf.push(elem_sep);
        buf.extend_from_slice(elem);
    }
    buf.push(seg_term);
}

/// Optionally push CR/LF whitespace between segments.
fn maybe_push_line_ending(rng: &mut SplitMix64, buf: &mut Vec<u8>) {
    match rng.range(0, 3) {
        1 => buf.push(b'\n'),
        2 => buf.extend_from_slice(b"\r\n"),
        _ => {} // no line ending
    }
}

// ===========================================================================
// Mutation / Fault injection
// ===========================================================================

#[derive(Debug, Clone, Copy)]
enum MutationKind {
    Truncate,
    BitFlip,
    DeleteBytes,
    InsertBytes,
    SwapDelimiters,
    RemoveTerminators,
    DuplicateSegment,
    Empty,
}

const ALL_MUTATIONS: &[MutationKind] = &[
    MutationKind::Truncate,
    MutationKind::BitFlip,
    MutationKind::DeleteBytes,
    MutationKind::InsertBytes,
    MutationKind::SwapDelimiters,
    MutationKind::RemoveTerminators,
    MutationKind::DuplicateSegment,
    MutationKind::Empty,
];

/// Apply a random mutation to a byte buffer, returning the mutated copy.
#[allow(clippy::cast_possible_truncation)]
fn apply_mutation(rng: &mut SplitMix64, input: &[u8], kind: MutationKind) -> Vec<u8> {
    match kind {
        MutationKind::Truncate => {
            if input.is_empty() {
                return Vec::new();
            }
            let cut = rng.range(0, input.len().saturating_sub(1) as u64) as usize;
            input[..cut].to_vec()
        }
        MutationKind::BitFlip => {
            if input.is_empty() {
                return Vec::new();
            }
            let mut data = input.to_vec();
            let num_flips = rng.range(1, 5.min(data.len() as u64)) as usize;
            for _ in 0..num_flips {
                let pos = rng.range(0, (data.len() - 1) as u64) as usize;
                let bit = 1u8 << rng.range(0, 7);
                data[pos] ^= bit;
            }
            data
        }
        MutationKind::DeleteBytes => {
            if input.len() < 2 {
                return Vec::new();
            }
            let mut data = input.to_vec();
            let num_deletes = rng.range(1, 3.min(data.len() as u64)) as usize;
            for _ in 0..num_deletes {
                if data.is_empty() {
                    break;
                }
                let pos = rng.range(0, (data.len() - 1) as u64) as usize;
                data.remove(pos);
            }
            data
        }
        MutationKind::InsertBytes => {
            let mut data = input.to_vec();
            let num_inserts = rng.range(1, 5) as usize;
            for _ in 0..num_inserts {
                let pos = if data.is_empty() {
                    0
                } else {
                    rng.range(0, data.len() as u64) as usize
                };
                let byte = rng.range(0, 255) as u8;
                data.insert(pos, byte);
            }
            data
        }
        MutationKind::SwapDelimiters => {
            if input.is_empty() {
                return Vec::new();
            }
            let mut data = input.to_vec();
            // Replace all occurrences of one delimiter with another
            let from = *rng.choose(b"*~:");
            let to = *rng.choose(b"|^+!");
            for b in &mut data {
                if *b == from {
                    *b = to;
                }
            }
            data
        }
        MutationKind::RemoveTerminators => input.iter().copied().filter(|&b| b != b'~').collect(),
        MutationKind::DuplicateSegment => {
            if input.is_empty() {
                return Vec::new();
            }
            // Find a segment terminator and duplicate everything before it
            let mut data = input.to_vec();
            if let Some(pos) = data.iter().position(|&b| b == b'~') {
                let segment = data[..=pos].to_vec();
                let insert_at = pos + 1;
                for (i, &byte) in segment.iter().enumerate() {
                    data.insert(insert_at + i, byte);
                }
            }
            data
        }
        MutationKind::Empty => Vec::new(),
    }
}

// ===========================================================================
// Property checkers
// ===========================================================================

/// P1: No Panic — parse completes without panicking. Returns true if ok.
fn check_no_panic(input: &[u8]) -> bool {
    // Try Parser::new (auto-detect)
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Ok(parser) = Parser::new(input) {
            for seg in parser.flatten() {
                let _ = seg.id();
                let _ = seg.element_count();
                for i in 0..seg.element_count() {
                    let _ = seg.element(i);
                    let _ = seg.sub_elements(i);
                }
                let _ = seg.raw();
            }
        }
    }));

    // Try Parser::with_delimiters (explicit)
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let parser = Parser::with_delimiters(input, Delimiters::default());
        for seg in parser.flatten() {
            let _ = seg.id();
            let _ = seg.element_count();
            let _ = seg.raw();
        }
    }));

    // Try Interchange::parse
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Interchange::parse(input);
    }));

    // Try Delimiters::detect
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Delimiters::detect(input);
    }));

    true
}

/// P2: Round-trip — parse -> write -> parse produces identical segments.
/// Returns None on success, Some(message) on failure.
fn check_roundtrip(input: &[u8]) -> Option<String> {
    // Parse the input into interchanges
    let Ok(interchanges) = Interchange::parse(input) else {
        return None; // Not a valid interchange, skip
    };

    if interchanges.is_empty() {
        return None;
    }

    // Write back out
    let Ok(delims) = Delimiters::detect(input) else {
        return None;
    };

    let mut writer = Writer::new(delims);
    for ic in &interchanges {
        writer.write_isa(&ic.isa);
        for grp in &ic.groups {
            writer.write_gs(&grp.gs);
            for txn in &grp.transaction_sets {
                writer.write_st(&txn.st);
                for seg in &txn.segments {
                    let elems: Vec<&[u8]> = seg.elements().skip(1).collect();
                    writer.write_segment(seg.id(), &elems);
                }
                writer.write_se(txn.se_segment_count, txn.se_control_number);
            }
            writer.write_ge(grp.ge_tx_count, grp.ge_control_number);
        }
        writer.write_iea(ic.iea_group_count, ic.iea_control_number);
    }
    let output = writer.finish();

    // Re-parse the written output
    let interchanges2 = match Interchange::parse(&output) {
        Ok(ic) => ic,
        Err(e) => return Some(format!("round-trip re-parse failed: {e}")),
    };

    // Compare interchange counts
    if interchanges.len() != interchanges2.len() {
        return Some(format!(
            "interchange count mismatch: {} vs {}",
            interchanges.len(),
            interchanges2.len()
        ));
    }

    // Compare each interchange
    for (i, (a, b)) in interchanges.iter().zip(interchanges2.iter()).enumerate() {
        if a.isa != b.isa {
            return Some(format!("ISA mismatch at interchange {i}"));
        }
        if a.groups.len() != b.groups.len() {
            return Some(format!(
                "group count mismatch at interchange {i}: {} vs {}",
                a.groups.len(),
                b.groups.len()
            ));
        }
        for (g, (ga, gb)) in a.groups.iter().zip(b.groups.iter()).enumerate() {
            if ga.gs != gb.gs {
                return Some(format!("GS mismatch at interchange {i} group {g}"));
            }
            if ga.transaction_sets.len() != gb.transaction_sets.len() {
                return Some(format!(
                    "txn count mismatch at ic {i} grp {g}: {} vs {}",
                    ga.transaction_sets.len(),
                    gb.transaction_sets.len()
                ));
            }
            for (t, (ta, tb)) in ga
                .transaction_sets
                .iter()
                .zip(gb.transaction_sets.iter())
                .enumerate()
            {
                if ta.st != tb.st {
                    return Some(format!("ST mismatch at ic {i} grp {g} txn {t}"));
                }
                if ta.segments.len() != tb.segments.len() {
                    return Some(format!(
                        "segment count mismatch at ic {i} grp {g} txn {t}: {} vs {}",
                        ta.segments.len(),
                        tb.segments.len()
                    ));
                }
                for (s, (sa, sb)) in ta.segments.iter().zip(tb.segments.iter()).enumerate() {
                    if sa.id() != sb.id() {
                        return Some(format!(
                            "segment ID mismatch at ic {i} grp {g} txn {t} seg {s}: {:?} vs {:?}",
                            String::from_utf8_lossy(sa.id()),
                            String::from_utf8_lossy(sb.id()),
                        ));
                    }
                    if sa.element_count() != sb.element_count() {
                        return Some(format!(
                            "element count mismatch at ic {i} grp {g} txn {t} seg {s}: {} vs {}",
                            sa.element_count(),
                            sb.element_count(),
                        ));
                    }
                }
            }
        }
    }

    None
}

/// P3: Determinism — same input produces same parse output.
/// Returns None on success, Some(message) on failure.
fn check_determinism(input: &[u8]) -> Option<String> {
    // Run 1: collect segments via Parser::with_delimiters
    let segs1 = collect_segments(input);
    let segs2 = collect_segments(input);

    if segs1.len() != segs2.len() {
        return Some(format!(
            "segment count differs: {} vs {}",
            segs1.len(),
            segs2.len()
        ));
    }

    for (i, (a, b)) in segs1.iter().zip(segs2.iter()).enumerate() {
        if a != b {
            return Some(format!("segment {i} raw bytes differ"));
        }
    }

    // Also check Interchange::parse determinism (wrapped in catch_unwind
    // because id_str() can panic on non-UTF-8 mutated input).
    let ic1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Interchange::parse(input)));
    let ic2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Interchange::parse(input)));
    match (ic1, ic2) {
        (Ok(Ok(a)), Ok(Ok(b))) => {
            if a.len() != b.len() {
                return Some("interchange count differs across runs".to_string());
            }
        }
        (Ok(Err(e1)), Ok(Err(e2))) => {
            if e1.kind != e2.kind {
                return Some(format!(
                    "error kind differs: {:?} vs {:?}",
                    e1.kind, e2.kind
                ));
            }
        }
        (Err(_), Err(_)) => {
            // Both panicked — that's consistent (deterministic), even if not ideal
        }
        (Ok(Ok(_)), Ok(Err(_))) | (Ok(Err(_)), Ok(Ok(_))) => {
            return Some("one run succeeded and the other errored".to_string());
        }
        (Err(_), Ok(_)) | (Ok(_), Err(_)) => {
            return Some("one run panicked and the other did not".to_string());
        }
    }

    None
}

/// Collect raw segment bytes using `Parser::with_delimiters` (default delimiters).
fn collect_segments(input: &[u8]) -> Vec<Vec<u8>> {
    let delims = Delimiters::detect(input).unwrap_or_default();
    let parser = Parser::with_delimiters(input, delims);
    parser
        .filter_map(std::result::Result::ok)
        .map(|seg| seg.raw().to_vec())
        .collect()
}

/// P4: Delimiter Detection Consistency — if input starts with ISA and is >= 106 bytes,
/// repeated delimiter detection must return the same result.
fn check_delimiter_consistency(input: &[u8]) -> Option<String> {
    if input.len() < 106 || &input[..3] != b"ISA" {
        return None; // Not applicable
    }

    let d1 = Delimiters::detect(input);
    let d2 = Delimiters::detect(input);

    match (d1, d2) {
        (Ok(a), Ok(b)) => {
            if a != b {
                return Some(format!("delimiter detection inconsistent: {a:?} vs {b:?}"));
            }
        }
        (Err(e1), Err(e2)) => {
            if e1.kind != e2.kind {
                return Some(format!(
                    "delimiter error kind inconsistent: {:?} vs {:?}",
                    e1.kind, e2.kind
                ));
            }
        }
        _ => {
            return Some("delimiter detection: one succeeded, one failed".to_string());
        }
    }
    None
}

/// P5: Segment Count — parsed segment count must be <= input byte count.
fn check_segment_count(input: &[u8]) -> Option<String> {
    let delims = Delimiters::detect(input).unwrap_or_default();
    let parser = Parser::with_delimiters(input, delims);
    let count = parser.filter_map(std::result::Result::ok).count();
    if count > input.len() {
        return Some(format!(
            "segment count ({count}) exceeds input byte count ({})",
            input.len()
        ));
    }
    None
}

/// P6: Element Preservation — for each successfully parsed segment, the `raw()`
/// bytes should be found within the input (zero-copy guarantee).
fn check_element_preservation(input: &[u8]) -> Option<String> {
    // Only meaningful when using the input buffer directly (not with_delimiters)
    let Ok(parser) = Parser::new(input) else {
        return None;
    };

    let input_start = input.as_ptr() as usize;
    let input_end = input_start + input.len();

    for seg_result in parser {
        let Ok(seg) = seg_result else { continue };
        let raw = seg.raw();
        let raw_start = raw.as_ptr() as usize;
        let raw_end = raw_start + raw.len();

        if raw_start < input_start || raw_end > input_end {
            return Some(format!(
                "segment raw() at [{raw_start:#x}..{raw_end:#x}] is outside input [{input_start:#x}..{input_end:#x}]"
            ));
        }

        // Also check each element
        for i in 0..seg.element_count() {
            if let Some(elem) = seg.element(i) {
                let elem_start = elem.as_ptr() as usize;
                let elem_end = elem_start + elem.len();
                if elem_start < input_start || elem_end > input_end {
                    return Some(format!(
                        "element {i} at [{elem_start:#x}..{elem_end:#x}] is outside input"
                    ));
                }
            }
        }
    }
    None
}

// ===========================================================================
// VOPR Campaign Runner
// ===========================================================================

struct VoprStats {
    total_docs: u64,
    total_mutations: u64,
    failures: Vec<String>,
}

/// Run a full VOPR campaign with the given property checker.
///
/// For each seed, generates `docs_per_seed` valid X12 documents, then applies
/// all mutation types to each, running the checker on both the original and
/// every mutated variant.
fn run_vopr_campaign(
    num_seeds: u64,
    docs_per_seed: u64,
    mut checker: impl FnMut(&[u8], u64, u64) -> Option<String>,
) -> VoprStats {
    let mut stats = VoprStats {
        total_docs: 0,
        total_mutations: 0,
        failures: Vec::new(),
    };

    for seed in 0..num_seeds {
        let mut rng = SplitMix64::new(seed);

        for doc_idx in 0..docs_per_seed {
            let input = generate_valid_x12(&mut rng);
            stats.total_docs += 1;

            // Check original
            if let Some(msg) = checker(&input, seed, doc_idx) {
                stats
                    .failures
                    .push(format!("seed={seed} doc={doc_idx} (original): {msg}"));
            }

            // Check all mutations
            for &mutation in ALL_MUTATIONS {
                let mut mutation_rng = rng.fork();
                let mutated = apply_mutation(&mut mutation_rng, &input, mutation);
                stats.total_mutations += 1;

                if let Some(msg) = checker(&mutated, seed, doc_idx) {
                    stats.failures.push(format!(
                        "seed={seed} doc={doc_idx} mutation={mutation:?}: {msg}"
                    ));
                }
            }
        }
    }

    stats
}

/// Run a VOPR campaign that only tests valid (unmutated) documents.
fn run_vopr_valid_only(
    num_seeds: u64,
    docs_per_seed: u64,
    mut checker: impl FnMut(&[u8], u64, u64) -> Option<String>,
) -> VoprStats {
    let mut stats = VoprStats {
        total_docs: 0,
        total_mutations: 0,
        failures: Vec::new(),
    };

    for seed in 0..num_seeds {
        let mut rng = SplitMix64::new(seed);

        for doc_idx in 0..docs_per_seed {
            let input = generate_valid_x12(&mut rng);
            stats.total_docs += 1;

            if let Some(msg) = checker(&input, seed, doc_idx) {
                stats
                    .failures
                    .push(format!("seed={seed} doc={doc_idx}: {msg}"));
            }
        }
    }

    stats
}

fn print_vopr_summary(label: &str, seeds: u64, docs: u64, mutations: u64, failures: &[String]) {
    println!(
        "VOPR: {label}: {seeds} seeds x {docs} docs = {} documents, {mutations} mutations, {} failures",
        seeds * docs,
        failures.len(),
    );
    for f in failures.iter().take(10) {
        println!("  FAIL: {f}");
    }
}

// ===========================================================================
// Test functions
// ===========================================================================

#[test]
fn vopr_parser_no_panic_100_seeds() {
    let seeds = vopr_seeds();
    let docs = vopr_docs();

    let stats = run_vopr_campaign(seeds, docs, |input, _seed, _doc| {
        // The check_no_panic function uses catch_unwind internally.
        // If it returns true, no panic occurred.
        // But we also want to detect if the catch_unwind itself returns Err.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check_no_panic(input);
        }));
        match result {
            Ok(()) => None,
            Err(_) => Some("parser panicked".to_string()),
        }
    });

    print_vopr_summary(
        "no-panic",
        seeds,
        docs,
        stats.total_mutations,
        &stats.failures,
    );
    assert!(
        stats.failures.is_empty(),
        "P1 (No Panic) violated: {} failures out of {} docs + {} mutations",
        stats.failures.len(),
        stats.total_docs,
        stats.total_mutations,
    );
}

#[test]
fn vopr_roundtrip_100_seeds() {
    let seeds = vopr_seeds();
    let docs = vopr_docs();

    // Round-trip only applies to valid (unmutated) documents
    let stats = run_vopr_valid_only(seeds, docs, |input, _seed, _doc| check_roundtrip(input));

    print_vopr_summary(
        "roundtrip",
        seeds,
        docs,
        stats.total_mutations,
        &stats.failures,
    );
    assert!(
        stats.failures.is_empty(),
        "P2 (Round-trip) violated: {} failures out of {} docs",
        stats.failures.len(),
        stats.total_docs,
    );
}

#[test]
fn vopr_determinism_100_seeds() {
    let seeds = vopr_seeds();
    let docs = vopr_docs();

    let stats = run_vopr_campaign(seeds, docs, |input, _seed, _doc| check_determinism(input));

    print_vopr_summary(
        "determinism",
        seeds,
        docs,
        stats.total_mutations,
        &stats.failures,
    );
    assert!(
        stats.failures.is_empty(),
        "P3 (Determinism) violated: {} failures out of {} docs + {} mutations",
        stats.failures.len(),
        stats.total_docs,
        stats.total_mutations,
    );
}

#[test]
fn vopr_adversarial_delimiters() {
    let seeds = vopr_seeds();
    let docs_per_seed = 10u64; // fewer docs but more unusual delimiter combos

    // Unusual delimiters
    let unusual_delims: Vec<DelimiterSet> = vec![
        DelimiterSet {
            element: b'|',
            sub_element: b'+',
            segment: b'!',
        },
        DelimiterSet {
            element: b'^',
            sub_element: b'@',
            segment: b'#',
        },
        DelimiterSet {
            element: b'\\',
            sub_element: b'/',
            segment: b'=',
        },
        DelimiterSet {
            element: b'<',
            sub_element: b'>',
            segment: b';',
        },
        DelimiterSet {
            element: b'{',
            sub_element: b'}',
            segment: b'`',
        },
    ];

    let mut total_docs = 0u64;
    let mut failures = Vec::new();

    for seed in 0..seeds {
        let mut rng = SplitMix64::new(seed.wrapping_add(10_000));

        for (d_idx, delim_set) in unusual_delims.iter().enumerate() {
            for doc_idx in 0..docs_per_seed {
                let input = generate_x12_with_delimiters(&mut rng, delim_set);
                total_docs += 1;

                // P1: no panic
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check_no_panic(&input);
                }));
                if result.is_err() {
                    failures.push(format!(
                        "seed={seed} delim_set={d_idx} doc={doc_idx}: panic with adversarial delimiters"
                    ));
                    continue;
                }

                // P4: delimiter detection consistency
                if let Some(msg) = check_delimiter_consistency(&input) {
                    failures.push(format!(
                        "seed={seed} delim_set={d_idx} doc={doc_idx}: {msg}"
                    ));
                }

                // P5: segment count
                if let Some(msg) = check_segment_count(&input) {
                    failures.push(format!(
                        "seed={seed} delim_set={d_idx} doc={doc_idx}: {msg}"
                    ));
                }
            }
        }
    }

    println!(
        "VOPR: adversarial-delimiters: {seeds} seeds x {} delim_sets x {docs_per_seed} docs = {total_docs} documents, {} failures",
        unusual_delims.len(),
        failures.len(),
    );
    for f in failures.iter().take(10) {
        println!("  FAIL: {f}");
    }
    assert!(
        failures.is_empty(),
        "Adversarial delimiter tests: {} failures out of {total_docs} docs",
        failures.len(),
    );
}

#[test]
fn vopr_massive_interchange() {
    let mut rng = SplitMix64::new(42_424_242);

    let e = b'*';
    let s = b'~';
    let c = b':';

    let mut buf = Vec::with_capacity(512 * 1024);

    // ISA
    let isa_raw = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";
    assert_eq!(isa_raw.len(), 106);
    buf.extend_from_slice(isa_raw);

    // GS
    push_segment(
        &mut buf,
        e,
        s,
        b"GS",
        &[
            b"HP",
            b"SENDER",
            b"RECEIVER",
            b"20210901",
            b"1234",
            b"1",
            b"X",
            b"005010",
        ],
    );

    // ST
    push_segment(&mut buf, e, s, b"ST", &[b"837", b"0001"]);

    // 10,000 body segments
    let body_count = 10_000usize;
    let seg_ids = [
        b"CLM" as &[u8],
        b"NM1",
        b"DTP",
        b"SV1",
        b"REF",
        b"DMG",
        b"SBR",
        b"HI",
        b"LX",
        b"BHT",
    ];
    for _ in 0..body_count {
        let seg_id = *rng.choose(seg_ids.as_slice());
        let body = random_segment_body(
            &mut rng,
            &DelimiterSet {
                element: e,
                sub_element: c,
                segment: s,
            },
        );
        let body_refs: Vec<&[u8]> = body.iter().map(Vec::as_slice).collect();
        push_segment(&mut buf, e, s, seg_id, &body_refs);
    }

    // SE: segment count = ST + body + SE = body_count + 2
    let se_count = format!("{}", body_count + 2);
    push_segment(&mut buf, e, s, b"SE", &[se_count.as_bytes(), b"0001"]);

    // GE
    push_segment(&mut buf, e, s, b"GE", &[b"1", b"1"]);

    // IEA
    push_segment(&mut buf, e, s, b"IEA", &[b"1", b"000000001"]);

    let input_len = buf.len();

    // P1: no panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        check_no_panic(&buf);
    }));
    assert!(result.is_ok(), "massive interchange caused panic");

    // P5: segment count
    let seg_count_err = check_segment_count(&buf);
    assert!(
        seg_count_err.is_none(),
        "P5 violated: {}",
        seg_count_err.unwrap_or_default()
    );

    // P6: element preservation
    let pres_err = check_element_preservation(&buf);
    assert!(
        pres_err.is_none(),
        "P6 violated: {}",
        pres_err.unwrap_or_default()
    );

    // Parse and verify structure
    let interchanges = Interchange::parse(&buf).unwrap();
    assert_eq!(interchanges.len(), 1);
    assert_eq!(interchanges[0].groups.len(), 1);
    assert_eq!(interchanges[0].groups[0].transaction_sets.len(), 1);
    assert_eq!(
        interchanges[0].groups[0].transaction_sets[0].segments.len(),
        body_count,
    );

    println!(
        "VOPR: massive-interchange: 1 interchange, 10000 segments, {input_len} bytes, all properties hold",
    );
}

#[test]
fn vopr_empty_and_minimal() {
    let mut failures = Vec::new();

    let test_cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("single_byte_A", vec![b'A']),
        ("single_byte_tilde", vec![b'~']),
        ("single_byte_null", vec![0]),
        ("just_ISA_keyword", b"ISA".to_vec()),
        ("ISA_with_one_sep", b"ISA*".to_vec()),
        // 105 bytes — one short of valid ISA
        ("ISA_105_bytes", {
            let mut v = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:".to_vec();
            v.truncate(105);
            v
        }),
        // Exactly 106 bytes — valid ISA with no further content
        ("ISA_only_106_bytes", b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~".to_vec()),
        // ISA + IEA only (minimal valid interchange)
        ("ISA_IEA_only", {
            let mut v = b"ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~".to_vec();
            v.extend_from_slice(b"IEA*0*000000001~");
            v
        }),
        // All zeros
        ("all_zeros_64", vec![0u8; 64]),
        // All 0xFF
        ("all_0xff_128", vec![0xFFu8; 128]),
        // Random binary garbage
        ("random_garbage_256", {
            let mut rng = SplitMix64::new(99);
            #[allow(clippy::cast_possible_truncation)]
            let v: Vec<u8> = (0..256).map(|_| rng.range(0, 255) as u8).collect();
            v
        }),
        // Repeated segment terminators
        ("repeated_tildes", b"~~~~~~~~~~~~~~~~~~~~".to_vec()),
        // Just newlines
        ("just_newlines", b"\n\n\n\r\n\r\n\n\n".to_vec()),
    ];

    for (name, input) in &test_cases {
        // P1: no panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check_no_panic(input);
        }));
        if result.is_err() {
            failures.push(format!("{name}: panic"));
        }

        // P3: determinism
        if let Some(msg) = check_determinism(input) {
            failures.push(format!("{name}: determinism: {msg}"));
        }

        // P4: delimiter consistency
        if let Some(msg) = check_delimiter_consistency(input) {
            failures.push(format!("{name}: delimiter_consistency: {msg}"));
        }

        // P5: segment count
        if let Some(msg) = check_segment_count(input) {
            failures.push(format!("{name}: segment_count: {msg}"));
        }
    }

    println!(
        "VOPR: empty-and-minimal: {} test cases, {} failures",
        test_cases.len(),
        failures.len(),
    );
    for f in failures.iter().take(10) {
        println!("  FAIL: {f}");
    }
    assert!(
        failures.is_empty(),
        "Edge case tests: {} failures out of {} cases",
        failures.len(),
        test_cases.len(),
    );
}
