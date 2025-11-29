use lindera::{
    dictionary::load_dictionary, mode::Mode, segmenter::Segmenter, token::Token,
    tokenizer::Tokenizer,
};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

static TOKENIZER: OnceLock<Mutex<Tokenizer>> = OnceLock::new();

fn dictionary_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data dir: {err}"))?;
    path.push("dictionary");
    path.push("unidic");
    Ok(path)
}

fn initialize_tokenizer(app: &AppHandle) -> Result<(), String> {
    let path = dictionary_path(app)?;
    let dictionary_uri = format!("file://{}", path.to_string_lossy());
    let dictionary = load_dictionary(&dictionary_uri)
        .map_err(|err| format!("failed to load dictionary: {err}"))?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Mutex::new(Tokenizer::new(segmenter));

    TOKENIZER
        .set(tokenizer)
        .map_err(|_| "tokenizer already initialized".to_string())
}

fn tokenizer(app: &AppHandle) -> Result<std::sync::MutexGuard<'static, Tokenizer>, String> {
    if TOKENIZER.get().is_none() {
        initialize_tokenizer(app)?;
    }

    TOKENIZER
        .get()
        .ok_or_else(|| "tokenizer not initialized".to_string())?
        .lock()
        .map_err(|err| format!("tokenizer lock poisoned: {err}"))
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn katakana_to_hiragana(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if ('\u{30A1}'..='\u{30F6}').contains(&c) {
                // Katakana block to Hiragana offset.
                char::from_u32((c as u32) - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

fn token_reading(token: &mut Token) -> Option<String> {
    token.get_detail(7).and_then(|reading| {
        let trimmed = reading.trim();
        if trimmed.is_empty() || trimmed == "*" {
            None
        } else {
            Some(katakana_to_hiragana(trimmed))
        }
    })
}

fn annotate_with_furigana(app: &AppHandle, text: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let tokenizer = tokenizer(app)?;
    let mut tokens = tokenizer
        .tokenize(text)
        .map_err(|err| format!("tokenization failed: {err}"))?;
    let mut annotated = String::with_capacity(text.len() * 2);
    let mut cursor = 0usize;

    for token in tokens.iter_mut() {
        if token.byte_start > cursor && token.byte_start <= text.len() {
            annotated.push_str(&escape_html(&text[cursor..token.byte_start]));
        }

        let start = token.byte_start.min(text.len());
        let end = token.byte_end.min(text.len());
        let surface = &text[start..end];

        if let Some(reading) = token_reading(token) {
            if reading == surface {
                annotated.push_str(&escape_html(surface));
            } else {
                annotated.push_str("<ruby>");
                annotated.push_str(&escape_html(surface));
                annotated.push_str("<rt>");
                annotated.push_str(&escape_html(&reading));
                annotated.push_str("</rt></ruby>");
            }
        } else {
            annotated.push_str(&escape_html(surface));
        }

        cursor = end;
    }

    if cursor < text.len() {
        annotated.push_str(&escape_html(&text[cursor..]));
    }

    Ok(annotated)
}

#[tauri::command]
pub fn annotate_furigana(app: AppHandle, text: String) -> Result<String, String> {
    annotate_with_furigana(&app, &text)
}
