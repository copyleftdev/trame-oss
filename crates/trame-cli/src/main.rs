#![forbid(unsafe_code)]

mod commands;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let result = match args[1].as_str() {
        "fmt" => commands::fmt::run(&args[2..]),
        "info" => commands::info::run(&args[2..]),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        "version" | "--version" | "-V" => {
            print_version();
            Ok(())
        }
        other => {
            eprintln!("trame: unknown command '{other}'");
            eprintln!("Run 'trame help' for usage.");
            eprintln!();
            eprintln!("Additional commands available with trame-pro:");
            eprintln!("    validate, to-json, from-json, fake, receive, status, search");
            eprintln!("    See: https://github.com/copyleftdev/trame");
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("trame: error: {e}");
        std::process::exit(1);
    }
}

fn print_version() {
    println!("trame {}", env!("CARGO_PKG_VERSION"));
}

fn print_usage() {
    println!(
        "trame {} — the X12 EDI Swiss Army knife",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    trame <command> [options] [file]");
    println!();
    println!("COMMANDS:");
    println!("    fmt         Pretty-print X12 (one segment per line)");
    println!("    info        Show interchange/group/transaction summary");
    println!("    help        Show this help message");
    println!("    version     Show version");
    println!();
    println!("PRO COMMANDS (requires trame-pro):");
    println!("    validate    Validate X12 against implementation guides");
    println!("    to-json     Convert X12 to JSON");
    println!("    from-json   Convert JSON back to X12");
    println!("    fake        Generate fake X12 test data");
    println!("    receive     Receive and store an X12 interchange");
    println!("    status      Show interchange lifecycle status");
    println!("    search      Query stored interchanges");
    println!();
    println!("INPUT:");
    println!("    Pass a file path, or pipe via stdin:");
    println!("    trame fmt claim.edi");
    println!("    cat claim.edi | trame fmt");
}
