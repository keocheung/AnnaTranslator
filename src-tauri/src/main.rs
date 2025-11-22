#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use arboard::Clipboard;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use rusqlite::{Connection, OptionalExtension};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::Builder as StoreBuilder;
use tokio::task::spawn_blocking;
use tokio::time::sleep;
use xxhash_rust::xxh3::xxh3_64;

const DEFAULT_PORT: u16 = 17889;
static CLIPBOARD_ENABLED: AtomicBool = AtomicBool::new(false);
static OPENAI_COMPATIBLE_INPUT: AtomicBool = AtomicBool::new(false);
static TRANSLATION_HISTORY: Lazy<Mutex<Vec<HistoryEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));
const MAX_HISTORY: usize = 1000;

fn cache_db_path(app: &AppHandle) -> Result<PathBuf> {
    let mut dir = app.path().app_data_dir()?;
    dir.push("cache");
    fs::create_dir_all(&dir)?;
    dir.push("translations.sqlite3");
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

#[derive(Deserialize, Debug)]
struct OpenAIChatCompletionRequest {
    messages: Vec<OpenAIMessage>,
}

#[derive(Deserialize, Debug)]
struct OpenAIMessage {
    role: String,
    #[serde(default)]
    content: OpenAIContent,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Parts(Vec<OpenAIContentPart>),
}

impl Default for OpenAIContent {
    fn default() -> Self {
        OpenAIContent::Text(String::new())
    }
}

#[derive(Deserialize, Debug)]
struct OpenAIContentPart {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    text: Option<String>,
}

async fn submit(
    State(app): State<AppHandle>,
    body: String,
) -> impl axum::response::IntoResponse {
    println!("[tauri] received /submit, len={}", body.len());
    if let Err(err) = app.emit("incoming_text", body) {
        eprintln!("[tauri] failed to emit incoming_text event: {err}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "error", "message": err.to_string() })),
        );
    }
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

async fn openai_chat_completions(
    State(app): State<AppHandle>,
    Json(payload): Json<OpenAIChatCompletionRequest>,
) -> impl axum::response::IntoResponse {
    if !OPENAI_COMPATIBLE_INPUT.load(Ordering::Relaxed) {
        return StatusCode::NOT_FOUND;
    }

    let maybe_text = payload
        .messages
        .iter()
        .rev()
        .find(|msg| msg.role.eq_ignore_ascii_case("user"))
        .and_then(|msg| extract_content_text(&msg.content))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    if let Some(text) = maybe_text {
        println!(
            "[tauri] received OpenAI-compatible /v1/chat/completions, len={}",
            text.len()
        );
        if let Err(err) = app.emit("incoming_text", text) {
            eprintln!("[tauri] failed to emit incoming_text from OpenAI-compatible input: {err}");
        }
    } else {
        eprintln!("[tauri] OpenAI-compatible request missing user message");
    }

    StatusCode::NOT_FOUND
}

fn extract_content_text(content: &OpenAIContent) -> Option<String> {
    match content {
        OpenAIContent::Text(t) => Some(t.clone()),
        OpenAIContent::Parts(parts) => parts.iter().find_map(|part| {
            part.text
                .as_ref()
                .map(|t| t.to_string())
                .filter(|t| !t.trim().is_empty())
        }),
    }
}

async fn start_http_server(app: AppHandle, port: u16) -> Result<()> {
    let app_router = Router::new()
        .route("/submit", post(submit))
        .route("/v1/chat/completions", post(openai_chat_completions))
        .with_state(app.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("[tauri] HTTP server listening on http://{addr}");
    axum::serve(listener, app_router).await?;
    Ok(())
}

fn read_port_from_env() -> u16 {
    std::env::var("TRANSLATOR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

async fn poll_clipboard_text() -> Result<Option<String>> {
    let text = spawn_blocking(|| -> Result<Option<String>> {
        let mut clipboard = Clipboard::new()?;
        Ok(clipboard.get_text().ok())
    })
    .await
    .map_err(|err| anyhow::anyhow!("clipboard task join error: {err}"))??;

    Ok(text.map(|t| t.trim().to_string()))
}

fn start_clipboard_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last = String::new();

        loop {
            if !CLIPBOARD_ENABLED.load(Ordering::Relaxed) {
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            match poll_clipboard_text().await {
                Ok(Some(text)) if !text.is_empty() && text != last => {
                    last = text.clone();
                    if let Err(err) = app.emit("incoming_text", text) {
                        eprintln!("[tauri] failed to emit clipboard incoming_text: {err}");
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    eprintln!("[tauri] clipboard poll failed: {err}");
                }
            }

            sleep(Duration::from_millis(1500)).await;
        }
    });
}

#[tauri::command]
fn set_clipboard_watch(enabled: bool) {
    CLIPBOARD_ENABLED.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
fn set_openai_compatible_input(enabled: bool) {
    OPENAI_COMPATIBLE_INPUT.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Serialize)]
struct HistoryEntry {
    original: String,
    translation: String,
}

#[tauri::command]
fn record_translation_history(app: AppHandle, original: String, translation: String) {
    if original.trim().is_empty() || translation.trim().is_empty() {
        return;
    }

    let mut history = TRANSLATION_HISTORY
        .lock()
        .expect("translation history mutex poisoned");
    history.push(HistoryEntry {
        original,
        translation,
    });

    if history.len() > MAX_HISTORY {
        let overflow = history.len() - MAX_HISTORY;
        history.drain(0..overflow);
    }

    drop(history);

    if let Err(err) = app.emit("translation_history_updated", ()) {
        eprintln!("[tauri] failed to emit translation_history_updated: {err}");
    }
}

#[tauri::command]
fn get_translation_history() -> Vec<HistoryEntry> {
    TRANSLATION_HISTORY
        .lock()
        .expect("translation history mutex poisoned")
        .clone()
}

#[tauri::command]
async fn get_cached_translation(app: AppHandle, text: String) -> Result<Option<String>, String> {
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
async fn store_translation(
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

fn main() -> Result<()> {
    tauri::Builder::default()
        .plugin(StoreBuilder::default().build())
        .invoke_handler(tauri::generate_handler![
            set_clipboard_watch,
            set_openai_compatible_input,
            get_cached_translation,
            store_translation,
            record_translation_history,
            get_translation_history
        ])
        .setup(|app| {
            // Clone to detach lifetime from setup closure.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let port = read_port_from_env();
                if let Err(err) = start_http_server(app_handle.clone(), port).await {
                    eprintln!("[tauri] failed to start HTTP listener: {err}");
                }
            });

            start_clipboard_watcher(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
