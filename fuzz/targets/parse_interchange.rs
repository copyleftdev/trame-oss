use afl::fuzz;

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(interchanges) = trame_wire::parse_interchanges(data) {
            for ix in &interchanges {
                // Touch all ISA fields
                let _ = std::str::from_utf8(ix.isa.sender_id);
                let _ = std::str::from_utf8(ix.isa.receiver_id);
                let _ = std::str::from_utf8(ix.isa.control_number);
                let _ = std::str::from_utf8(ix.isa.version);

                for group in &ix.groups {
                    let _ = std::str::from_utf8(group.gs.functional_id);
                    let _ = std::str::from_utf8(group.gs.version);

                    for txn in &group.transaction_sets {
                        let _ = std::str::from_utf8(txn.st.transaction_set_id);
                        let _ = txn.segments.len();
                        for seg in &txn.segments {
                            let _ = seg.id();
                            let _ = seg.element_count();
                        }
                    }
                }
            }
        }
    });
}
