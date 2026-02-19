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
