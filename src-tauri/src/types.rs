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

// -- Outlier Detection Types --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZScores {
    pub pe_z: Option<f64>,
    pub pb_z: Option<f64>,
    pub price_z: f64,
    pub volume_z: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutlierType {
    Undervalued,
    Overvalued,
    Momentum,
    ValueTrap,
    GrowthPremium,
    Mixed,
}

impl std::fmt::Display for OutlierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutlierType::Undervalued => write!(f, "Undervalued"),
            OutlierType::Overvalued => write!(f, "Overvalued"),
            OutlierType::Momentum => write!(f, "Momentum"),
            OutlierType::ValueTrap => write!(f, "ValueTrap"),
            OutlierType::GrowthPremium => write!(f, "GrowthPremium"),
            OutlierType::Mixed => write!(f, "Mixed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignificanceLevel {
    Moderate,
    Strong,
    Extreme,
}

impl std::fmt::Display for SignificanceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignificanceLevel::Moderate => write!(f, "Moderate"),
            SignificanceLevel::Strong => write!(f, "Strong"),
            SignificanceLevel::Extreme => write!(f, "Extreme"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierStock {
    pub stock_id: i32,
    pub symbol: String,
    pub name: String,
    pub z_scores: ZScores,
    pub composite_score: f64,
    pub outlier_type: OutlierType,
    pub significance_level: SignificanceLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorOutliers {
    pub sector_id: i32,
    pub sector_name: String,
    pub sector_symbol: String,
    pub outlier_count: usize,
    pub outliers: Vec<OutlierStock>,
}
