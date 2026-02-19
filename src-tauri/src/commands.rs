use crate::types::{Sector, Stock};
use crate::DbState;
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
