use crate::text_replacements::{apply_text_replacements, emit_processed_text};
use anyhow::{Error, Result};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 17889;

static OPENAI_COMPATIBLE_INPUT: AtomicBool = AtomicBool::new(false);
static HTTP_SERVER_ERROR: Lazy<Mutex<Option<HttpServerErrorPayload>>> =
    Lazy::new(|| Mutex::new(None));

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
    text: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HttpServerErrorPayload {
    pub port: u16,
    pub message: String,
}

async fn submit(State(app): State<AppHandle>, body: String) -> impl axum::response::IntoResponse {
    println!("[tauri] received /submit, len={}", body.len());
    if let Err(err) = emit_processed_text(&app, &body) {
        eprintln!("[tauri] failed to emit incoming_text event: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
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
        let processed = apply_text_replacements(&text);
        if let Err(err) = app.emit("incoming_text", processed) {
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

pub async fn start_http_server(app: AppHandle, port: u16) -> Result<()> {
    let app_router = Router::new()
        .route("/submit", post(submit))
        .route("/v1/chat/completions", post(openai_chat_completions))
        .with_state(app.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    println!("[tauri] HTTP server listening on http://{addr}");
    axum::serve(listener, app_router).await?;
    Ok(())
}

pub fn read_port_from_env() -> u16 {
    std::env::var("TRANSLATOR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

#[tauri::command]
pub fn set_openai_compatible_input(enabled: bool) {
    OPENAI_COMPATIBLE_INPUT.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_http_server_error() -> Option<HttpServerErrorPayload> {
    HTTP_SERVER_ERROR
        .lock()
        .ok()
        .and_then(|state| state.clone())
}

pub fn record_http_error(app_handle: &AppHandle, port: u16, err: &Error) {
    let payload = HttpServerErrorPayload {
        port,
        message: err.to_string(),
    };
    if let Ok(mut last_error) = HTTP_SERVER_ERROR.lock() {
        *last_error = Some(payload.clone());
    }
    if let Err(emit_err) = app_handle.emit("http_server_failed", payload) {
        eprintln!("[tauri] failed to notify frontend about HTTP listener error: {emit_err}");
    }
}
