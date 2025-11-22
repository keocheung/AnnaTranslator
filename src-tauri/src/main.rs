#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter};
use serde_json::json;

const DEFAULT_PORT: u16 = 17889;

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

fn main() -> Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            // Clone to detach lifetime from setup closure.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let port = read_port_from_env();
                if let Err(err) = start_http_server(app_handle.clone(), port).await {
                    eprintln!("[tauri] failed to start HTTP listener: {err}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
