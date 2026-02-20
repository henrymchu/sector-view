use crate::cache::SectorCache;
use crate::market_data;
use crate::outlier_detection;
use crate::russell_discovery;
use crate::stock_discovery;
use crate::types::{OutlierStock, RefreshResult, Sector, SectorOutliers, SectorSummary, Stock};
use crate::DbState;
use reqwest::Client;
use serde::Serialize;
use tauri::{Emitter, State};

#[derive(Clone, Serialize)]
struct ProgressPayload {
    current: u32,
    total: u32,
    phase: String,
}

#[tauri::command]
pub async fn get_sectors(db: State<'_, DbState>) -> Result<Vec<Sector>, String> {
    sqlx::query_as::<_, Sector>("SELECT id, name, symbol FROM sectors ORDER BY name")
        .fetch_all(&db.0)
        .await
        .map_err(|e| format!("Failed to fetch sectors: {e}"))
}

#[tauri::command]
pub async fn get_stocks_by_sector(
    sector_id: i32,
    db: State<'_, DbState>,
) -> Result<Vec<Stock>, String> {
    sqlx::query_as::<_, Stock>(
        "SELECT id, symbol, name, sector_id FROM stocks WHERE sector_id = ? ORDER BY symbol",
    )
    .bind(sector_id)
    .fetch_all(&db.0)
    .await
    .map_err(|e| format!("Failed to fetch stocks: {e}"))
}

#[tauri::command]
pub async fn get_sector_performance(
    universe: Option<String>,
    db: State<'_, DbState>,
    cache: State<'_, SectorCache>,
) -> Result<Vec<SectorSummary>, String> {
    let universe_str = universe.as_deref().unwrap_or("sp500");

    // Use cache only for the default sp500 universe
    if universe_str == "sp500" {
        if let Some(cached) = cache.get() {
            return Ok(cached);
        }
    }

    let summaries = query_sector_summaries(&db.0, universe_str).await?;

    if universe_str == "sp500" && !summaries.is_empty() {
        cache.set(summaries.clone());
    }

    Ok(summaries)
}

