use anyhow::{Context, Result};
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

/// Scan pages [start_page, start_page + max_pages - 1].
/// If `stop_when_empty` is true, it stops early when a page has no data rows.
pub async fn scan_included_deposits(
    client: &Client,
    start_page: u32,
    max_pages: u32,
    stop_when_empty: bool,
) -> Result<Vec<ValidatorHit>> {
    let mut hits = Vec::new();

    // Prebuild selectors
    let table_row_sel = Selector::parse("table tbody tr").unwrap();
    let cell_sel = Selector::parse("td").unwrap();
    let red_icon_sel = Selector::parse(".text-danger").unwrap();
    let yellow_icon_sel = Selector::parse(".text-warning").unwrap();

    for i in 0..max_pages {
        let page = start_page + i;

        let url = if page == 1 {
            format!("{BASE_URL}?{BASE_PARAMS}")
        } else {
            format!("{BASE_URL}?{BASE_PARAMS}&p={page}")
        };

        // You may want to add some logging here
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

        let mut page_rows = 0u32;
        let mut page_hits = 0u32;

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

        if stop_when_empty && page_rows == 0 {
            println!("No table rows on page {page}; stopping early.");
            break;
        }
    }

    // Optional: dedupe by (public_key, color) or (index, color)
    hits.sort_by(|a, b| {
        (a.public_key.as_str(), a.color as i32, a.page).cmp(&(
            b.public_key.as_str(),
            b.color as i32,
            b.page,
        ))
    });
    hits.dedup_by(|a, b| a.public_key == b.public_key && a.color as i32 == b.color as i32);

    Ok(hits)
}
