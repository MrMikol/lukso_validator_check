mod scan;

use anyhow::Result;
use scan::{scan_included_deposits, IconColor};
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .user_agent("lukso-validator-checker/0.1 (github.com/your-org/your-repo)")
        .build()?;

    // Example: scan first 1000 pages, stop early if a page has no rows
    let hits = scan_included_deposits(&client, 1, 1000, true).await?;

    println!("\n=== SUMMARY ===");
    let red_count = hits.iter().filter(|h| matches!(h.color, IconColor::Red)).count();
    let yellow_count = hits.iter().filter(|h| matches!(h.color, IconColor::Yellow)).count();
    println!("Red Active validators : {red_count}");
    println!("Yellow Active validators: {yellow_count}");
    println!("Total hits: {}", hits.len());

    println!("\n=== DETAILS (page, url, index, public_key, color) ===");
    for h in &hits {
        let color_str = match h.color {
            IconColor::Red => "red",
            IconColor::Yellow => "yellow",
        };
        println!(
            "page={} url={} index={} public_key={} color={}",
            h.page, h.url, h.index, h.public_key, color_str
        );
    }

    Ok(())
}