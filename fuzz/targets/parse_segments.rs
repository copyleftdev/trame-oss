use afl::fuzz;

fn main() {
    fuzz!(|data: &[u8]| {
        // Try auto-detect parser
        if let Ok(parser) = trame_wire::Parser::new(data) {
            for segment in parser {
                if let Ok(seg) = segment {
                    let _ = seg.id();
                    let _ = seg.id_str();
                    let _ = seg.element_count();
                    let _ = seg.raw();
                    // Access every element
                    for i in 0..seg.element_count() {
                        let _ = seg.element(i);
                        let _ = seg.element_str(i);
                        let _ = seg.sub_elements(i).map(|iter| iter.count());
                    }
                }
            }
        }

        // Also try with explicit default delimiters
        let parser = trame_wire::Parser::with_delimiters(data, trame_wire::Delimiters::default());
        for segment in parser {
            if let Ok(seg) = segment {
                let _ = seg.id();
                let _ = seg.element_count();
            }
        }
    });
}
