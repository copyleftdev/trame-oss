//! Pretty-print X12 with one segment per line, indented by envelope depth.

use super::read_input;
use trame_wire::Parser;

/// Run the `fmt` command.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let input = read_input(args)?;
    let parser = Parser::new(&input)?;
    let delimiters = parser.delimiters();

    for result in parser {
        let seg = result?;
        let id = seg.id_str().unwrap_or("???");

        let indent = match id {
            "ISA" | "IEA" => "",
            "GS" | "GE" => "  ",
            "ST" | "SE" => "    ",
            _ => "      ",
        };

        // Reconstruct the segment with its original delimiters.
        let raw = seg.raw();
        let raw_str = std::str::from_utf8(raw).unwrap_or("???");
        println!("{indent}{raw_str}{}", delimiters.segment as char);
    }

    Ok(())
}
