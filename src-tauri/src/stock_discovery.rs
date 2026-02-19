use crate::types::DiscoveryResult;
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

struct WikiStock {
    symbol: String,
    name: String,
    gics_sector: String,
}

/// Fetch S&P 500 stock list from Wikipedia and parse HTML table.
async fn fetch_sp500_from_wikipedia(client: &Client) -> Result<Vec<WikiStock>, String> {
    let url = "https://en.wikipedia.org/wiki/List_of_S%26P_500_companies";
    let html = client
        .get(url)
        .header("User-Agent", "SectorView/1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Wikipedia: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read Wikipedia response: {e}"))?;

    let document = Html::parse_document(&html);
    let table_sel = Selector::parse("table.wikitable.sortable").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();

    let table = document
        .select(&table_sel)
        .next()
        .ok_or_else(|| "Could not find S&P 500 table on Wikipedia".to_string())?;

    let mut stocks = Vec::new();

    for row in table.select(&tr_sel).skip(1) {
        let cells: Vec<_> = row.select(&td_sel).collect();
        if cells.len() < 4 {
            continue;
        }

        // Column 0: Symbol (inside <a> tag)
        let symbol = cells[0]
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>())
            .unwrap_or_else(|| cells[0].text().collect::<String>())
            .trim()
            .to_string();

        // Column 1: Security name (inside <a> tag)
        let name = cells[1]
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>())
            .unwrap_or_else(|| cells[1].text().collect::<String>())
            .trim()
            .to_string();

        // Column 3: GICS Sector
        let gics_sector = cells[3].text().collect::<String>().trim().to_string();

        if !symbol.is_empty() && !gics_sector.is_empty() {
            stocks.push(WikiStock {
                symbol,
                name,
                gics_sector,
            });
        }
    }

    Ok(stocks)
}

/// Build a mapping from Wikipedia GICS sector names to our sector IDs.
async fn build_sector_map(pool: &SqlitePool) -> Result<HashMap<String, i32>, String> {
    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM sectors")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Failed to query sectors: {e}"))?;

    let mut map = HashMap::new();
    for (id, name) in &rows {
        map.insert(name.clone(), *id);
    }

    // Wikipedia uses "Information Technology" but our DB uses "Technology"
    if let Some(&tech_id) = map.get("Technology") {
        map.insert("Information Technology".to_string(), tech_id);
    }

    Ok(map)
}

/// Discover S&P 500 stocks from Wikipedia and upsert into the database.
pub async fn discover_stocks(pool: &SqlitePool, client: &Client) -> Result<DiscoveryResult, String> {
    let wiki_stocks = fetch_sp500_from_wikipedia(client).await?;
    let sector_map = build_sector_map(pool).await?;

    let mut stocks_discovered: u32 = 0;
    let mut stocks_updated: u32 = 0;
    let mut stocks_unchanged: u32 = 0;
    let mut errors: Vec<String> = Vec::new();

    for ws in &wiki_stocks {
        let sector_id = match sector_map.get(&ws.gics_sector) {
            Some(&id) => id,
            None => {
                errors.push(format!("Unknown sector '{}' for {}", ws.gics_sector, ws.symbol));
                continue;
            }
        };

        // Check if stock already exists
        let existing: Option<(i32, Option<i32>)> = sqlx::query_as(
            "SELECT id, sector_id FROM stocks WHERE symbol = ?",
        )
        .bind(&ws.symbol)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("DB error checking {}: {e}", ws.symbol))?;

        match existing {
            Some((_id, current_sector_id)) => {
                if current_sector_id != Some(sector_id) {
                    // Sector changed — update
                    sqlx::query("UPDATE stocks SET sector_id = ?, name = ? WHERE symbol = ?")
                        .bind(sector_id)
                        .bind(&ws.name)
                        .bind(&ws.symbol)
                        .execute(pool)
                        .await
                        .map_err(|e| format!("Failed to update {}: {e}", ws.symbol))?;
                    stocks_updated += 1;
                } else {
                    stocks_unchanged += 1;
                }
            }
            None => {
                // New stock — insert
                sqlx::query("INSERT INTO stocks (symbol, name, sector_id) VALUES (?, ?, ?)")
                    .bind(&ws.symbol)
                    .bind(&ws.name)
                    .bind(sector_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Failed to insert {}: {e}", ws.symbol))?;
                stocks_discovered += 1;
            }
        }
    }

    println!(
        "Discovery complete: {} new, {} updated, {} unchanged, {} errors",
        stocks_discovered, stocks_updated, stocks_unchanged, errors.len()
    );

    Ok(DiscoveryResult {
        stocks_discovered,
        stocks_updated,
        stocks_unchanged,
        errors,
    })
}
