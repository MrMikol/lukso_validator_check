// Cargo.toml dependencies:
// [dependencies]
// reqwest = { version = "0.11", features = ["blocking"] }
// scraper = "0.18"
// tokio = { version = "1", features = ["full"] }

use reqwest;
use scraper::{Html, Selector};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let total_pages = 1887;
    let base_url = "https://dora.explorer.mainnet.lukso.network/validators/included_deposits?f&f.orphaned=1&f.valid=1&c=50&p=";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    let mut total_active_with_red = 0;

    println!("Starting validator check...");
    println!("Total pages to check: {}", total_pages);
    println!("This will take a while...\n");

    // First, let's check page 1 and print what we find for verification
    println!("=== CHECKING PAGE 1 FOR VERIFICATION ===");
    let url = format!("{}{}", base_url, 1);
    let response = client.get(&url).send().await?;
    let body = response.text().await?;
    let document = Html::parse_document(&body);

    // Print the structure of the first few rows for debugging
    let row_selector = Selector::parse("tbody tr").unwrap();
    let rows: Vec<_> = document.select(&row_selector).collect();

    println!("Found {} rows on page 1", rows.len());
    println!("\n--- First 3 rows (for verification) ---");

    for (i, row) in rows.iter().take(3).enumerate() {
        println!("\nRow {}:", i + 1);
        println!("HTML: {}", row.html());
        println!("Text: {}", row.text().collect::<Vec<_>>().join(" "));

        // Check for active status
        let text_lower = row.text().collect::<String>().to_lowercase();
        let has_active = text_lower.contains("active");

        // Check for red icon (multiple possible selectors)
        let has_red = row.select(&Selector::parse("[class*='red']").unwrap()).next().is_some()
            || row.select(&Selector::parse(".text-danger").unwrap()).next().is_some()
            || row.select(&Selector::parse(".badge-danger").unwrap()).next().is_some()
            || row.select(&Selector::parse("svg.text-danger").unwrap()).next().is_some();

        println!("Has 'active': {}", has_active);
        println!("Has red icon: {}", has_red);
        println!("---");
    }

    println!("\n\nDoes this look correct? (Press Ctrl+C to stop, or it will continue after 5 seconds)");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Now proceed with full scan
    println!("\n=== STARTING FULL SCAN ===\n");

    for page in 1..=total_pages {
        let url = format!("{}{}", base_url, page);

        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(body) = response.text().await {
                    let document = Html::parse_document(&body);
                    let row_selector = Selector::parse("tbody tr").unwrap();

                    let mut page_count = 0;

                    for row in document.select(&row_selector) {
                        let text_lower = row.text().collect::<String>().to_lowercase();
                        let has_active = text_lower.contains("active");

                        let has_red = row.select(&Selector::parse("[class*='red']").unwrap()).next().is_some()
                            || row.select(&Selector::parse(".text-danger").unwrap()).next().is_some()
                            || row.select(&Selector::parse(".badge-danger").unwrap()).next().is_some()
                            || row.select(&Selector::parse("svg.text-danger").unwrap()).next().is_some();

                        if has_active && has_red {
                            page_count += 1;
                        }
                    }

                    total_active_with_red += page_count;

                    if page % 10 == 0 || page == 1 {
                        println!("Page {}/{}: Found {} on this page (Total: {})",
                                 page, total_pages, page_count, total_active_with_red);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error on page {}: {}", page, e);
            }
        }

        // Rate limiting - wait 500ms between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n========== FINAL RESULTS ==========");
    println!("Total active validators with red icon: {}", total_active_with_red);
    println!("===================================");

    Ok(())
}