use crate::text_replacements::apply_text_replacements;
use arboard::Clipboard;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::task::spawn_blocking;
use tokio::time::sleep;

static CLIPBOARD_ENABLED: AtomicBool = AtomicBool::new(false);

async fn poll_clipboard_text() -> anyhow::Result<Option<String>> {
    let text = spawn_blocking(|| -> anyhow::Result<Option<String>> {
        let mut clipboard = Clipboard::new()?;
        Ok(clipboard.get_text().ok())
    })
    .await
    .map_err(|err| anyhow::anyhow!("clipboard task join error: {err}"))??;

    Ok(text.map(|t| t.trim().to_string()))
}

pub fn start_clipboard_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last = String::new();

        loop {
            if !CLIPBOARD_ENABLED.load(Ordering::Relaxed) {
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            match poll_clipboard_text().await {
                Ok(Some(text)) if !text.is_empty() => {
                    let processed = apply_text_replacements(&text);
                    if processed.is_empty() || processed == last {
                        sleep(Duration::from_millis(1500)).await;
                        continue;
                    }

                    last = processed.clone();
                    if let Err(err) = app.emit("incoming_text", processed) {
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
pub fn set_clipboard_watch(enabled: bool) {
    CLIPBOARD_ENABLED.store(enabled, Ordering::Relaxed);
}
