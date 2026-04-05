//! Show interchange/group/transaction summary.

use super::read_input;
use trame_wire::parse_interchanges;

/// Describe a functional identifier code.
fn describe_functional_id(id: &str) -> &'static str {
    match id {
        "HC" => "Health Care",
        "HP" => "Health Care Claim Payment/Advice",
        "HS" => "Eligibility, Coverage or Benefit Inquiry",
        "HB" => "Eligibility, Coverage or Benefit Information",
        "HN" => "Health Care Claim Status Notification",
        "HR" => "Health Care Claim Status Request",
        "FA" => "Functional Acknowledgment",
        "RA" => "Remittance Advice",
        _ => "Unknown",
    }
}

/// Describe a transaction set ID.
fn describe_transaction_set(id: &str) -> &'static str {
    match id {
        "270" => "Eligibility Inquiry",
        "271" => "Eligibility Response",
        "276" => "Claim Status Request",
        "277" => "Claim Status Response",
        "278" => "Health Care Services Review",
        "835" => "Claim Payment/Remittance Advice",
        "837" => "Health Care Claim",
        "820" => "Premium Payment",
        "834" => "Benefit Enrollment",
        "997" => "Functional Acknowledgment",
        "999" => "Implementation Acknowledgment",
        _ => "Unknown",
    }
}

/// Describe a usage indicator.
fn describe_usage(indicator: &str) -> &'static str {
    match indicator {
        "P" => "P (Production)",
        "T" => "T (Test)",
        "I" => "I (Information)",
        _ => "Unknown",
    }
}

/// Run the `info` command.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let input = read_input(args)?;
    let interchanges = parse_interchanges(&input)?;

    if interchanges.is_empty() {
        println!("No interchanges found in input.");
        return Ok(());
    }

    for ic in &interchanges {
        let control = std::str::from_utf8(ic.isa.control_number)
            .unwrap_or("???")
            .trim();
        let sender_q = std::str::from_utf8(ic.isa.sender_qualifier)
            .unwrap_or("??")
            .trim();
        let sender_id = std::str::from_utf8(ic.isa.sender_id)
            .unwrap_or("???")
            .trim();
        let receiver_q = std::str::from_utf8(ic.isa.receiver_qualifier)
            .unwrap_or("??")
            .trim();
        let receiver_id = std::str::from_utf8(ic.isa.receiver_id)
            .unwrap_or("???")
            .trim();
        let date = std::str::from_utf8(ic.isa.date).unwrap_or("??????");
        let time = std::str::from_utf8(ic.isa.time).unwrap_or("????");
        let version = std::str::from_utf8(ic.isa.version)
            .unwrap_or("?????")
            .trim();
        let usage = std::str::from_utf8(ic.isa.usage_indicator)
            .unwrap_or("?")
            .trim();

        println!("Interchange: {control}");
        println!("  Sender:   {sender_q}/{sender_id}");
        println!("  Receiver: {receiver_q}/{receiver_id}");
        println!("  Date:     {date} {time}");
        println!("  Version:  {version}");
        println!("  Usage:    {}", describe_usage(usage));
        println!("  Groups:   {}", ic.groups.len());

        for (gi, grp) in ic.groups.iter().enumerate() {
            let func_id = std::str::from_utf8(grp.gs.functional_id).unwrap_or("??");
            let gs_version = std::str::from_utf8(grp.gs.version).unwrap_or("???");
            let gs_control = std::str::from_utf8(grp.gs.control_number)
                .unwrap_or("?")
                .trim();

            println!(
                "    Group {}: {} ({}) v{}",
                gi + 1,
                func_id,
                describe_functional_id(func_id),
                gs_version
            );
            println!("      Control: {gs_control}");
            println!("      Transactions: {}", grp.transaction_sets.len());

            for (ti, txn) in grp.transaction_sets.iter().enumerate() {
                let ts_id = std::str::from_utf8(txn.st.transaction_set_id).unwrap_or("???");
                let ts_desc = describe_transaction_set(ts_id);
                // Total segments = body segments + ST + SE
                let total_segs = txn.segments.len() + 2;
                println!(
                    "        [{}] {} {} — {} segments",
                    ti + 1,
                    ts_id,
                    ts_desc,
                    total_segs
                );
            }
        }
    }

    Ok(())
}
