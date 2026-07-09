use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub gemini_key: Option<String>,
    pub ai_language: Option<String>,
}

#[derive(Default)]
pub struct SettingsState(pub RwLock<Settings>);

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_local_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn persist(app: &AppHandle, s: &Settings) {
    if let Some(p) = settings_path(app) {
        if let Ok(json) = serde_json::to_string_pretty(s) {
            let _ = std::fs::write(p, json);
        }
    }
}

/// Key resolution order: settings → process env → .env files near cwd.
pub fn resolve_gemini_key(state: &State<'_, SettingsState>) -> Option<String> {
    if let Some(k) = &state.0.read().gemini_key {
        if !k.is_empty() {
            return Some(k.clone());
        }
    }
    if let Ok(k) = std::env::var("GOOGLE_GENERATIVE_AI_API_KEY") {
        if !k.is_empty() {
            return Some(k);
        }
    }
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..3 {
        let env_file = dir.join(".env");
        if let Ok(content) = std::fs::read_to_string(&env_file) {
            for line in content.lines() {
                if let Some(v) = line.trim().strip_prefix("GOOGLE_GENERATIVE_AI_API_KEY=") {
                    let v = v.trim().trim_matches('"');
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub has_gemini_key: bool,
    pub key_source: &'static str,
    pub ai_language: String,
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> SettingsDto {
    let s = state.0.read().clone();
    let from_settings = s.gemini_key.as_deref().is_some_and(|k| !k.is_empty());
    let has_env = !from_settings && resolve_env_key_exists();
    SettingsDto {
        has_gemini_key: from_settings || has_env,
        key_source: if from_settings {
            "settings"
        } else if has_env {
            "env"
        } else {
            "none"
        },
        ai_language: s.ai_language.unwrap_or_else(|| "en".into()),
    }
}

fn resolve_env_key_exists() -> bool {
    if std::env::var("GOOGLE_GENERATIVE_AI_API_KEY").is_ok_and(|k| !k.is_empty()) {
        return true;
    }
    let Ok(mut dir) = std::env::current_dir() else {
        return false;
    };
    for _ in 0..3 {
        if let Ok(content) = std::fs::read_to_string(dir.join(".env")) {
            if content.lines().any(|l| {
                l.trim().starts_with("GOOGLE_GENERATIVE_AI_API_KEY=") && l.trim().len() > 29
            }) {
                return true;
            }
        }
        if !dir.pop() {
            break;
        }
    }
    false
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    gemini_key: Option<String>,
    ai_language: Option<String>,
) -> SettingsDto {
    {
        let mut s = state.0.write();
        if let Some(k) = gemini_key {
            s.gemini_key = if k.trim().is_empty() {
                None
            } else {
                Some(k.trim().to_string())
            };
        }
        if let Some(l) = ai_language {
            s.ai_language = Some(l);
        }
        persist(&app, &s);
    }
    get_settings(state)
}
