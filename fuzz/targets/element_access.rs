use afl::fuzz;

fn main() {
    fuzz!(|data: &[u8]| {
        let parser =
            trame_wire::Parser::with_delimiters(data, trame_wire::Delimiters::default());
        for segment in parser {
            if let Ok(seg) = segment {
                // Exhaustive element access at every possible index
                for i in 0..256 {
                    let _ = seg.element(i);
                    let _ = seg.element_str(i);
                    if let Some(sub_iter) = seg.sub_elements(i) {
                        for sub in sub_iter {
                            let _ = std::str::from_utf8(sub);
                        }
                    }
                }
            }
        }
    });
}
