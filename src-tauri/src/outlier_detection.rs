use crate::types::{OutlierStock, OutlierType, SectorOutliers, SignificanceLevel, ZScores};
use sqlx::sqlite::SqlitePool;

/// Raw market data for a single stock (latest entry).
#[derive(Debug, sqlx::FromRow)]
struct StockMarketRow {
    stock_id: i32,
    symbol: String,
    name: String,
    sector_id: i32,
    price_change_percent: f64,
    pe_ratio: Option<f64>,
    pb_ratio: Option<f64>,
    volume: Option<i64>,
    avg_volume_10d: Option<i64>,
}

/// Sector-level statistics for Z-score calculation.
struct SectorStats {
    pe_mean: Option<f64>,
    pe_std: Option<f64>,
    pb_mean: Option<f64>,
    pb_std: Option<f64>,
    price_mean: f64,
    price_std: f64,
    vol_ratio_mean: Option<f64>,
    vol_ratio_std: Option<f64>,
}

/// Detect outliers across all sectors.
pub async fn detect_all_outliers(
    pool: &SqlitePool,
    threshold: f64,
) -> Result<Vec<SectorOutliers>, String> {
    // Get all sectors
    let sectors: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT id, name, symbol FROM sectors ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch sectors: {e}"))?;

    let mut results = Vec::new();

    for (sector_id, sector_name, sector_symbol) in &sectors {
        let outliers = detect_sector_outliers(pool, *sector_id, threshold).await?;
        results.push(SectorOutliers {
            sector_id: *sector_id,
            sector_name: sector_name.clone(),
            sector_symbol: sector_symbol.clone(),
            outlier_count: outliers.len(),
            outliers,
        });
    }

    Ok(results)
}

/// Detect outliers within a single sector.
pub async fn detect_sector_outliers(
    pool: &SqlitePool,
    sector_id: i32,
    threshold: f64,
) -> Result<Vec<OutlierStock>, String> {
    // Get latest market data for all stocks in this sector
    let rows: Vec<StockMarketRow> = sqlx::query_as(
        "SELECT s.id as stock_id, s.symbol, s.name, s.sector_id,
                md.price_change_percent,
                md.pe_ratio, md.pb_ratio,
                md.volume, md.avg_volume_10d
         FROM stocks s
         JOIN market_data md ON md.stock_id = s.id
            AND md.id = (
                SELECT md2.id FROM market_data md2
                WHERE md2.stock_id = s.id
                ORDER BY md2.timestamp DESC LIMIT 1
            )
         WHERE s.sector_id = ?",
    )
    .bind(sector_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch sector market data: {e}"))?;

    if rows.len() < 3 {
        // Not enough data for meaningful statistics
        return Ok(Vec::new());
    }

    let stats = calculate_stats(&rows);
    let mut outliers = Vec::new();

    for row in &rows {
        let z_scores = calculate_z_scores(row, &stats);
        let composite = calculate_composite_score(&z_scores);

        if composite >= threshold {
            let outlier_type = classify_outlier(&z_scores);
            let significance = classify_significance(composite);

            outliers.push(OutlierStock {
                stock_id: row.stock_id,
                symbol: row.symbol.clone(),
                name: row.name.clone(),
                z_scores,
                composite_score: (composite * 100.0).round() / 100.0,
                outlier_type,
                significance_level: significance,
            });
        }
    }

    // Sort by composite score descending (strongest outliers first)
    outliers.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));

    // Save detections to database
    for outlier in &outliers {
        save_detection(pool, outlier, sector_id, threshold).await.ok();
    }

    Ok(outliers)
}

/// Calculate sector statistics (mean and std dev for each metric).
fn calculate_stats(rows: &[StockMarketRow]) -> SectorStats {
    // Price change
    let prices: Vec<f64> = rows.iter().map(|r| r.price_change_percent).collect();
    let (price_mean, price_std) = mean_std(&prices);

    // P/E ratio (skip nulls)
    let pes: Vec<f64> = rows.iter().filter_map(|r| r.pe_ratio).collect();
    let (pe_mean, pe_std) = if pes.len() >= 2 {
        let (m, s) = mean_std(&pes);
        (Some(m), Some(s))
    } else {
        (None, None)
    };

    // P/B ratio (skip nulls)
    let pbs: Vec<f64> = rows.iter().filter_map(|r| r.pb_ratio).collect();
    let (pb_mean, pb_std) = if pbs.len() >= 2 {
        let (m, s) = mean_std(&pbs);
        (Some(m), Some(s))
    } else {
        (None, None)
    };

    // Volume ratio (volume / avg_volume_10d)
    let vol_ratios: Vec<f64> = rows
        .iter()
        .filter_map(|r| {
            match (r.volume, r.avg_volume_10d) {
                (Some(v), Some(av)) if av > 0 => Some(v as f64 / av as f64),
                _ => None,
            }
        })
        .collect();
    let (vol_mean, vol_std) = if vol_ratios.len() >= 2 {
        let (m, s) = mean_std(&vol_ratios);
        (Some(m), Some(s))
    } else {
        (None, None)
    };

    SectorStats {
        pe_mean,
        pe_std,
        pb_mean,
        pb_std,
        price_mean,
        price_std,
        vol_ratio_mean: vol_mean,
        vol_ratio_std: vol_std,
    }
}

