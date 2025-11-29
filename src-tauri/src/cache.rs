use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use xxhash_rust::xxh3::xxh3_64;

const CACHE_FILENAME: &str = "translations.sqlite3";

fn cache_db_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let mut dir = app.path().app_data_dir()?;
    dir.push("cache");
    fs::create_dir_all(&dir)?;
    dir.push(CACHE_FILENAME);
    Ok(dir)
}

fn init_cache_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS translations (
            key TEXT PRIMARY KEY,
            original TEXT NOT NULL,
            translation TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn cache_key(text: &str) -> String {
    format!("{:016x}", xxh3_64(text.as_bytes()))
}

#[tauri::command]
pub async fn get_cached_translation(
    app: AppHandle,
    text: String,
) -> Result<Option<String>, String> {
    let path = cache_db_path(&app).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        init_cache_schema(&conn).map_err(|e| e.to_string())?;
        let key = cache_key(&text);
        let mut stmt = conn
            .prepare("SELECT translation FROM translations WHERE key = ?1 LIMIT 1")
            .map_err(|e| e.to_string())?;
        let translation = stmt
            .query_row([key], |row| row.get::<_, String>(0))
            .optional()
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(translation)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn store_translation(
    app: AppHandle,
    text: String,
    translation: String,
) -> Result<(), String> {
    if translation.trim().is_empty() {
        return Ok(());
    }

    let path = cache_db_path(&app).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        init_cache_schema(&conn).map_err(|e| e.to_string())?;
        let key = cache_key(&text);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO translations (key, original, translation, created_at)
            VALUES (?1, ?2, ?3, ?4)",
            (&key, &text, &translation, now),
        )
        .map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}
