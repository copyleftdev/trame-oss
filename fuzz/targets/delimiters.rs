use afl::fuzz;

fn main() {
    fuzz!(|data: &[u8]| {
        // Delimiter detection should never panic regardless of input
        let _ = trame_wire::Delimiters::detect(data);
    });
}