/// Calculate Z-scores for a single stock relative to sector stats.
fn calculate_z_scores(row: &StockMarketRow, stats: &SectorStats) -> ZScores {
    let price_z = if stats.price_std > 0.001 {
        (row.price_change_percent - stats.price_mean) / stats.price_std
    } else {
        0.0
    };

    let pe_z = match (row.pe_ratio, stats.pe_mean, stats.pe_std) {
        (Some(pe), Some(mean), Some(std)) if std > 0.001 => Some((pe - mean) / std),
        _ => None,
    };

    let pb_z = match (row.pb_ratio, stats.pb_mean, stats.pb_std) {
        (Some(pb), Some(mean), Some(std)) if std > 0.001 => Some((pb - mean) / std),
        _ => None,
    };

    let volume_z = match (row.volume, row.avg_volume_10d, stats.vol_ratio_mean, stats.vol_ratio_std) {
        (Some(v), Some(av), Some(mean), Some(std)) if av > 0 && std > 0.001 => {
            let ratio = v as f64 / av as f64;
            Some((ratio - mean) / std)
        }
        _ => None,
    };

    ZScores {
        pe_z,
        pb_z,
        price_z,
        volume_z,
    }
}

/// Calculate composite outlier score from Z-scores (weighted RMS).
fn calculate_composite_score(z: &ZScores) -> f64 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    // Price change: weight 0.3
    weighted_sum += 0.3 * z.price_z * z.price_z;
    total_weight += 0.3;

    // P/E: weight 0.3
    if let Some(pe) = z.pe_z {
        weighted_sum += 0.3 * pe * pe;
        total_weight += 0.3;
    }

    // P/B: weight 0.2
    if let Some(pb) = z.pb_z {
        weighted_sum += 0.2 * pb * pb;
        total_weight += 0.2;
    }

    // Volume: weight 0.2
    if let Some(vol) = z.volume_z {
        weighted_sum += 0.2 * vol * vol;
        total_weight += 0.2;
    }

    if total_weight > 0.0 {
        (weighted_sum / total_weight).sqrt()
    } else {
        0.0
    }
}

/// Classify the type of outlier based on Z-score directions.
fn classify_outlier(z: &ZScores) -> OutlierType {
    let pe_low = z.pe_z.map_or(false, |v| v < -1.0);
    let pe_high = z.pe_z.map_or(false, |v| v > 1.0);
    let pb_low = z.pb_z.map_or(false, |v| v < -1.0);
    let pb_high = z.pb_z.map_or(false, |v| v > 1.0);
    let price_high = z.price_z > 1.0;
    let price_low = z.price_z < -1.0;
    let vol_high = z.volume_z.map_or(false, |v| v > 1.0);

    if pe_low && pb_low {
        OutlierType::Undervalued
    } else if pe_high && pb_high {
        OutlierType::Overvalued
    } else if price_high && vol_high {
        OutlierType::Momentum
    } else if pe_low && price_low {
        OutlierType::ValueTrap
    } else if pe_high && price_high {
        OutlierType::GrowthPremium
    } else {
        OutlierType::Mixed
    }
}

/// Classify significance level from composite score.
fn classify_significance(score: f64) -> SignificanceLevel {
    if score >= 3.0 {
        SignificanceLevel::Extreme
    } else if score >= 2.0 {
        SignificanceLevel::Strong
    } else {
        SignificanceLevel::Moderate
    }
}

/// Save an outlier detection to the database.
async fn save_detection(
    pool: &SqlitePool,
    outlier: &OutlierStock,
    sector_id: i32,
    threshold: f64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO outlier_detections (
            stock_id, sector_id, pe_z_score, pb_z_score,
            price_z_score, volume_z_score, composite_score,
            outlier_type, significance_level, threshold_used
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(outlier.stock_id)
    .bind(sector_id)
    .bind(outlier.z_scores.pe_z)
    .bind(outlier.z_scores.pb_z)
    .bind(outlier.z_scores.price_z)
    .bind(outlier.z_scores.volume_z)
    .bind(outlier.composite_score)
    .bind(outlier.outlier_type.to_string())
    .bind(outlier.significance_level.to_string())
    .bind(threshold)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to save outlier detection: {e}"))?;

    Ok(())
}

/// Calculate mean and standard deviation of a slice.
fn mean_std(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n < 1.0 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / n;
    if n < 2.0 {
        return (mean, 0.0);
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, variance.sqrt())
}
