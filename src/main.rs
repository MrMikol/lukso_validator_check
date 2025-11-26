mod scan;

use anyhow::Result;
use reqwest::Client;
use scan::{get_last_page, scan_included_deposits, IconColor};

use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .user_agent("lukso-validator-checker/0.1")
        .build()?;

    // 1) Discover how many pages exist
    let last_page = get_last_page(&client).await?;

    // 2) Scan pages [1..=last_page] with limited concurrency
    let concurrency = 5; // good daily-safe default
    let hits = scan_included_deposits(&client, 1, last_page, concurrency).await?;

    // Split into red / yellow
    let red_hits: Vec<_> = hits
        .iter()
        .filter(|h| matches!(h.color, IconColor::Red))
        .collect();
    let yellow_hits: Vec<_> = hits
        .iter()
        .filter(|h| matches!(h.color, IconColor::Yellow))
        .collect();

    let red_count = red_hits.len();
    let yellow_count = yellow_hits.len();
    let total_pages_checked = last_page - 1 + 1; // since we start at 1

    println!("\n================ SUMMARY ================");
    println!("Pages checked              : {}", total_pages_checked);
    println!("🟥 Red Active validators   : {}", red_count);
    println!("🟨 Yellow Active validators: {}", yellow_count);
    println!("------------------------------------------");
    println!(
        "Total problematic validators: {}",
        red_count + yellow_count
    );
    println!("==========================================\n");

    // Write red hits to file
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

    // Write yellow hits to file
    if yellow_count > 0 {
        let mut y_file = File::create("yellow_validators.txt")?;
        for h in &yellow_hits {
            writeln!(
                y_file,
                "page={} url={} index={} public_key={} color=yellow",
                h.page, h.url, h.index, h.public_key
            )?;
        }
        println!(
            "➡ Wrote {} yellow validators to yellow_validators.txt",
            yellow_count
        );
    } else {
        println!("➡ No yellow validators found.");
    }

    // Optional console preview
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

    Ok(())
}
