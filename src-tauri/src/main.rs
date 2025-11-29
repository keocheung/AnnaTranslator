#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache;
mod clipboard;
mod furigana;
mod history;
mod http_server;
mod text_replacements;

use anyhow::Result;
use tauri_plugin_store::Builder as StoreBuilder;

fn main() -> Result<()> {
    tauri::Builder::default()
        .plugin(StoreBuilder::default().build())
        .invoke_handler(tauri::generate_handler![
            clipboard::set_clipboard_watch,
            http_server::set_openai_compatible_input,
            text_replacements::set_text_replacements,
            furigana::annotate_furigana,
            cache::get_cached_translation,
            cache::store_translation,
            history::record_translation_history,
            history::get_translation_history,
            http_server::get_http_server_error
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let port = http_server::read_port_from_env();
                if let Err(err) = http_server::start_http_server(app_handle.clone(), port).await {
                    eprintln!("[tauri] failed to start HTTP listener: {err}");
                    http_server::record_http_error(&app_handle, port, &err);
                }
            });

            clipboard::start_clipboard_watcher(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
