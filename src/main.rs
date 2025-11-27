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

    // 1) Discover number of pages
    let last_page = get_last_page(&client).await?;

    // 2) Scan all pages using concurrency 5
    let concurrency = 5;
    let hits = scan_included_deposits(&client, 1, last_page, concurrency).await?;

    // Separate red, yellow, green
    let red_hits: Vec<_> = hits.iter().filter(|h| matches!(h.color, IconColor::Red)).collect();
    let yellow_hits: Vec<_> = hits.iter().filter(|h| matches!(h.color, IconColor::Yellow)).collect();
    let green_hits: Vec<_> = hits.iter().filter(|h| matches!(h.color, IconColor::Green)).collect();

    let red_count = red_hits.len();
    let yellow_count = yellow_hits.len();
    let green_count = green_hits.len();

    println!("\n================ SUMMARY ================");
    println!("Pages checked               : {}", last_page);
    println!("🟥 Red Active validators    : {}", red_count);
    println!("🟨 Yellow Active validators : {}", yellow_count);
    println!("🟩 Green Active validators  : {}", green_count);
    println!("------------------------------------------");
    println!("Total matched validators     : {}", hits.len());
    println!("==========================================\n");

    // Write red
    if red_count > 0 {
        let mut f = File::create("red_validators.txt")?;
        for h in &red_hits {
            writeln!(
                f,
                "page={} url={} index={} public_key={} color=red",
                h.page, h.url, h.index, h.public_key
            )?;
        }
        println!("➡ Wrote {} red validators to red_validators.txt", red_count);
    } else {
        println!("➡ No red validators found.");
    }

    // Write yellow
    if yellow_count > 0 {
        let mut f = File::create("yellow_validators.txt")?;
        for h in &yellow_hits {
            writeln!(
                f,
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

    // Write green
    if green_count > 0 {
        let mut f = File::create("green_validators.txt")?;
        for h in &green_hits {
            writeln!(
                f,
                "page={} url={} index={} public_key={} color=green",
                h.page, h.url, h.index, h.public_key
            )?;
        }
        println!(
            "➡ Wrote {} green validators to green_validators.txt",
            green_count
        );
    } else {
        println!("➡ No green validators found.");
    }

    Ok(())
}
