use serde::Serialize;

use super::models::oauth_access_token_public as oauth_access_token;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageWindow {
    /// 0.0..1.0 — percentage of the window consumed.
    pub utilization: f64,
    /// Unix epoch seconds when this window resets.
    pub resets_at: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsage {
    pub overall_status: String,
    pub five_hour: Option<ClaudeUsageWindow>,
    pub seven_day: Option<ClaudeUsageWindow>,
}

pub async fn get_usage() -> Result<ClaudeUsage, String> {
    let token = oauth_access_token()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .ok_or("no Claude Code OAuth token or ANTHROPIC_API_KEY found".to_string())?;
    probe_usage(&token).await
}

/// Minimal `POST /v1/messages` call with `max_tokens=1` just to collect
/// the `anthropic-ratelimit-unified-*` response headers. Uses Haiku to
/// keep the cost as low as possible; the text body is discarded.
async fn probe_usage(token: &str) -> Result<ClaudeUsage, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "." }],
        }))
        .send()
        .await
        .map_err(|e| format!("usage probe request: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("usage probe status {status}: {body}"));
    }
    parse_usage_headers(resp.headers())
}

pub(crate) fn parse_usage_headers(headers: &reqwest::header::HeaderMap) -> Result<ClaudeUsage, String> {
    let get_str = |k: &str| -> Option<String> {
        headers.get(k).and_then(|v| v.to_str().ok()).map(str::to_string)
    };
    let get_f64 = |k: &str| -> Option<f64> { get_str(k).and_then(|v| v.parse().ok()) };
    let get_u64 = |k: &str| -> Option<u64> { get_str(k).and_then(|v| v.parse().ok()) };

    let overall_status = get_str("anthropic-ratelimit-unified-status")
        .unwrap_or_else(|| "unknown".to_string());

    let build_window = |util_k: &str, reset_k: &str, status_k: &str| -> Option<ClaudeUsageWindow> {
        let utilization = get_f64(util_k)?;
        Some(ClaudeUsageWindow {
            utilization,
            resets_at: get_u64(reset_k),
            status: get_str(status_k).unwrap_or_else(|| "allowed".to_string()),
        })
    };

    Ok(ClaudeUsage {
        overall_status,
        five_hour: build_window(
            "anthropic-ratelimit-unified-5h-utilization",
            "anthropic-ratelimit-unified-5h-reset",
            "anthropic-ratelimit-unified-5h-status",
        ),
        seven_day: build_window(
            "anthropic-ratelimit-unified-7d-utilization",
            "anthropic-ratelimit-unified-7d-reset",
            "anthropic-ratelimit-unified-7d-status",
        ),
    })
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
