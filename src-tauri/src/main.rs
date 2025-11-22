#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use arboard::Clipboard;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde_json::json;
use tokio::task::spawn_blocking;
use tokio::time::sleep;
use std::sync::atomic::{AtomicBool, Ordering};

const DEFAULT_PORT: u16 = 17889;
static CLIPBOARD_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize, Debug)]
struct IncomingText {
    text: String,
}

async fn submit(
    State(app): State<AppHandle>,
    Json(payload): Json<IncomingText>,
) -> impl axum::response::IntoResponse {
    let text = payload.text;
    println!("[tauri] received /submit, len={}", text.len());
    if let Err(err) = app.emit("incoming_text", text) {
        eprintln!("[tauri] failed to emit incoming_text event: {err}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "error", "message": err.to_string() })),
        );
    }
    (
        StatusCode::OK,
        Json(json!({ "status": "ok" })),
    )
}

async fn start_http_server(app: AppHandle, port: u16) -> Result<()> {
    let app_router = Router::new()
        .route("/submit", post(submit))
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

fn main() -> Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![set_clipboard_watch])
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
