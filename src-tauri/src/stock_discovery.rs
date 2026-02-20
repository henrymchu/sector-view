use crate::types::DiscoveryResult;
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

#[derive(Debug)]
struct WikiStock {
    symbol: String,
    name: String,
    gics_sector: String,
}

/// Map known Wikipedia GICS sector name variants to internal DB sector names.
/// Returns the unchanged name if no alias is defined.
fn apply_wikipedia_name_alias(name: &str) -> &str {
    match name {
        "Information Technology" => "Technology",
        _ => name,
    }
}

/// Parse an S&P 500 Wikipedia HTML page into a list of WikiStock entries.
fn parse_sp500_html(html: &str) -> Result<Vec<WikiStock>, String> {
    let document = Html::parse_document(html);
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

        // Column 0: Symbol (inside <a> tag on Wikipedia)
        let symbol = cells[0]
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>())
            .unwrap_or_else(|| cells[0].text().collect::<String>())
            .trim()
            .to_string();

        // Column 1: Security name (inside <a> tag on Wikipedia)
        let name = cells[1]
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>())
            .unwrap_or_else(|| cells[1].text().collect::<String>())
            .trim()
            .to_string();

        // Column 2: GICS Sector
        let gics_sector = cells[2].text().collect::<String>().trim().to_string();

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

    parse_sp500_html(&html)
}

