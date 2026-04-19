use serde::{Deserialize, Serialize};

use super::models::oauth_access_token_public as oauth_access_token;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeProfile {
    pub email: String,
    pub display_name: String,
    pub subscription_tier: String,
    pub rate_limit_tier: String,
    pub organization_name: String,
    pub subscription_status: String,
}

#[derive(Deserialize)]
struct RawProfile {
    account: RawAccount,
    organization: RawOrg,
}

#[derive(Deserialize)]
struct RawAccount {
    #[serde(default)]
    email: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    has_claude_max: bool,
    #[serde(default)]
    has_claude_pro: bool,
}

#[derive(Deserialize)]
struct RawOrg {
    #[serde(default)]
    name: String,
    #[serde(default)]
    rate_limit_tier: String,
    #[serde(default)]
    subscription_status: String,
}

pub async fn get_profile() -> Result<ClaudeProfile, String> {
    let token = oauth_access_token()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .ok_or("no Claude Code OAuth token or ANTHROPIC_API_KEY found".to_string())?;
    fetch_profile(&token).await
}

async fn fetch_profile(token: &str) -> Result<ClaudeProfile, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;
    let resp = client
        .get("https://api.anthropic.com/api/oauth/profile")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("profile request: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("profile api status {status}: {body}"));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| format!("profile read body: {e}"))?;
    parse_profile(&text)
}

pub(crate) fn parse_profile(body: &str) -> Result<ClaudeProfile, String> {
    let raw: RawProfile =
        serde_json::from_str(body).map_err(|e| format!("parse profile json: {e}"))?;
    let subscription_tier = if raw.account.has_claude_max {
        "max"
    } else if raw.account.has_claude_pro {
        "pro"
    } else {
        "free"
    }
    .to_string();
    Ok(ClaudeProfile {
        email: raw.account.email,
        display_name: raw.account.display_name,
        subscription_tier,
        rate_limit_tier: raw.organization.rate_limit_tier,
        organization_name: raw.organization.name,
        subscription_status: raw.organization.subscription_status,
    })
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
