use serde::{Deserialize, Serialize};
use std::process::Command;

use super::auth::read_access_token;

const LIVE_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_minutes: Option<i64>,
    pub reset_at: Option<i64>,
    pub reset_after_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub source: String,
    pub checked_at: String,
    pub allowed: bool,
    pub limit_reached: bool,
    pub primary: Option<UsageWindow>,
    pub secondary: Option<UsageWindow>,
}

// ── Live fetch ─────────────────────────────────────────────

#[derive(Deserialize)]
struct LiveResponse {
    #[serde(default)]
    rate_limit: Option<LiveRateLimit>,
}

#[derive(Deserialize)]
struct LiveRateLimit {
    #[serde(default)]
    allowed: bool,
    #[serde(default)]
    limit_reached: bool,
    #[serde(default)]
    primary_window: Option<LiveWindow>,
    #[serde(default)]
    secondary_window: Option<LiveWindow>,
}

#[derive(Deserialize)]
struct LiveWindow {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    limit_window_seconds: Option<i64>,
    #[serde(default)]
    reset_at: Option<i64>,
    #[serde(default)]
    reset_after_seconds: Option<i64>,
}

pub async fn fetch_live() -> Result<UsageSnapshot, String> {
    let token = read_access_token().ok_or("no access_token")?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("http client error: {e}"))?;

    let resp = client
        .get(LIVE_USAGE_URL)
        .header("Accept", "application/json")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("usage request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("usage API returned {}", resp.status()));
    }

    let data: LiveResponse = resp.json().await.map_err(|e| format!("parse error: {e}"))?;
    let rl = data.rate_limit.as_ref();

    Ok(UsageSnapshot {
        source: "live".into(),
        checked_at: chrono_now(),
        allowed: rl.map(|r| r.allowed).unwrap_or(false),
        limit_reached: rl.map(|r| r.limit_reached).unwrap_or(false),
        primary: rl.and_then(|r| map_live_window(r.primary_window.as_ref())),
        secondary: rl.and_then(|r| map_live_window(r.secondary_window.as_ref())),
    })
}

// ── Cached from Codex SQLite ───────────────────────────────

pub fn load_cached() -> Result<UsageSnapshot, String> {
    let logs_path = dirs::home_dir()
        .ok_or("cannot resolve home")?
        .join(".codex")
        .join("logs_1.sqlite");

    if !logs_path.exists() {
        return Err("Codex logs not found".into());
    }

    let query = r#"select ts, message from logs where message like 'websocket event: {"type":"codex.rate_limits"%' order by ts desc, ts_nanos desc, id desc limit 1;"#;

    let output = Command::new("sqlite3")
        .arg("-tabs")
        .arg(&logs_path)
        .arg(query)
        .output()
        .map_err(|e| format!("sqlite3 failed: {e}"))?;

    if !output.status.success() {
        return Err("sqlite3 query failed".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let row = stdout.trim();
    if row.is_empty() {
        return Err("no cached rate_limits".into());
    }

    let (ts_str, message) = row.split_once('\t').ok_or("unexpected row format")?;
    let ts: i64 = ts_str.parse().map_err(|_| "invalid timestamp")?;
    let json_start = message.find('{').ok_or("no JSON in log")?;

    let cached: serde_json::Value =
        serde_json::from_str(&message[json_start..]).map_err(|e| format!("parse error: {e}"))?;

    let rl = cached.get("rate_limits");

    Ok(UsageSnapshot {
        source: "cached".into(),
        checked_at: format_unix_ts(ts),
        allowed: rl
            .and_then(|v| v.get("allowed"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        limit_reached: rl
            .and_then(|v| v.get("limit_reached"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        primary: rl
            .and_then(|v| v.get("primary"))
            .and_then(map_cached_window),
        secondary: rl
            .and_then(|v| v.get("secondary"))
            .and_then(map_cached_window),
    })
}

/// Try live first, fall back to cached.
pub async fn get_snapshot() -> Result<UsageSnapshot, String> {
    match fetch_live().await {
        Ok(snap) => Ok(snap),
        Err(_) => load_cached(),
    }
}

// ── Helpers ────────────────────────────────────────────────

fn map_live_window(w: Option<&LiveWindow>) -> Option<UsageWindow> {
    w.map(|w| {
        let used = w.used_percent.unwrap_or(0.0);
        UsageWindow {
            used_percent: used,
            remaining_percent: (100.0 - used).max(0.0),
            window_minutes: w.limit_window_seconds.map(|s| s / 60),
            reset_at: w.reset_at,
            reset_after_seconds: w.reset_after_seconds,
        }
    })
}

fn map_cached_window(v: &serde_json::Value) -> Option<UsageWindow> {
    let used = v.get("used_percent")?.as_f64().unwrap_or(0.0);
    Some(UsageWindow {
        used_percent: used,
        remaining_percent: (100.0 - used).max(0.0),
        window_minutes: v.get("window_minutes").and_then(|v| v.as_i64()),
        reset_at: v.get("reset_at").and_then(|v| v.as_i64()),
        reset_after_seconds: v.get("reset_after_seconds").and_then(|v| v.as_i64()),
    })
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn format_unix_ts(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| format!("{ts}"))
}
