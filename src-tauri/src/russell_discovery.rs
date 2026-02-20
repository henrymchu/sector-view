use crate::types::DiscoveryResult;
use reqwest::Client;
use sqlx::sqlite::SqlitePool;

const IWM_CSV_URL: &str = "https://www.ishares.com/us/products/239710/ISHARES-RUSSELL-2000-ETF/1467271812596.ajax?fileType=csv&fileName=IWM_holdings&dataType=fund";

/// Split a CSV line, respecting double-quoted fields.
fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

/// Parse an iShares IWM holdings CSV into (ticker, name) tuples.
///
/// The CSV has metadata rows at the top before the column header row.
/// Only rows with `Asset Class == "Equity"` and a non-empty, non-dash ticker are returned.
pub fn parse_iwm_csv(csv: &str) -> Vec<(String, String)> {
    let mut header_found = false;
    let mut ticker_col = usize::MAX;
    let mut name_col = usize::MAX;
    let mut asset_class_col = usize::MAX;
    let mut stocks = Vec::new();

    for line in csv.lines() {
        if !header_found {
            // Find the header row by looking for both "Ticker" and "Asset Class"
            let lower = line.to_lowercase();
            if lower.contains("ticker") && lower.contains("asset class") {
                let cols = split_csv_line(line);
                for (i, col) in cols.iter().enumerate() {
                    match col.trim().to_lowercase().as_str() {
                        "ticker" => ticker_col = i,
                        "name" => name_col = i,
                        "asset class" => asset_class_col = i,
                        _ => {}
                    }
                }
                if ticker_col != usize::MAX {
                    header_found = true;
                }
            }
            continue;
        }

        let cols = split_csv_line(line);

        // Need at least enough columns to reach the required fields
        let required_max = ticker_col
            .max(if name_col != usize::MAX { name_col } else { 0 })
            .max(if asset_class_col != usize::MAX { asset_class_col } else { 0 });
        if cols.len() <= required_max {
            continue;
        }

        // Filter to equities only
        if asset_class_col != usize::MAX {
            if !cols[asset_class_col].trim().eq_ignore_ascii_case("Equity") {
                continue;
            }
        }

        let ticker = cols[ticker_col].trim().to_string();
        let name = if name_col != usize::MAX && cols.len() > name_col {
            cols[name_col].trim().to_string()
        } else {
            ticker.clone()
        };

        if !ticker.is_empty() && ticker != "-" {
            stocks.push((ticker, name));
        }
    }

    stocks
}

/// Fetch the IWM holdings CSV from iShares.
async fn fetch_iwm_csv(client: &Client) -> Result<String, String> {
    client
        .get(IWM_CSV_URL)
        .header("User-Agent", "SectorView/1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch IWM holdings: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read IWM response: {e}"))
}