#[tauri::command]
pub async fn refresh_market_data(
    app: tauri::AppHandle,
    db: State<'_, DbState>,
    cache: State<'_, SectorCache>,
) -> Result<RefreshResult, String> {
    let client = Client::new();

    // Step 1: Stock discovery (non-fatal — if it fails, continue with existing stocks)
    let _ = app.emit("refresh-progress", ProgressPayload {
        current: 0,
        total: 0,
        phase: "discovery".to_string(),
    });

    let discovery = match stock_discovery::discover_stocks(&db.0, &client).await {
        Ok(result) => Some(result),
        Err(e) => {
            eprintln!("Stock discovery failed (non-fatal): {e}");
            None
        }
    };

    // Step 2: Authenticate with Yahoo Finance for fundamentals data
    let session = market_data::YahooSession::new().await
        .map_err(|e| format!("Yahoo Finance auth failed: {e}"))?;

    // Step 3: Fetch market data for ALL stocks (including any newly discovered)
    let stocks = sqlx::query_as::<_, Stock>(
        "SELECT id, symbol, name, sector_id FROM stocks WHERE sector_id IS NOT NULL ORDER BY symbol",
    )
    .fetch_all(&db.0)
    .await
    .map_err(|e| format!("Failed to fetch stocks: {e}"))?;

    let total = stocks.len() as u32;
    let mut success_count = 0;
    let mut error_count = 0;

    for (i, stock) in stocks.iter().enumerate() {
        let _ = app.emit("refresh-progress", ProgressPayload {
            current: (i + 1) as u32,
            total,
            phase: "market-data".to_string(),
        });

        match market_data::fetch_stock_quote(&client, &session, stock.id, &stock.symbol).await {
            Ok(quote) => {
                if let Err(e) = market_data::save_quote(&db.0, &quote).await {
                    eprintln!("Failed to save {}: {e}", stock.symbol);
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch {}: {e}", stock.symbol);
                error_count += 1;
            }
        }

        // Small delay to respect rate limits
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("Refresh complete: {success_count} succeeded, {error_count} failed");

    // Recalculate sector summaries from fresh data
    let summaries = query_sector_summaries(&db.0, "sp500").await?;
    cache.set(summaries.clone());

    Ok(RefreshResult {
        sectors: summaries,
        discovery,
    })
}

#[tauri::command]
pub async fn refresh_sector_data(
    app: tauri::AppHandle,
    sector_symbol: String,
    db: State<'_, DbState>,
    cache: State<'_, SectorCache>,
) -> Result<Vec<SectorSummary>, String> {
    let client = Client::new();
    let session = market_data::YahooSession::new().await
        .map_err(|e| format!("Yahoo Finance auth failed: {e}"))?;

    // Get stocks for this sector only
    let stocks = sqlx::query_as::<_, Stock>(
        "SELECT s.id, s.symbol, s.name, s.sector_id FROM stocks s
         JOIN sectors sec ON s.sector_id = sec.id
         WHERE sec.symbol = ?
         ORDER BY s.symbol",
    )
    .bind(&sector_symbol)
    .fetch_all(&db.0)
    .await
    .map_err(|e| format!("Failed to fetch stocks: {e}"))?;

    let total = stocks.len() as u32;
    let mut success_count = 0;

    for (i, stock) in stocks.iter().enumerate() {
        let _ = app.emit("refresh-progress", ProgressPayload {
            current: (i + 1) as u32,
            total,
            phase: "market-data".to_string(),
        });

        match market_data::fetch_stock_quote(&client, &session, stock.id, &stock.symbol).await {
            Ok(quote) => {
                if market_data::save_quote(&db.0, &quote).await.is_ok() {
                    success_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch {}: {e}", stock.symbol);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("Sector refresh ({sector_symbol}): {success_count}/{} succeeded", stocks.len());

    let summaries = query_sector_summaries(&db.0, "sp500").await?;
    cache.set(summaries.clone());

    Ok(summaries)
}

/// Query sector summaries from the latest market_data entries, filtered by universe.
async fn query_sector_summaries(
    pool: &sqlx::sqlite::SqlitePool,
    universe: &str,
) -> Result<Vec<SectorSummary>, String> {
    let rows: Vec<SectorSummaryRow> = sqlx::query_as(
        "SELECT
            sec.id as sector_id,
            sec.name,
            sec.symbol,
            COALESCE(AVG(md.price_change_percent), 0.0) as avg_change_percent,
            AVG(md.pe_ratio) as avg_pe_ratio,
            SUM(md.market_cap) as total_market_cap,
            COUNT(DISTINCT s.id) as stock_count,
            AVG(md.beta) as avg_beta
        FROM sectors sec
        LEFT JOIN stocks s ON s.sector_id = sec.id
            AND s.id IN (
                SELECT stock_id FROM stock_universe
                WHERE universe_type = ? AND date_removed IS NULL
            )
        LEFT JOIN market_data md ON md.stock_id = s.id
            AND md.id = (
                SELECT md2.id FROM market_data md2
                WHERE md2.stock_id = s.id
                ORDER BY md2.timestamp DESC LIMIT 1
            )
        GROUP BY sec.id
        ORDER BY sec.name",
    )
    .bind(universe)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to query sector summaries: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| SectorSummary {
            sector_id: r.sector_id,
            name: r.name,
            symbol: r.symbol,
            avg_change_percent: r.avg_change_percent,
            avg_pe_ratio: r.avg_pe_ratio,
            total_market_cap: r.total_market_cap,
            stock_count: r.stock_count,
            avg_beta: r.avg_beta,
        })
        .collect())
}

#[derive(Debug, sqlx::FromRow)]
struct SectorSummaryRow {
    sector_id: i32,
    name: String,
    symbol: String,
    avg_change_percent: f64,
    avg_pe_ratio: Option<f64>,
    total_market_cap: Option<i64>,
    stock_count: i32,
    avg_beta: Option<f64>,
}

// -- Outlier Detection Commands --

#[tauri::command]
pub async fn detect_outliers(
    threshold: Option<f64>,
    universe: Option<String>,
    db: State<'_, DbState>,
) -> Result<Vec<SectorOutliers>, String> {
    let universe_str = universe.as_deref().unwrap_or("sp500");
    let default_threshold = if universe_str == "russell2000" { 2.0 } else { 1.5 };
    let threshold = threshold.unwrap_or(default_threshold);
    outlier_detection::detect_all_outliers(&db.0, threshold, universe_str).await
}

#[tauri::command]
pub async fn get_sector_outliers(
    sector_id: i32,
    threshold: Option<f64>,
    universe: Option<String>,
    db: State<'_, DbState>,
) -> Result<Vec<OutlierStock>, String> {
    let universe_str = universe.as_deref().unwrap_or("sp500");
    let default_threshold = if universe_str == "russell2000" { 2.0 } else { 1.5 };
    let threshold = threshold.unwrap_or(default_threshold);
    outlier_detection::detect_sector_outliers(&db.0, sector_id, threshold, universe_str).await
}

/// Map a Yahoo Finance sector name to the matching DB sector name.
/// Yahoo Finance uses different labels than GICS (e.g. "Healthcare" vs "Health Care").
fn map_yahoo_sector_to_db(yahoo_sector: &str) -> Option<&'static str> {
    match yahoo_sector {
        "Technology" => Some("Technology"),
        "Healthcare" => Some("Health Care"),
        "Financial Services" => Some("Financials"),
        "Consumer Cyclical" => Some("Consumer Discretionary"),
        "Communication Services" => Some("Communication Services"),
        "Industrials" => Some("Industrials"),
        "Consumer Defensive" => Some("Consumer Staples"),
        "Energy" => Some("Energy"),
        "Utilities" => Some("Utilities"),
        "Real Estate" => Some("Real Estate"),
        "Basic Materials" => Some("Materials"),
        _ => None,
    }
}

// -- Russell 2000 Universe Command --

#[tauri::command]
pub async fn refresh_russell_2000_data(
    app: tauri::AppHandle,
    db: State<'_, DbState>,
) -> Result<RefreshResult, String> {
    let client = Client::new();

    // Step 1: Discover Russell 2000 stocks from iShares IWM CSV
    let _ = app.emit("refresh-progress", ProgressPayload {
        current: 0,
        total: 0,
        phase: "discovery".to_string(),
    });

    let discovery = match russell_discovery::discover_russell_2000(&db.0, &client).await {
        Ok(result) => Some(result),
        Err(e) => {
            eprintln!("Russell 2000 discovery failed (non-fatal): {e}");
            None
        }
    };

    // Step 2: Authenticate with Yahoo Finance
    let session = market_data::YahooSession::new()
        .await
        .map_err(|e| format!("Yahoo Finance auth failed: {e}"))?;

    // Step 3: Build sector name → id map for assigning sectors to unclassified stocks
    let sector_rows: Vec<(i32, String)> = sqlx::query_as("SELECT id, name FROM sectors")
        .fetch_all(&db.0)
        .await
        .map_err(|e| format!("Failed to fetch sectors: {e}"))?;
    let sector_map: std::collections::HashMap<String, i32> =
        sector_rows.into_iter().map(|(id, name)| (name, id)).collect();

    // Step 4: Fetch market data for all Russell 2000 stocks
    let stocks: Vec<Stock> = sqlx::query_as(
        "SELECT s.id, s.symbol, s.name, s.sector_id
         FROM stocks s
         JOIN stock_universe su ON su.stock_id = s.id
         WHERE su.universe_type = 'russell2000' AND su.date_removed IS NULL
         ORDER BY s.symbol",
    )
    .fetch_all(&db.0)
    .await
    .map_err(|e| format!("Failed to fetch Russell 2000 stocks: {e}"))?;

    let total = stocks.len() as u32;
    let mut success_count = 0;
    let mut error_count = 0;

    for (i, stock) in stocks.iter().enumerate() {
        let _ = app.emit("refresh-progress", ProgressPayload {
            current: (i + 1) as u32,
            total,
            phase: "market-data".to_string(),
        });

        match market_data::fetch_stock_quote(&client, &session, stock.id, &stock.symbol).await {
            Ok(quote) => {
                // Assign sector_id from Yahoo Finance data for unclassified stocks
                if stock.sector_id.is_none() {
                    if let Some(ref yahoo_sector) = quote.yahoo_sector {
                        if let Some(db_name) = map_yahoo_sector_to_db(yahoo_sector) {
                            if let Some(&sector_id) = sector_map.get(db_name) {
                                let _ = sqlx::query(
                                    "UPDATE stocks SET sector_id = ? WHERE id = ? AND sector_id IS NULL",
                                )
                                .bind(sector_id)
                                .bind(stock.id)
                                .execute(&db.0)
                                .await;
                            }
                        }
                    }
                }

                if let Err(e) = market_data::save_quote(&db.0, &quote).await {
                    eprintln!("Failed to save {}: {e}", stock.symbol);
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch {}: {e}", stock.symbol);
                error_count += 1;
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("Russell 2000 refresh: {success_count} succeeded, {error_count} failed");

    let summaries = query_sector_summaries(&db.0, "russell2000").await?;

    Ok(RefreshResult {
        sectors: summaries,
        discovery,
    })
}
