//! Minimal Gemini REST client with structured JSON output and retry.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MODEL: &str = "gemini-3.1-flash-lite";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiAction {
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub estimated_bytes: Option<u64>,
    pub risk: String,
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiReport {
    pub headline: String,
    pub summary: String,
    pub health: String,
    pub actions: Vec<AiAction>,
    #[serde(default)]
    pub observations: Vec<String>,
}

fn response_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "headline": { "type": "STRING", "description": "One punchy sentence: the main takeaway." },
            "summary": { "type": "STRING", "description": "2-4 sentences interpreting the data." },
            "health": { "type": "STRING", "enum": ["good", "attention", "critical"] },
            "actions": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "title": { "type": "STRING" },
                        "detail": { "type": "STRING" },
                        "target": { "type": "STRING", "description": "A path or token exactly as it appears in the digest, if applicable." },
                        "estimatedBytes": { "type": "INTEGER" },
                        "risk": { "type": "STRING", "enum": ["safe", "caution", "expert"] },
                        "kind": { "type": "STRING", "enum": ["advisor", "duplicates", "manual"] }
                    },
                    "required": ["title", "detail", "risk", "kind"]
                }
            },
            "observations": { "type": "ARRAY", "items": { "type": "STRING" } }
        },
        "required": ["headline", "summary", "health", "actions"]
    })
}

fn build_prompt(digest_json: &str, language: &str) -> String {
    let lang_line = match language {
        "ru" => "Respond in Russian (русский язык).",
        _ => "Respond in English.",
    };
    format!(
        "You are Diskovery's disk-space analyst. You receive an anonymized statistical digest \
of a Windows disk scan as JSON. Privacy rules of the digest: user folder and file names are \
replaced by tokens like <dir3> or <file7>.mp4 — refer to those tokens EXACTLY as written and \
never invent names for them. Well-known system folders keep real names.\n\
Your job: interpret where the space actually goes, spot anomalies (bloated caches, stale \
projects, duplicate waste, unusual category skew, very old data), and produce a prioritized, \
realistic cleanup plan. Ground every action in the digest numbers; use advisorFindings rule \
ids where they apply (kind=\"advisor\"), duplicates data (kind=\"duplicates\"), and manual \
review suggestions (kind=\"manual\"). Be quantitative and specific. Never recommend deleting \
anything the digest does not justify. {lang_line}\n\nDIGEST:\n{digest_json}"
    )
}

pub async fn analyze(key: &str, digest_json: &str, language: &str) -> Result<AiReport, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={key}"
    );
    let body = json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": build_prompt(digest_json, language) }]
        }],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 4096,
            "responseMimeType": "application/json",
            "responseSchema": response_schema()
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_err = String::new();
    for attempt in 0..3u32 {
        if attempt > 0 {
            tokio_sleep(1500 * u64::from(attempt)).await;
        }
        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    return parse_report(&text);
                }
                last_err = format!("Gemini HTTP {status}: {}", excerpt(&text));
                if !(status.as_u16() == 429 || status.is_server_error()) {
                    break;
                }
            }
            Err(e) => last_err = format!("network: {e}"),
        }
    }
    Err(last_err)
}

async fn tokio_sleep(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

fn excerpt(s: &str) -> String {
    let t: String = s.chars().take(300).collect();
    t
}

fn parse_report(body: &str) -> Result<AiReport, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("bad response json: {e}"))?;
    let text = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| {
            let reason = v["candidates"][0]["finishReason"]
                .as_str()
                .unwrap_or("unknown");
            format!("empty model response (finishReason: {reason})")
        })?;
    serde_json::from_str::<AiReport>(text).map_err(|e| format!("bad report json: {e}"))
}
