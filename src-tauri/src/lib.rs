mod cache;
mod commands;
mod database;
mod market_data;
mod outlier_detection;
mod types;

use cache::SectorCache;
use sqlx::sqlite::SqlitePool;
use tauri::Manager;

pub struct DbState(pub SqlitePool);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_sectors,
            commands::get_stocks_by_sector,
            commands::get_sector_performance,
            commands::refresh_market_data,
            commands::refresh_sector_data,
            commands::detect_outliers,
            commands::get_sector_outliers,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize cache
            handle.manage(SectorCache::new());

            tauri::async_runtime::block_on(async move {
                match database::init_database(&handle).await {
                    Ok(pool) => {
                        handle.manage(DbState(pool));
                        println!("Database initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("Database initialization failed: {e}");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
