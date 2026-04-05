//! Criterion benchmarks for trame-wire.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use trame_wire::{Delimiters, Interchange, Parser};

/// Build a realistic 837P professional claim interchange.
fn build_837p_interchange() -> Vec<u8> {
    let isa = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *210901*1234*U*00401*000000001*0*P*:~";

    let mut buf = String::from(isa);
    buf.push_str("GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1~");

    // Generate 10 transaction sets, each with ~20 segments.
    for tx_num in 1..=10 {
        buf.push_str(&format!("ST*837*{tx_num:04}*005010X222A1~"));
        buf.push_str(&format!(
            "BHT*0019*00*{tx_num:06}*20210901*1234*CH~"
        ));

        // Subscriber loop
        buf.push_str("HL*1**20*1~");
        buf.push_str("SBR*P*18*******CI~");
        buf.push_str("NM1*IL*1*DOE*JOHN****MI*12345678901~");
        buf.push_str("N3*123 MAIN ST~");
        buf.push_str("N4*ANYTOWN*CA*90210~");
        buf.push_str("DMG*D8*19800101*M~");

        // Payer loop
        buf.push_str("NM1*PR*2*AETNA*****PI*12345~");
        buf.push_str("N3*PO BOX 981106~");
        buf.push_str("N4*EL PASO*TX*79998~");

        // Patient loop
        buf.push_str("HL*2*1*22*0~");
        buf.push_str("CLM*CLAIM001*150***11:B:1*Y*A*Y*I~");

        // Service lines
        for line in 1..=3 {
            buf.push_str(&format!(
                "LX*{line}~\
                 SV1*HC:99213:25*50*UN*1***1~\
                 DTP*472*D8*20210901~"
            ));
        }

        // Count: ST + BHT + HL + SBR + NM1 + N3 + N4 + DMG + NM1 + N3 + N4 + HL + CLM
        //        + 3*(LX + SV1 + DTP) + SE = 13 + 9 + 1 = 23
        let seg_count = 22; // ST through last DTP + SE
        buf.push_str(&format!("SE*{seg_count}*{tx_num:04}~"));
    }

    buf.push_str("GE*10*1~");
    buf.push_str("IEA*1*000000001~");

    buf.into_bytes()
}

fn bench_delimiter_detection(c: &mut Criterion) {
    let input = build_837p_interchange();
    c.bench_function("delimiter_detection", |b| {
        b.iter(|| Delimiters::detect(black_box(&input)));
    });
}

fn bench_segment_iteration(c: &mut Criterion) {
    let input = build_837p_interchange();
    c.bench_function("segment_iteration", |b| {
        b.iter(|| {
            let parser = Parser::new(black_box(&input)).unwrap();
            let mut count = 0u64;
            for seg in parser {
                let seg = seg.unwrap();
                black_box(seg.id());
                count += 1;
            }
            count
        });
    });
}

fn bench_full_interchange_parse(c: &mut Criterion) {
    let input = build_837p_interchange();
    c.bench_function("full_interchange_parse", |b| {
        b.iter(|| {
            let interchanges = Interchange::parse(black_box(&input)).unwrap();
            black_box(&interchanges);
        });
    });
}

fn bench_element_access(c: &mut Criterion) {
    let input = build_837p_interchange();
    c.bench_function("element_access", |b| {
        b.iter(|| {
            let parser = Parser::new(black_box(&input)).unwrap();
            let mut total = 0usize;
            for seg in parser {
                let seg = seg.unwrap();
                for i in 0..seg.element_count() {
                    if let Some(elem) = seg.element(i) {
                        total += elem.len();
                    }
                }
            }
            total
        });
    });
}

criterion_group!(
    benches,
    bench_delimiter_detection,
    bench_segment_iteration,
    bench_full_interchange_parse,
    bench_element_access,
);
criterion_main!(benches);
