use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Sector {
    pub id: i32,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Stock {
    pub id: i32,
    pub symbol: String,
    pub name: String,
    pub sector_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorSummary {
    pub sector_id: i32,
    pub name: String,
    pub symbol: String,
    pub avg_change_percent: f64,
    pub avg_pe_ratio: Option<f64>,
    pub total_market_cap: Option<i64>,
    pub stock_count: i32,
    pub avg_beta: Option<f64>,
}
