#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use arboard::Clipboard;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde_json::json;
use tauri_plugin_store::Builder as StoreBuilder;
use tokio::task::spawn_blocking;
use tokio::time::sleep;
use std::sync::atomic::{AtomicBool, Ordering};

const DEFAULT_PORT: u16 = 17889;
static CLIPBOARD_ENABLED: AtomicBool = AtomicBool::new(false);
static OPENAI_COMPATIBLE_INPUT: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize, Debug)]
struct IncomingText {
    text: String,
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
            eprintln!(
                "[tauri] failed to emit incoming_text from OpenAI-compatible input: {err}"
            );
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

fn main() -> Result<()> {
    tauri::Builder::default()
        .plugin(StoreBuilder::default().build())
        .invoke_handler(tauri::generate_handler![
            set_clipboard_watch,
            set_openai_compatible_input
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
