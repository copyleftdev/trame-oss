pub mod fmt;
pub mod info;

/// Read input from a file argument or stdin.
pub fn read_input(args: &[String]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if let Some(path) = args.first() {
        if path != "-" && !path.starts_with('-') {
            return Ok(std::fs::read(path)?);
        }
    }
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut std::io::stdin(), &mut buf)?;
    Ok(buf)
}
