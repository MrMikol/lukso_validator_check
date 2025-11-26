use anyhow::{Context, Result};
use futures::future::join_all;              // NEW: for batching async requests
use reqwest::Client;
use scraper::{Html, Selector};

#[derive(Debug, Clone, Copy)]
pub enum IconColor {
    Red,
    Yellow,
}

#[derive(Debug, Clone)]
pub struct ValidatorHit {
    pub page: u32,
    pub url: String,
    pub index: String,
    pub public_key: String,
    pub color: IconColor,
}

/// Base URL and fixed params as per your console tests:
const BASE_URL: &str =
    "https://dora.explorer.mainnet.lukso.network/validators/included_deposits";
const BASE_PARAMS: &str = "f=&f.valid=1&f.orphaned=1&c=100"; // 100 rows per page

/// Column layout (0-based):
/// 0: slot
/// 1: time
/// 2: index
/// 3: depositor
/// 4: public key
/// 5: withdrawal cred
/// 6: amount
/// 7: transaction
/// 8: incl. status
/// 9: validator state (icons live here)
const INDEX_COL: usize = 2;
const PUBKEY_COL: usize = 4;

/// Build the URL for a given page.
fn page_url(page: u32) -> String {
    if page == 1 {
        format!("{BASE_URL}?{BASE_PARAMS}")
    } else {
        format!("{BASE_URL}?{BASE_PARAMS}&p={page}")
    }
}

/// Discover the last page number by looking at pagination links on page 1.
pub async fn get_last_page(client: &Client) -> Result<u32> {
    let url = page_url(1);
    println!("Discovering last page from: {url}");

    let html = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("request failed for {url}"))?
        .error_for_status()
        .with_context(|| format!("non-success status for {url}"))?
        .text()
        .await
        .with_context(|| format!("failed to read body for {url}"))?;

    let doc = Html::parse_document(&html);
    let link_sel = Selector::parse("a[href]").unwrap();

    let mut last_page = 1u32;

    for el in doc.select(&link_sel) {
        if let Some(href) = el.value().attr("href") {
            // Look for links that include this table + a page param
            if href.contains("validators/included_deposits") && href.contains("p=") {
                // crude parse: find "p=" then read digits
                if let Some(idx) = href.find("p=") {
                    let p_part = &href[idx + 2..];
                    let mut end = p_part.len();
                    for (i, ch) in p_part.char_indices() {
                        if !ch.is_ascii_digit() {
                            end = i;
                            break;
                        }
                    }
                    let num_str = &p_part[..end];
                    if let Ok(p) = num_str.parse::<u32>() {
                        if p > last_page {
                            last_page = p;
                        }
                    }
                }
            }
        }
    }

    println!("Detected last page: {last_page}");
    Ok(last_page)
}

/// Scan a single page and return all red/yellow hits on that page.
async fn scan_single_page(client: &Client, page: u32) -> Result<Vec<ValidatorHit>> {
    let url = page_url(page);
    println!("Fetching page {page}: {url}");

    let html = client
        .get(&url)
        .header(
            reqwest::header::USER_AGENT,
            "lukso-validator-checker/0.1 (github.com/your-org/your-repo)",
        )
        .send()
        .await
        .with_context(|| format!("request failed for {url}"))?
        .error_for_status()
        .with_context(|| format!("non-success status for {url}"))?
        .text()
        .await
        .with_context(|| format!("failed to read body for {url}"))?;

    let doc = Html::parse_document(&html);

    let table_row_sel = Selector::parse("table tbody tr").unwrap();
    let cell_sel = Selector::parse("td").unwrap();
    let red_icon_sel = Selector::parse(".text-danger").unwrap();
    let yellow_icon_sel = Selector::parse(".text-warning").unwrap();

    let mut page_rows = 0u32;
    let mut page_hits = 0u32;
    let mut hits = Vec::new();

    for row in doc.select(&table_row_sel) {
        let cells: Vec<_> = row.select(&cell_sel).collect();
        if cells.is_empty() {
            continue;
        }
        page_rows += 1;

        // Validator state is last cell
        let state_cell = match cells.last() {
            Some(td) => td,
            None => continue,
        };

        let state_text = state_cell.text().collect::<String>().to_lowercase();
        if !state_text.contains("active") {
            continue;
        }

        let is_red = state_cell.select(&red_icon_sel).next().is_some();
        let is_yellow = state_cell.select(&yellow_icon_sel).next().is_some();

        if !is_red && !is_yellow {
            continue;
        }

        // Extract index + public key from their columns
        let index = cells
            .get(INDEX_COL)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "UNKNOWN_INDEX".to_string());

        let public_key = cells
            .get(PUBKEY_COL)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "UNKNOWN_PUBLIC_KEY".to_string());

        if is_red {
            hits.push(ValidatorHit {
                page,
                url: url.clone(),
                index: index.clone(),
                public_key: public_key.clone(),
                color: IconColor::Red,
            });
            page_hits += 1;
        }

        if is_yellow {
            hits.push(ValidatorHit {
                page,
                url: url.clone(),
                index: index.clone(),
                public_key: public_key.clone(),
                color: IconColor::Yellow,
            });
            page_hits += 1;
        }
    }

    println!(
        "Page {page}: rows={page_rows}, hits={page_hits} (red+yellow Active)"
    );

    Ok(hits)
}

/// Scan pages [start_page, end_page] inclusive, using up to `concurrency` parallel requests.
pub async fn scan_included_deposits(
    client: &Client,
    start_page: u32,
    end_page: u32,
    concurrency: usize,
) -> Result<Vec<ValidatorHit>> {
    let mut all_hits = Vec::new();

    if end_page < start_page {
        return Ok(all_hits);
    }

    let pages: Vec<u32> = (start_page..=end_page).collect();

    for chunk in pages.chunks(concurrency) {
        // For each batch of pages, fire them in parallel.
        let futures = chunk.iter().map(|&page| {
            let client = client.clone();
            async move { scan_single_page(&client, page).await }
        });

        let results: Vec<Result<Vec<ValidatorHit>>> = join_all(futures).await;

        for res in results {
            match res {
                Ok(mut page_hits) => {
                    all_hits.append(&mut page_hits);
                }
                Err(e) => {
                    eprintln!("Error scanning a page: {e:?}");
                }
            }
        }
    }

    // Dedupe by (public_key, color) as before
    all_hits.sort_by(|a, b| {
        (a.public_key.as_str(), a.color as i32, a.page).cmp(&(
            b.public_key.as_str(),
            b.color as i32,
            b.page,
        ))
    });
    all_hits.dedup_by(|a, b| a.public_key == b.public_key && a.color as i32 == b.color as i32);

    Ok(all_hits)
}
