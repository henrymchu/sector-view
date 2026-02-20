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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn make_row(
        stock_id: i32,
        symbol: &str,
        price_change_percent: f64,
        pe_ratio: Option<f64>,
        pb_ratio: Option<f64>,
        volume: Option<i64>,
        avg_volume_10d: Option<i64>,
    ) -> StockMarketRow {
        StockMarketRow {
            stock_id,
            symbol: symbol.to_string(),
            name: format!("Company {symbol}"),
            sector_id: 1,
            price_change_percent,
            pe_ratio,
            pb_ratio,
            volume,
            avg_volume_10d,
        }
    }

    // ---- mean_std ----

    #[test]
    fn test_mean_std_empty() {
        let (mean, std) = mean_std(&[]);
        assert_eq!(mean, 0.0);
        assert_eq!(std, 0.0);
    }

    #[test]
    fn test_mean_std_single_value() {
        let (mean, std) = mean_std(&[5.0]);
        assert!(approx_eq(mean, 5.0));
        assert_eq!(std, 0.0);
    }

    #[test]
    fn test_mean_std_two_values() {
        // mean = (1+3)/2 = 2.0
        // variance = ((1-2)² + (3-2)²) / (2-1) = 2.0, std = sqrt(2)
        let (mean, std) = mean_std(&[1.0, 3.0]);
        assert!(approx_eq(mean, 2.0));
        assert!(approx_eq(std, 2.0_f64.sqrt()));
    }

    #[test]
    fn test_mean_std_known_dataset() {
        // [1, 2, 3]: mean=2, variance=((1-2)²+(2-2)²+(3-2)²)/2=1.0, std=1.0
        let (mean, std) = mean_std(&[1.0, 2.0, 3.0]);
        assert!(approx_eq(mean, 2.0));
        assert!(approx_eq(std, 1.0));
    }

    #[test]
    fn test_mean_std_all_same_values() {
        // Zero std dev when all values are equal
        let (mean, std) = mean_std(&[5.0, 5.0, 5.0, 5.0]);
        assert!(approx_eq(mean, 5.0));
        assert!(approx_eq(std, 0.0));
    }

    #[test]
    fn test_mean_std_negative_values() {
        // [-1, 0, 1]: mean=0.0, variance=(1+0+1)/2=1.0, std=1.0
        let (mean, std) = mean_std(&[-1.0, 0.0, 1.0]);
        assert!(approx_eq(mean, 0.0));
        assert!(approx_eq(std, 1.0));
    }

    // ---- calculate_stats ----

    #[test]
    fn test_calculate_stats_all_data() {
        let rows = vec![
            make_row(1, "A", 1.0, Some(10.0), Some(1.0), Some(1000), Some(500)),
            make_row(2, "B", 2.0, Some(20.0), Some(2.0), Some(2000), Some(1000)),
            make_row(3, "C", 3.0, Some(30.0), Some(3.0), Some(3000), Some(1500)),
        ];
        let stats = calculate_stats(&rows);

        // price: [1,2,3] → mean=2.0, std=1.0
        assert!(approx_eq(stats.price_mean, 2.0));
        assert!(approx_eq(stats.price_std, 1.0));

        // pe: [10,20,30] → mean=20.0, std=10.0
        assert!(approx_eq(stats.pe_mean.unwrap(), 20.0));
        assert!(approx_eq(stats.pe_std.unwrap(), 10.0));

        // pb: [1,2,3] → mean=2.0, std=1.0
        assert!(approx_eq(stats.pb_mean.unwrap(), 2.0));
        assert!(approx_eq(stats.pb_std.unwrap(), 1.0));

        // vol_ratio: [2.0, 2.0, 2.0] (each stock trades at 2x avg) → mean=2.0, std=0.0
        assert!(approx_eq(stats.vol_ratio_mean.unwrap(), 2.0));
        assert!(approx_eq(stats.vol_ratio_std.unwrap(), 0.0));
    }

    #[test]
    fn test_calculate_stats_missing_pe_returns_none() {
        // Only 1 stock has PE data — needs >= 2 for stats
        let rows = vec![
            make_row(1, "A", 1.0, Some(10.0), None, None, None),
            make_row(2, "B", 2.0, None, None, None, None),
            make_row(3, "C", 3.0, None, None, None, None),
        ];
        let stats = calculate_stats(&rows);
        assert!(stats.pe_mean.is_none());
        assert!(stats.pe_std.is_none());
    }

    #[test]
    fn test_calculate_stats_no_volume_data() {
        let rows = vec![
            make_row(1, "A", 1.0, None, None, None, None),
            make_row(2, "B", 2.0, None, None, None, None),
            make_row(3, "C", 3.0, None, None, None, None),
        ];
        let stats = calculate_stats(&rows);
        assert!(stats.vol_ratio_mean.is_none());
        assert!(stats.vol_ratio_std.is_none());
    }

    // ---- calculate_z_scores ----

    #[test]
    fn test_z_scores_all_present() {
        let stats = SectorStats {
            price_mean: 0.0,
            price_std: 1.0,
            pe_mean: Some(20.0),
            pe_std: Some(5.0),
            pb_mean: Some(3.0),
            pb_std: Some(1.0),
            vol_ratio_mean: Some(1.0),
            vol_ratio_std: Some(0.5),
        };
        // vol ratio = 2_000_000 / 1_000_000 = 2.0
        let row = make_row(1, "AAPL", 2.0, Some(30.0), Some(5.0), Some(2_000_000), Some(1_000_000));
        let z = calculate_z_scores(&row, &stats);

        // price_z = (2.0 - 0.0) / 1.0 = 2.0
        assert!(approx_eq(z.price_z, 2.0));
        // pe_z = (30 - 20) / 5 = 2.0
        assert!(approx_eq(z.pe_z.unwrap(), 2.0));
        // pb_z = (5 - 3) / 1 = 2.0
        assert!(approx_eq(z.pb_z.unwrap(), 2.0));
        // volume_z = (2.0 - 1.0) / 0.5 = 2.0
        assert!(approx_eq(z.volume_z.unwrap(), 2.0));
    }

    #[test]
    fn test_z_scores_negative_deviation() {
        let stats = SectorStats {
            price_mean: 0.0,
            price_std: 2.0,
            pe_mean: Some(20.0),
            pe_std: Some(5.0),
            pb_mean: None,
            pb_std: None,
            vol_ratio_mean: None,
            vol_ratio_std: None,
        };
        let row = make_row(1, "X", -4.0, Some(10.0), None, None, None);
        let z = calculate_z_scores(&row, &stats);

        // price_z = (-4.0 - 0.0) / 2.0 = -2.0
        assert!(approx_eq(z.price_z, -2.0));
        // pe_z = (10 - 20) / 5 = -2.0
        assert!(approx_eq(z.pe_z.unwrap(), -2.0));
    }

    #[test]
    fn test_z_scores_zero_price_std_returns_zero() {
        // price_std <= 0.001 → price_z must be 0.0
        let stats = SectorStats {
            price_mean: 1.0,
            price_std: 0.0,
            pe_mean: None,
            pe_std: None,
            pb_mean: None,
            pb_std: None,
            vol_ratio_mean: None,
            vol_ratio_std: None,
        };
        let row = make_row(1, "A", 5.0, None, None, None, None);
        let z = calculate_z_scores(&row, &stats);
        assert_eq!(z.price_z, 0.0);
    }

    #[test]
    fn test_z_scores_missing_pe_ratio_gives_none() {
        let stats = SectorStats {
            price_mean: 0.0,
            price_std: 1.0,
            pe_mean: Some(20.0),
            pe_std: Some(5.0),
            pb_mean: None,
            pb_std: None,
            vol_ratio_mean: None,
            vol_ratio_std: None,
        };
        let row = make_row(1, "A", 1.0, None, None, None, None);
        let z = calculate_z_scores(&row, &stats);
        assert!(z.pe_z.is_none());
    }

    #[test]
    fn test_z_scores_zero_pe_std_gives_none() {
        // pe_std <= 0.001 → pe_z must be None even if pe_ratio is present
        let stats = SectorStats {
            price_mean: 0.0,
            price_std: 1.0,
            pe_mean: Some(20.0),
            pe_std: Some(0.0),
            pb_mean: None,
            pb_std: None,
            vol_ratio_mean: None,
            vol_ratio_std: None,
        };
        let row = make_row(1, "A", 1.0, Some(25.0), None, None, None);
        let z = calculate_z_scores(&row, &stats);
        assert!(z.pe_z.is_none());
    }

    #[test]
    fn test_z_scores_zero_avg_volume_gives_none() {
        // avg_volume_10d = 0 → volume_z must be None (avoid division by zero)
        let stats = SectorStats {
            price_mean: 0.0,
            price_std: 1.0,
            pe_mean: None,
            pe_std: None,
            pb_mean: None,
            pb_std: None,
            vol_ratio_mean: Some(1.0),
            vol_ratio_std: Some(0.5),
        };
        let row = make_row(1, "A", 1.0, None, None, Some(1_000_000), Some(0));
        let z = calculate_z_scores(&row, &stats);
        assert!(z.volume_z.is_none());
    }

    // ---- calculate_composite_score ----

    #[test]
    fn test_composite_score_all_present() {
        // All z = 2.0: weighted_sum = 0.3*4+0.3*4+0.2*4+0.2*4 = 4.0, weight = 1.0
        // score = sqrt(4.0/1.0) = 2.0
        let z = ZScores { price_z: 2.0, pe_z: Some(2.0), pb_z: Some(2.0), volume_z: Some(2.0) };
        assert!(approx_eq(calculate_composite_score(&z), 2.0));
    }

    #[test]
    fn test_composite_score_price_only() {
        // price_z=2.0, others None: weighted_sum=0.3*4=1.2, weight=0.3
        // score = sqrt(1.2/0.3) = sqrt(4.0) = 2.0
        let z = ZScores { price_z: 2.0, pe_z: None, pb_z: None, volume_z: None };
        assert!(approx_eq(calculate_composite_score(&z), 2.0));
    }

    #[test]
    fn test_composite_score_all_zero() {
        let z = ZScores { price_z: 0.0, pe_z: Some(0.0), pb_z: Some(0.0), volume_z: Some(0.0) };
        assert!(approx_eq(calculate_composite_score(&z), 0.0));
    }

    #[test]
    fn test_composite_score_mixed_presence() {
        // price_z=1.0, pe_z=3.0, others None
        // weighted_sum = 0.3*1 + 0.3*9 = 3.0, weight = 0.6
        // score = sqrt(3.0/0.6) = sqrt(5.0)
        let z = ZScores { price_z: 1.0, pe_z: Some(3.0), pb_z: None, volume_z: None };
        assert!(approx_eq(calculate_composite_score(&z), 5.0_f64.sqrt()));
    }

    #[test]
    fn test_composite_score_negative_z_uses_squares() {
        // Negative z-scores → same composite as positive (squaring removes sign)
        let pos = ZScores { price_z: 2.0, pe_z: Some(2.0), pb_z: Some(2.0), volume_z: Some(2.0) };
        let neg = ZScores { price_z: -2.0, pe_z: Some(-2.0), pb_z: Some(-2.0), volume_z: Some(-2.0) };
        assert!(approx_eq(calculate_composite_score(&pos), calculate_composite_score(&neg)));
    }

    // ---- classify_outlier ----

    #[test]
    fn test_classify_undervalued() {
        // pe_z < -1 AND pb_z < -1
        let z = ZScores { price_z: 0.0, pe_z: Some(-2.0), pb_z: Some(-2.0), volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::Undervalued));
    }

    #[test]
    fn test_classify_overvalued() {
        // pe_z > 1 AND pb_z > 1
        let z = ZScores { price_z: 0.0, pe_z: Some(2.0), pb_z: Some(2.0), volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::Overvalued));
    }

    #[test]
    fn test_classify_momentum() {
        // price_z > 1 AND volume_z > 1, with pe/pb absent so earlier conditions don't fire
        let z = ZScores { price_z: 2.0, pe_z: None, pb_z: None, volume_z: Some(2.0) };
        assert!(matches!(classify_outlier(&z), OutlierType::Momentum));
    }

    #[test]
    fn test_classify_value_trap() {
        // pe_z < -1 AND price_z < -1, but pb_z absent so Undervalued doesn't trigger
        let z = ZScores { price_z: -2.0, pe_z: Some(-2.0), pb_z: None, volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::ValueTrap));
    }

    #[test]
    fn test_classify_growth_premium() {
        // pe_z > 1 AND price_z > 1, but pb_z absent so Overvalued doesn't trigger
        let z = ZScores { price_z: 2.0, pe_z: Some(2.0), pb_z: None, volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::GrowthPremium));
    }

    #[test]
    fn test_classify_mixed() {
        // No condition met
        let z = ZScores { price_z: 0.5, pe_z: None, pb_z: None, volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::Mixed));
    }

    #[test]
    fn test_classify_boundary_exactly_one_not_triggered() {
        // pe_z = 1.0 uses strict >, so pe_high = false → Mixed
        let z = ZScores { price_z: 0.0, pe_z: Some(1.0), pb_z: Some(1.0), volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::Mixed));
    }

    #[test]
    fn test_classify_boundary_exactly_neg_one_not_triggered() {
        // pe_z = -1.0 uses strict <, so pe_low = false → Mixed
        let z = ZScores { price_z: 0.0, pe_z: Some(-1.0), pb_z: Some(-1.0), volume_z: None };
        assert!(matches!(classify_outlier(&z), OutlierType::Mixed));
    }

    // ---- classify_significance ----

    #[test]
    fn test_significance_moderate_range() {
        assert!(matches!(classify_significance(0.0), SignificanceLevel::Moderate));
        assert!(matches!(classify_significance(1.0), SignificanceLevel::Moderate));
        assert!(matches!(classify_significance(1.9999999), SignificanceLevel::Moderate));
    }

    #[test]
    fn test_significance_strong_range() {
        assert!(matches!(classify_significance(2.0), SignificanceLevel::Strong));
        assert!(matches!(classify_significance(2.5), SignificanceLevel::Strong));
        assert!(matches!(classify_significance(2.9999999), SignificanceLevel::Strong));
    }

    #[test]
    fn test_significance_extreme_range() {
        assert!(matches!(classify_significance(3.0), SignificanceLevel::Extreme));
        assert!(matches!(classify_significance(5.0), SignificanceLevel::Extreme));
        assert!(matches!(classify_significance(100.0), SignificanceLevel::Extreme));
    }

    #[test]
    fn test_significance_boundary_at_2() {
        assert!(matches!(classify_significance(2.0), SignificanceLevel::Strong));
        assert!(matches!(classify_significance(1.9999999), SignificanceLevel::Moderate));
    }

    #[test]
    fn test_significance_boundary_at_3() {
        assert!(matches!(classify_significance(3.0), SignificanceLevel::Extreme));
        assert!(matches!(classify_significance(2.9999999), SignificanceLevel::Strong));
    }

    // ---- Performance ----

    #[test]
    fn test_performance_500_stocks() {
        use std::time::Instant;

        let rows: Vec<StockMarketRow> = (0..500_i32)
            .map(|i| {
                make_row(
                    i,
                    &format!("S{i:03}"),
                    (i % 10) as f64 - 5.0,
                    Some(10.0 + (i % 30) as f64),
                    Some(1.0 + (i % 5) as f64),
                    Some(1_000_000 + i as i64 * 1_000),
                    Some(1_000_000),
                )
            })
            .collect();

        let start = Instant::now();
        let stats = calculate_stats(&rows);
        for row in &rows {
            let z = calculate_z_scores(row, &stats);
            let composite = calculate_composite_score(&z);
            let _ = classify_outlier(&z);
            let _ = classify_significance(composite);
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 100,
            "Performance: 500 stocks took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
    }
}
