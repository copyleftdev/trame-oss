use afl::fuzz;

fn main() {
    fuzz!(|data: &[u8]| {
        // Parse
        let parser = match trame_wire::Parser::new(data) {
            Ok(p) => p,
            Err(_) => return,
        };

        let delims = parser.delimiters();
        let segments: Vec<_> = parser.filter_map(|r| r.ok()).collect();

        if segments.is_empty() {
            return;
        }

        // Write back
        let mut writer = trame_wire::Writer::new(delims);
        for seg in &segments {
            let elements: Vec<&[u8]> = seg.elements().collect();
            if elements.is_empty() {
                continue;
            }
            writer.write_segment(elements[0], &elements[1..]);
        }
        let output = writer.finish();

        // Re-parse and verify segment count matches
        let parser2 = trame_wire::Parser::with_delimiters(&output, delims);
        let segments2: Vec<_> = parser2.filter_map(|r| r.ok()).collect();
        assert_eq!(
            segments.len(),
            segments2.len(),
            "roundtrip segment count mismatch"
        );
    });
}
