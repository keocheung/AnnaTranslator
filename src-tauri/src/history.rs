use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

const MAX_HISTORY: usize = 1000;

#[derive(Clone, Serialize)]
pub struct HistoryEntry {
    pub original: String,
    pub translation: String,
}

static TRANSLATION_HISTORY: Lazy<Mutex<Vec<HistoryEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[tauri::command]
pub fn record_translation_history(app: AppHandle, original: String, translation: String) {
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
pub fn get_translation_history() -> Vec<HistoryEntry> {
    TRANSLATION_HISTORY
        .lock()
        .expect("translation history mutex poisoned")
        .clone()
}
