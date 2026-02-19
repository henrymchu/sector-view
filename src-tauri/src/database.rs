use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Get the database file path in the app's data directory.
fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {e}"))?;
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data directory: {e}"))?;
    Ok(data_dir.join("sector_view.db"))
}

/// Initialize the database: create the file, connect, and run migrations.
pub async fn init_database(app: &AppHandle) -> Result<SqlitePool, String> {
    let db_path = db_path(app)?;
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| format!("Failed to connect to database: {e}"))?;

    // Enable WAL mode for better concurrent read performance
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to set WAL mode: {e}"))?;

    run_migrations(&pool).await?;

    Ok(pool)
}

/// Run migrations by executing SQL files in order.
async fn run_migrations(pool: &SqlitePool) -> Result<(), String> {
    // Create migration tracking table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create migrations table: {e}"))?;

    // Check if 001_initial has been applied
    let applied: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM _migrations WHERE name = '001_initial'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to check migrations: {e}"))?;

    if !applied {
        let migration_sql = include_str!("../migrations/001_initial.sql");
        // Execute each statement separately
        for statement in migration_sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Migration 001_initial failed: {e}"))?;
            }
        }

        sqlx::query("INSERT INTO _migrations (name) VALUES ('001_initial')")
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to record migration: {e}"))?;

        println!("Applied migration: 001_initial");
    }

    Ok(())
}
