use afl::fuzz;
use trame_schema::{Registry, SchemaWalker};

fn main() {
    let registry = Registry::new();

    fuzz!(|data: &[u8]| {
        // Parse segments
        let parser = match trame_wire::Parser::new(data) {
            Ok(p) => p,
            Err(_) => return,
        };

        let segments: Vec<_> = parser.filter_map(|r| r.ok()).collect();
        if segments.is_empty() {
            return;
        }

        // Try to identify the transaction set from ST segment
        let st_seg = segments.iter().find(|s| s.id() == b"ST");
        let ts_id = st_seg
            .and_then(|s| s.element_str(1))
            .unwrap_or("997");

        // Look up schema and walk
        if let Some(schema) = registry.lookup(ts_id, "005010") {
            let mut walker = SchemaWalker::new(schema);
            for seg in &segments {
                let qualifier = seg.element(1);
                let _ = walker.feed(seg.id(), qualifier);
            }
        }

        // Also try all schemas
        for schema_def in registry.all() {
            let mut walker = SchemaWalker::new(schema_def);
            for seg in &segments {
                let _ = walker.feed(seg.id(), seg.element(1));
            }
        }
    });
}