/// Build a mapping from DB sector names to sector IDs.
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
        // Translate Wikipedia GICS name to internal DB sector name before lookup
        let canonical_sector = apply_wikipedia_name_alias(&ws.gics_sector);
        let sector_id = match sector_map.get(canonical_sector) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- HTML test helpers ----

    /// Build a minimal Wikipedia-style S&P 500 HTML page.
    /// Each row tuple is (symbol, name, gics_sector).
    fn make_wiki_html(rows: &[(&str, &str, &str)]) -> String {
        let mut html = String::from(
            r#"<table class="wikitable sortable"><tbody>"#,
        );
        // Header row (skipped by parser)
        html.push_str(
            r#"<tr><th>Symbol</th><th>Security</th><th>GICS Sector</th><th>GICS Sub-Industry</th></tr>"#,
        );
        for (symbol, name, sector) in rows {
            html.push_str(&format!(
                r#"<tr><td><a href="/wiki/{symbol}">{symbol}</a></td><td><a href="/wiki/{name}">{name}</a></td><td>{sector}</td><td>Sub</td></tr>"#,
            ));
        }
        html.push_str("</tbody></table>");
        html
    }

    /// Build a row where symbol/name are plain text (no <a> tag).
    fn make_plain_row(symbol: &str, name: &str, sector: &str) -> String {
        format!(
            r#"<tr><td>{symbol}</td><td>{name}</td><td>{sector}</td><td>Sub</td></tr>"#
        )
    }

    // ---- apply_wikipedia_name_alias ----

    #[test]
    fn test_alias_information_technology_maps_to_technology() {
        assert_eq!(apply_wikipedia_name_alias("Information Technology"), "Technology");
    }

    #[test]
    fn test_alias_known_sectors_pass_through_unchanged() {
        for sector in &[
            "Health Care",
            "Financials",
            "Consumer Discretionary",
            "Communication Services",
            "Industrials",
            "Consumer Staples",
            "Energy",
            "Utilities",
            "Real Estate",
            "Materials",
            "Technology",
        ] {
            assert_eq!(
                apply_wikipedia_name_alias(sector),
                *sector,
                "Sector '{sector}' should pass through unchanged"
            );
        }
    }

    #[test]
    fn test_alias_unknown_name_returns_unchanged() {
        assert_eq!(apply_wikipedia_name_alias("Foo Bar"), "Foo Bar");
        assert_eq!(apply_wikipedia_name_alias(""), "");
    }

    // ---- parse_sp500_html ----

    #[test]
    fn test_parse_single_stock() {
        let html = make_wiki_html(&[("AAPL", "Apple Inc.", "Information Technology")]);
        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].symbol, "AAPL");
        assert_eq!(stocks[0].name, "Apple Inc.");
        assert_eq!(stocks[0].gics_sector, "Information Technology");
    }

    #[test]
    fn test_parse_multiple_stocks() {
        let html = make_wiki_html(&[
            ("AAPL", "Apple Inc.", "Information Technology"),
            ("JPM", "JPMorgan Chase", "Financials"),
            ("XOM", "Exxon Mobil", "Energy"),
        ]);
        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 3);
        assert_eq!(stocks[0].symbol, "AAPL");
        assert_eq!(stocks[1].symbol, "JPM");
        assert_eq!(stocks[2].symbol, "XOM");
    }

    #[test]
    fn test_parse_symbol_in_anchor_tag() {
        // Wikipedia wraps the ticker in an <a> tag — verify it's extracted from the link text
        let html = make_wiki_html(&[("MSFT", "Microsoft Corporation", "Information Technology")]);
        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks[0].symbol, "MSFT");
    }

    #[test]
    fn test_parse_symbol_without_anchor_tag() {
        // Falls back to plain cell text when no <a> is present
        let mut html = String::from(r#"<table class="wikitable sortable"><tbody>"#);
        html.push_str("<tr><th>Symbol</th><th>Security</th><th>GICS Sector</th><th>Sub</th></tr>");
        html.push_str(&make_plain_row("GOOG", "Alphabet Inc.", "Communication Services"));
        html.push_str("</tbody></table>");

        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].symbol, "GOOG");
        assert_eq!(stocks[0].name, "Alphabet Inc.");
    }

    #[test]
    fn test_parse_whitespace_is_trimmed() {
        let mut html = String::from(r#"<table class="wikitable sortable"><tbody>"#);
        html.push_str("<tr><th>Symbol</th><th>Security</th><th>GICS Sector</th><th>Sub</th></tr>");
        // Plain text cells with surrounding whitespace
        html.push_str(
            "<tr><td>  AMZN  </td><td>  Amazon.com Inc.  </td><td>  Consumer Discretionary  </td><td>Sub</td></tr>",
        );
        html.push_str("</tbody></table>");

        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks[0].symbol, "AMZN");
        assert_eq!(stocks[0].name, "Amazon.com Inc.");
        assert_eq!(stocks[0].gics_sector, "Consumer Discretionary");
    }

    #[test]
    fn test_parse_skips_header_row() {
        // The header <tr> contains <th> not <td>, so the td selector yields 0 cells → skipped
        let html = make_wiki_html(&[("WMT", "Walmart Inc.", "Consumer Staples")]);
        let stocks = parse_sp500_html(&html).unwrap();
        // Only 1 stock row, not 2 (header not included)
        assert_eq!(stocks.len(), 1);
    }

    #[test]
    fn test_parse_skips_rows_with_too_few_cells() {
        let mut html = String::from(r#"<table class="wikitable sortable"><tbody>"#);
        html.push_str("<tr><th>Symbol</th><th>Security</th><th>GICS Sector</th><th>Sub</th></tr>");
        // Only 2 cells — should be skipped
        html.push_str("<tr><td>BAD</td><td>Bad Row</td></tr>");
        // Valid row
        html.push_str(&make_plain_row("JPM", "JPMorgan Chase", "Financials"));
        html.push_str("</tbody></table>");

        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].symbol, "JPM");
    }

    #[test]
    fn test_parse_skips_row_with_empty_symbol() {
        let mut html = String::from(r#"<table class="wikitable sortable"><tbody>"#);
        html.push_str("<tr><th>Symbol</th><th>Security</th><th>GICS Sector</th><th>Sub</th></tr>");
        // Empty symbol cell
        html.push_str("<tr><td></td><td>No Symbol Co.</td><td>Financials</td><td>Sub</td></tr>");
        html.push_str(&make_plain_row("JPM", "JPMorgan Chase", "Financials"));
        html.push_str("</tbody></table>");

        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].symbol, "JPM");
    }

    #[test]
    fn test_parse_skips_row_with_empty_sector() {
        let mut html = String::from(r#"<table class="wikitable sortable"><tbody>"#);
        html.push_str("<tr><th>Symbol</th><th>Security</th><th>GICS Sector</th><th>Sub</th></tr>");
        // Empty sector cell
        html.push_str("<tr><td>XYZ</td><td>XYZ Corp</td><td></td><td>Sub</td></tr>");
        html.push_str(&make_plain_row("JPM", "JPMorgan Chase", "Financials"));
        html.push_str("</tbody></table>");

        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].symbol, "JPM");
    }

    #[test]
    fn test_parse_empty_table_returns_no_stocks() {
        // Table with only a header row and no data rows
        let html = make_wiki_html(&[]);
        let stocks = parse_sp500_html(&html).unwrap();
        assert!(stocks.is_empty());
    }

    #[test]
    fn test_parse_no_wikitable_returns_error() {
        // HTML with no table.wikitable.sortable
        let html = "<html><body><table><tr><td>No class</td></tr></table></body></html>";
        let result = parse_sp500_html(html);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Could not find S&P 500 table"));
    }

    #[test]
    fn test_parse_malformed_html_returns_error() {
        // Completely empty / non-HTML string
        let result = parse_sp500_html("not html at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_all_eleven_sectors() {
        // Verify all 11 GICS sectors parse without issue
        let rows = vec![
            ("AAPL", "Apple Inc.", "Information Technology"),
            ("JNJ", "Johnson & Johnson", "Health Care"),
            ("JPM", "JPMorgan Chase", "Financials"),
            ("AMZN", "Amazon.com Inc.", "Consumer Discretionary"),
            ("GOOGL", "Alphabet Inc.", "Communication Services"),
            ("HON", "Honeywell", "Industrials"),
            ("PG", "Procter & Gamble", "Consumer Staples"),
            ("XOM", "Exxon Mobil", "Energy"),
            ("NEE", "NextEra Energy", "Utilities"),
            ("PLD", "Prologis", "Real Estate"),
            ("LIN", "Linde plc", "Materials"),
        ];
        let html = make_wiki_html(&rows);
        let stocks = parse_sp500_html(&html).unwrap();
        assert_eq!(stocks.len(), 11);

        let sectors: Vec<&str> = stocks.iter().map(|s| s.gics_sector.as_str()).collect();
        assert!(sectors.contains(&"Information Technology"));
        assert!(sectors.contains(&"Health Care"));
        assert!(sectors.contains(&"Financials"));
    }

    // ---- alias + parse integration ----

    #[test]
    fn test_alias_applied_after_parsing_maps_it_to_technology() {
        // Simulate what discover_stocks does: parse HTML then apply alias during lookup
        let html = make_wiki_html(&[("AAPL", "Apple Inc.", "Information Technology")]);
        let stocks = parse_sp500_html(&html).unwrap();

        let canonical = apply_wikipedia_name_alias(&stocks[0].gics_sector);
        assert_eq!(canonical, "Technology");
    }

    #[test]
    fn test_alias_applied_after_parsing_leaves_other_sectors_unchanged() {
        let html = make_wiki_html(&[("JPM", "JPMorgan Chase", "Financials")]);
        let stocks = parse_sp500_html(&html).unwrap();

        let canonical = apply_wikipedia_name_alias(&stocks[0].gics_sector);
        assert_eq!(canonical, "Financials");
    }

    // ---- Performance ----

    #[test]
    fn test_parse_performance_500_rows() {
        use std::time::Instant;

        let rows: Vec<(&str, &str, &str)> = (0..500)
            .map(|_| ("AAPL", "Apple Inc.", "Information Technology"))
            .collect();
        let html = make_wiki_html(&rows);

        let start = Instant::now();
        let stocks = parse_sp500_html(&html).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(stocks.len(), 500);
        assert!(
            elapsed.as_millis() < 500,
            "Parsing 500 rows took {}ms, expected < 500ms",
            elapsed.as_millis()
        );
    }
}
