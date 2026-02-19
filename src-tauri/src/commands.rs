use crate::cache::SectorCache;
use crate::market_data;
use crate::types::{Sector, SectorSummary, Stock};
use crate::DbState;
use reqwest::Client;
use tauri::State;

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
    db: State<'_, DbState>,
    cache: State<'_, SectorCache>,
) -> Result<Vec<SectorSummary>, String> {
    // Try cache first
    if let Some(cached) = cache.get() {
        return Ok(cached);
    }

    // Query from database (latest market_data per stock)
    let summaries = query_sector_summaries(&db.0).await?;

    if !summaries.is_empty() {
        cache.set(summaries.clone());
    }

    Ok(summaries)
}

#[tauri::command]
pub async fn refresh_market_data(
    db: State<'_, DbState>,
    cache: State<'_, SectorCache>,
) -> Result<Vec<SectorSummary>, String> {
    let client = Client::new();

    // Get all stocks with sector assignments
    let stocks = sqlx::query_as::<_, Stock>(
        "SELECT id, symbol, name, sector_id FROM stocks WHERE sector_id IS NOT NULL ORDER BY symbol",
    )
    .fetch_all(&db.0)
    .await
    .map_err(|e| format!("Failed to fetch stocks: {e}"))?;

    let mut success_count = 0;
    let mut error_count = 0;

    for stock in &stocks {
        match market_data::fetch_stock_quote(&client, stock.id, &stock.symbol).await {
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
    let summaries = query_sector_summaries(&db.0).await?;
    cache.set(summaries.clone());

    Ok(summaries)
}

#[tauri::command]
pub async fn refresh_sector_data(
    sector_symbol: String,
    db: State<'_, DbState>,
    cache: State<'_, SectorCache>,
) -> Result<Vec<SectorSummary>, String> {
    let client = Client::new();

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

    let mut success_count = 0;

    for stock in &stocks {
        match market_data::fetch_stock_quote(&client, stock.id, &stock.symbol).await {
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

    let summaries = query_sector_summaries(&db.0).await?;
    cache.set(summaries.clone());

    Ok(summaries)
}

/// Query sector summaries from the latest market_data entries.
async fn query_sector_summaries(
    pool: &sqlx::sqlite::SqlitePool,
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
        LEFT JOIN market_data md ON md.stock_id = s.id
            AND md.id = (
                SELECT md2.id FROM market_data md2
                WHERE md2.stock_id = s.id
                ORDER BY md2.timestamp DESC LIMIT 1
            )
        GROUP BY sec.id
        ORDER BY sec.name",
    )
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
