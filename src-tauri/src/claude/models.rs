use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeModel {
    pub slug: String,
    pub display_name: String,
    pub supported_efforts: Vec<String>,
}

#[derive(Deserialize)]
struct RawModelsResponse {
    data: Vec<RawModel>,
}

#[derive(Deserialize)]
struct RawModel {
    id: String,
    display_name: String,
    #[serde(default)]
    capabilities: RawCapabilities,
}

#[derive(Deserialize, Default)]
struct RawCapabilities {
    #[serde(default)]
    effort: Option<RawEffort>,
}

#[derive(Deserialize, Default)]
struct RawEffort {
    #[serde(default)]
    low: RawSupport,
    #[serde(default)]
    medium: RawSupport,
    #[serde(default)]
    high: RawSupport,
    #[serde(default)]
    max: RawSupport,
    #[serde(default)]
    xhigh: RawSupport,
}

#[derive(Deserialize, Default)]
struct RawSupport {
    #[serde(default)]
    supported: bool,
}

pub async fn list_models() -> Result<Vec<ClaudeModel>, String> {
    let token = oauth_access_token()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .ok_or("no Claude Code OAuth token or ANTHROPIC_API_KEY found".to_string())?;
    fetch_models(&token).await
}

async fn fetch_models(token: &str) -> Result<Vec<ClaudeModel>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;
    let resp = client
        .get("https://api.anthropic.com/v1/models?limit=50")
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
        .map_err(|e| format!("models api request: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("models api status {status}: {body}"));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| format!("models api read body: {e}"))?;
    parse_models(&text)
}

/// Parse `/v1/models` JSON body into filtered `ClaudeModel` list.
///
/// Legacy `claude-2*` / `claude-3*` ids are dropped; only Claude 4.x+ surfaces.
pub(crate) fn parse_models(body: &str) -> Result<Vec<ClaudeModel>, String> {
    let parsed: RawModelsResponse =
        serde_json::from_str(body).map_err(|e| format!("parse models json: {e}"))?;
    Ok(parsed
        .data
        .into_iter()
        .filter(|m| is_modern_claude_id(&m.id))
        .map(|m| ClaudeModel {
            supported_efforts: collect_supported_efforts(&m.capabilities),
            slug: m.id,
            display_name: m.display_name,
        })
        .collect())
}

fn is_modern_claude_id(id: &str) -> bool {
    // Keep claude-4.x+ (opus/sonnet/haiku), drop claude-2*, claude-3*.
    !(id.starts_with("claude-2") || id.starts_with("claude-3"))
}

fn collect_supported_efforts(caps: &RawCapabilities) -> Vec<String> {
    let Some(eff) = &caps.effort else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if eff.low.supported {
        out.push("low".into());
    }
    if eff.medium.supported {
        out.push("medium".into());
    }
    if eff.high.supported {
        out.push("high".into());
    }
    if eff.xhigh.supported {
        out.push("xhigh".into());
    }
    if eff.max.supported {
        out.push("max".into());
    }
    out
}

/// Public shim — lets sibling modules (e.g. `profile`) share the same auth path.
pub(super) fn oauth_access_token_public() -> Option<String> {
    oauth_access_token()
}

#[cfg(target_os = "macos")]
fn oauth_access_token() -> Option<String> {
    let out = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8(out.stdout).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    parsed
        .get("claudeAiOauth")
        .and_then(|v| v.get("accessToken"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(not(target_os = "macos"))]
fn oauth_access_token() -> Option<String> {
    None
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
