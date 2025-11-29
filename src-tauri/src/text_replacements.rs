use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

#[derive(Debug)]
pub struct TextReplacementRule {
    pub regex: Regex,
    pub replacement: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct ReplacementRulePayload {
    pub pattern: String,
    pub replacement: String,
    #[serde(default)]
    pub flags: String,
}

static TEXT_REPLACEMENTS: Lazy<Mutex<Vec<TextReplacementRule>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

fn build_regex(pattern: &str, flags: &str) -> Result<Regex, regex::Error> {
    let mut builder = RegexBuilder::new(pattern);
    for flag in flags.chars() {
        match flag {
            'i' | 'I' => {
                builder.case_insensitive(true);
            }
            'm' | 'M' => {
                builder.multi_line(true);
            }
            's' | 'S' => {
                builder.dot_matches_new_line(true);
            }
            'x' | 'X' => {
                builder.ignore_whitespace(true);
            }
            'U' => {
                builder.swap_greed(true);
            }
            _ => {}
        };
    }
    builder.build()
}

fn compile_replacement_rules(rules: Vec<ReplacementRulePayload>) -> Vec<TextReplacementRule> {
    let mut compiled = Vec::new();
    for rule in rules {
        if rule.pattern.trim().is_empty() {
            continue;
        }

        match build_regex(&rule.pattern, &rule.flags) {
            Ok(regex) => compiled.push(TextReplacementRule {
                regex,
                replacement: rule.replacement,
            }),
            Err(err) => {
                eprintln!(
                    "[tauri] failed to compile regex '{}': {}",
                    rule.pattern, err
                );
            }
        }
    }
    compiled
}

pub fn apply_text_replacements(raw: &str) -> String {
    let rules = TEXT_REPLACEMENTS
        .lock()
        .expect("text replacement mutex poisoned");

    if rules.is_empty() {
        return raw.to_string();
    }

    let mut output = raw.to_string();
    for rule in rules.iter() {
        output = rule
            .regex
            .replace_all(&output, rule.replacement.as_str())
            .into_owned();
    }
    output
}

pub fn emit_processed_text(app: &AppHandle, raw: &str) -> Result<(), tauri::Error> {
    let processed = apply_text_replacements(raw);
    app.emit("incoming_text", processed)
}

#[tauri::command]
pub fn set_text_replacements(rules: Vec<ReplacementRulePayload>) -> Result<(), String> {
    let compiled = compile_replacement_rules(rules);
    let mut storage = TEXT_REPLACEMENTS.lock().map_err(|e| e.to_string())?;
    *storage = compiled;
    Ok(())
}
