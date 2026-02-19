mod commands;
mod database;
mod types;

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
        ])
        .setup(|app| {
            let handle = app.handle().clone();
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