/// Discover Russell 2000 stocks from iShares IWM CSV and upsert into the database.
///
/// New stocks are inserted with `sector_id = NULL` (GICS sector is not provided by IWM CSV).
/// All discovered stocks are tracked in `stock_universe` as `russell2000`.
pub async fn discover_russell_2000(pool: &SqlitePool, client: &Client) -> Result<DiscoveryResult, String> {
    let csv = fetch_iwm_csv(client).await?;
    let entries = parse_iwm_csv(&csv);

    let mut stocks_discovered: u32 = 0;
    let mut stocks_unchanged: u32 = 0;
    let errors: Vec<String> = Vec::new();

    for (ticker, name) in &entries {
        let existing: Option<(i32, Option<i32>)> =
            sqlx::query_as("SELECT id, sector_id FROM stocks WHERE symbol = ?")
                .bind(ticker)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("DB error checking {ticker}: {e}"))?;

        let stock_id = match existing {
            Some((id, _)) => {
                stocks_unchanged += 1;
                id
            }
            None => {
                let result = sqlx::query(
                    "INSERT INTO stocks (symbol, name, sector_id) VALUES (?, ?, NULL)",
                )
                .bind(ticker)
                .bind(name)
                .execute(pool)
                .await
                .map_err(|e| format!("Failed to insert {ticker}: {e}"))?;
                stocks_discovered += 1;
                result.last_insert_rowid() as i32
            }
        };

        sqlx::query(
            "INSERT OR IGNORE INTO stock_universe (stock_id, universe_type) VALUES (?, 'russell2000')",
        )
        .bind(stock_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to upsert universe for {ticker}: {e}"))?;
    }

    println!(
        "Russell 2000 discovery: {} new, {} existing, {} errors",
        stocks_discovered,
        stocks_unchanged,
        errors.len()
    );

    Ok(DiscoveryResult {
        stocks_discovered,
        stocks_updated: 0,
        stocks_unchanged,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "Name,Ticker,Asset Class,Market Value,Weight (%),Notional Value,Shares,CUSIP,ISIN,SEDOL,Price,Location,Exchange,Currency,FX Rate,Market Currency,Accrual Date";

    fn make_csv(data_rows: &[&str]) -> String {
        let mut csv = String::from("iShares Russell 2000 ETF\nAs of Feb 19, 2026\n\n");
        csv.push_str(HEADER);
        csv.push('\n');
        for row in data_rows {
            csv.push_str(row);
            csv.push('\n');
        }
        csv
    }

    // ---- parse_iwm_csv ----

    #[test]
    fn test_parse_single_equity() {
        let csv = make_csv(&["ACUTUS MEDICAL INC,AFIB,Equity,12345,0.01,12345,100,cusip,isin,sedol,1.23,US,NASDAQ,USD,1.0,USD,2026-02-19"]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "AFIB");
        assert_eq!(stocks[0].1, "ACUTUS MEDICAL INC");
    }

    #[test]
    fn test_parse_filters_non_equity() {
        let csv = make_csv(&[
            "CASH USD,USD,Cash,10000,0.5,10000,1,...",
            "SOME STOCK,TICK,Equity,100,0.01,100,10,...",
        ]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "TICK");
    }

    #[test]
    fn test_parse_filters_futures_and_other_asset_classes() {
        let csv = make_csv(&[
            "EMINI FUTURES,-,Futures,0,0.0,0,0,...",
            "REAL STOCK,REAL,Equity,100,0.01,100,10,...",
        ]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "REAL");
    }

    #[test]
    fn test_parse_skips_empty_ticker() {
        let csv = make_csv(&[
            "NO TICKER,,Equity,100,0.01,100,10,...",
            "HAS TICKER,GOOD,Equity,100,0.01,100,10,...",
        ]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "GOOD");
    }

    #[test]
    fn test_parse_skips_dash_ticker() {
        let csv = make_csv(&[
            "PLACEHOLDER,-,Equity,0,0.0,0,0,...",
            "REAL STOCK,REAL,Equity,100,0.01,100,10,...",
        ]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "REAL");
    }

    #[test]
    fn test_parse_empty_csv_returns_empty() {
        let stocks = parse_iwm_csv("");
        assert!(stocks.is_empty());
    }

    #[test]
    fn test_parse_no_header_returns_empty() {
        let csv = "iShares Russell 2000 ETF\nNo relevant header here\nSOME,DATA,ROWS";
        let stocks = parse_iwm_csv(csv);
        assert!(stocks.is_empty());
    }

    #[test]
    fn test_parse_multiple_equities() {
        let csv = make_csv(&[
            "COMPANY A,TICK1,Equity,100,0.01,100,10,...",
            "COMPANY B,TICK2,Equity,200,0.02,200,20,...",
            "COMPANY C,TICK3,Equity,300,0.03,300,30,...",
        ]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 3);
        assert_eq!(stocks[0].0, "TICK1");
        assert_eq!(stocks[1].0, "TICK2");
        assert_eq!(stocks[2].0, "TICK3");
    }

    #[test]
    fn test_parse_trims_whitespace() {
        let csv = make_csv(&["  MY COMPANY  ,  MYCO  ,  Equity  ,100,0.01,100,10,..."]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "MYCO");
        assert_eq!(stocks[0].1, "MY COMPANY");
    }

    #[test]
    fn test_parse_quoted_name_with_comma() {
        // Company names can contain commas when quoted
        let csv = make_csv(&["\"JONES LANG LASALLE, INC\",JLL,Equity,100,0.01,100,10,..."]);
        let stocks = parse_iwm_csv(&csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "JLL");
        assert_eq!(stocks[0].1, "JONES LANG LASALLE, INC");
    }

    #[test]
    fn test_parse_metadata_rows_skipped() {
        // Ensure rows before the header don't produce output
        let csv = "iShares Russell 2000 ETF\nAs of Feb 19, 2026\nFund Details here\n\nName,Ticker,Asset Class,...\nCOMPANY,ABC,Equity,...\n";
        let stocks = parse_iwm_csv(csv);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].0, "ABC");
    }

    // ---- split_csv_line ----

    #[test]
    fn test_split_csv_simple() {
        let fields = split_csv_line("a,b,c");
        assert_eq!(fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_csv_quoted_comma() {
        let fields = split_csv_line("\"a,b\",c,d");
        assert_eq!(fields, vec!["a,b", "c", "d"]);
    }

    #[test]
    fn test_split_csv_empty_field() {
        let fields = split_csv_line("a,,c");
        assert_eq!(fields, vec!["a", "", "c"]);
    }

    // ---- Performance ----

    #[test]
    fn test_parse_performance_2000_rows() {
        use std::time::Instant;

        let rows: Vec<String> = (0..2000)
            .map(|i| format!("COMPANY {i},TIC{i:04},Equity,100,0.01,100,10,..."))
            .collect();
        let row_strs: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
        let csv = make_csv(&row_strs);

        let start = Instant::now();
        let stocks = parse_iwm_csv(&csv);
        let elapsed = start.elapsed();

        assert_eq!(stocks.len(), 2000);
        assert!(
            elapsed.as_millis() < 500,
            "Parsing 2000 rows took {}ms, expected < 500ms",
            elapsed.as_millis()
        );
    }
}
