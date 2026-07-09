pub mod digest;
pub mod gemini;

use crate::dedup::DedupState;
use crate::ipc::ScanState;
use crate::settings::{self, SettingsState};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct AiState {
    pub token_map: RwLock<HashMap<String, String>>,
    pub cached: RwLock<Option<(String, AiAnalysisDto)>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAction {
    #[serde(flatten)]
    pub action: gemini::AiAction,
    /// Token resolved back to the real path — computed locally, never sent.
    pub resolved_target: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiAnalysisDto {
    pub headline: String,
    pub summary: String,
    pub health: String,
    pub actions: Vec<ResolvedAction>,
    pub observations: Vec<String>,
    pub model: String,
    pub approx_tokens: u32,
    pub cached: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestPreview {
    pub json: String,
    pub approx_tokens: u32,
}

fn scan_signature(tree: &crate::scanner::ScanTree) -> String {
    format!(
        "{}:{}:{}",
        tree.root_path.display(),
        tree.bytes,
        tree.files
    )
}

fn resolve_target(target: &Option<String>, map: &HashMap<String, String>) -> Option<String> {
    let t = target.as_deref()?;
    if let Some(real) = map.get(t) {
        return Some(real.clone());
    }
    // find an embedded <dirN>/<fileN> token and resolve that
    let start = t.find('<')?;
    let end = t[start..].find('>')? + start + 1;
    map.get(&t[start..end]).cloned()
}

#[tauri::command]
pub fn ai_digest_preview(
    scan: State<'_, ScanState>,
    dedup: State<'_, DedupState>,
    ai: State<'_, AiState>,
) -> Result<DigestPreview, String> {
    let guard = scan.tree.read();
    let tree = guard.as_ref().ok_or("No scan available")?;
    let bundle = digest::build(tree, &dedup);
    let json = serde_json::to_string_pretty(&bundle.digest).map_err(|e| e.to_string())?;
    *ai.token_map.write() = bundle.token_map;
    Ok(DigestPreview {
        approx_tokens: (json.len() / 4) as u32,
        json,
    })
}

#[tauri::command]
pub async fn ai_analyze(app: AppHandle, force: bool) -> Result<AiAnalysisDto, String> {
    // Sync phase: build digest and gather config; all guards dropped before await.
    let (digest_json, signature, language, key) = {
        let scan = app.state::<ScanState>();
        let dedup = app.state::<DedupState>();
        let ai = app.state::<AiState>();
        let settings_state = app.state::<SettingsState>();

        let guard = scan.tree.read();
        let tree = guard.as_ref().ok_or("No scan available")?;
        let signature = scan_signature(tree);

        if !force {
            if let Some((sig, cached)) = ai.cached.read().as_ref() {
                if *sig == signature {
                    let mut c = cached.clone();
                    c.cached = true;
                    return Ok(c);
                }
            }
        }

        let bundle = digest::build(tree, &dedup);
        let json = serde_json::to_string(&bundle.digest).map_err(|e| e.to_string())?;
        *ai.token_map.write() = bundle.token_map;

        let language = settings_state
            .0
            .read()
            .ai_language
            .clone()
            .unwrap_or_else(|| "en".into());
        let key = settings::resolve_gemini_key(&settings_state)
            .ok_or("No Gemini API key configured — add one in Settings")?;
        (json, signature, language, key)
    };

    let report = gemini::analyze(&key, &digest_json, &language).await?;

    let ai = app.state::<AiState>();
    let map = ai.token_map.read().clone();
    let dto = AiAnalysisDto {
        headline: report.headline,
        summary: report.summary,
        health: report.health,
        actions: report
            .actions
            .into_iter()
            .map(|a| ResolvedAction {
                resolved_target: resolve_target(&a.target, &map),
                action: a,
            })
            .collect(),
        observations: report.observations,
        model: gemini::MODEL.into(),
        approx_tokens: (digest_json.len() / 4) as u32,
        cached: false,
    };
    *ai.cached.write() = Some((signature, dto.clone()));
    Ok(dto)
}
