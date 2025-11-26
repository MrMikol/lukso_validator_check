mod scan;

use anyhow::Result;
use reqwest::Client;
use scan::{scan_included_deposits, IconColor};

use std::fs::File;      // NEW
use std::io::Write;     // NEW

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .user_agent("lukso-validator-checker/0.1")
        .build()?;

    // Scan up to 1000 pages (stopping early if no rows)
    let hits = scan_included_deposits(&client, 1, 1000, true).await?;

    // --- SEPARATE RED AND YELLOW ----------------------------------------- // NEW
    let red_hits: Vec<_> = hits
        .iter()
        .filter(|h| matches!(h.color, IconColor::Red))
        .collect();

    let yellow_hits: Vec<_> = hits
        .iter()
        .filter(|h| matches!(h.color, IconColor::Yellow))
        .collect();
    // ---------------------------------------------------------------------- // END NEW

    // --- SUMMARY ----------------------------------------------------------- // NEW
    let red_count = red_hits.len();
    let yellow_count = yellow_hits.len();

    println!("\n================ SUMMARY ================");
    println!("🟥 Red Active validators   : {}", red_count);
    println!("🟨 Yellow Active validators: {}", yellow_count);
    println!("------------------------------------------");
    println!("Total problematic validators: {}", red_count + yellow_count);
    println!("==========================================\n");
    // ---------------------------------------------------------------------- // END NEW

    // --- WRITE RED HITS TO FILE ------------------------------------------ // NEW
    if red_count > 0 {
        let mut red_file = File::create("red_validators.txt")?;
        for h in &red_hits {
            writeln!(
                red_file,
                "page={} url={} index={} public_key={} color=red",
                h.page, h.url, h.index, h.public_key
            )?;
        }
        println!("➡ Wrote {} red validators to red_validators.txt", red_count);
    } else {
        println!("➡ No red validators found.");
    }
    // ---------------------------------------------------------------------- // END NEW

    // --- WRITE YELLOW HITS TO FILE --------------------------------------- // NEW
    if yellow_count > 0 {
        let mut y_file = File::create("yellow_validators.txt")?;
        for h in &yellow_hits {
            writeln!(
                y_file,
                "page={} url={} index={} public_key={} color=yellow",
                h.page, h.url, h.index, h.public_key
            )?;
        }
        println!("➡ Wrote {} yellow validators to yellow_validators.txt", yellow_count);
    } else {
        println!("➡ No yellow validators found.");
    }
    // ---------------------------------------------------------------------- // END NEW

    // --- OPTIONAL: Show lists in console (can remove if noisy) ------------ // NEW
    println!("\n=== RED VALIDATORS (console preview) ===");
    for h in &red_hits {
        println!(
            "page={} url={} index={} public_key={} color=red",
            h.page, h.url, h.index, h.public_key
        );
    }

    println!("\n=== YELLOW VALIDATORS (console preview) ===");
    for h in &yellow_hits {
        println!(
            "page={} url={} index={} public_key={} color=yellow",
            h.page, h.url, h.index, h.public_key
        );
    }
    // ---------------------------------------------------------------------- // END NEW

    Ok(())
}
