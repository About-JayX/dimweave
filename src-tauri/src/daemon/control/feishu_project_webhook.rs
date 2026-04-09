use crate::daemon::SharedState;
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::AppHandle;

const MAX_SEEN_UUIDS: usize = 500;
static SEEN_UUIDS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WebhookHeader {
    pub token: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    #[serde(default)]
    pub project_key: Option<String>,
    #[serde(default)]
    pub work_item_id: Option<i64>,
    #[serde(default)]
    pub work_item_type_key: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sub_stage: Option<String>,
    #[serde(default)]
    pub updated_by: Option<String>,
    #[serde(default)]
    pub updated_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookBody {
    #[serde(default)]
    pub header: Option<WebhookHeader>,
    #[serde(default)]
    pub event: Option<WebhookEvent>,
    #[serde(default)]
    pub challenge: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default, rename = "type")]
    pub msg_type: Option<String>,
}

type WR = (StatusCode, Json<serde_json::Value>);

// ── Handler ──────────────────────────────────────────────────────────────────

pub async fn webhook_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    Json(body): Json<WebhookBody>,
) -> WR {
    // URL verification challenge (required by Feishu event subscription setup)
    if body.msg_type.as_deref() == Some("url_verification") {
        return handle_challenge(body);
    }
    let header = match body.header {
        Some(h) => h,
        None => return reply(StatusCode::BAD_REQUEST, "missing header"),
    };
    if !verify_token(&header.token) {
        return reply(StatusCode::UNAUTHORIZED, "invalid token");
    }
    if is_duplicate(&header.uuid) {
        return reply(StatusCode::OK, "duplicate");
    }
    if let Some(item) = body.event.and_then(|e| build_item(e, &header.uuid)) {
        tokio::spawn(async move {
            crate::daemon::feishu_project_lifecycle::ingest_webhook_item(&state, &app, item).await;
        });
    }
    reply(StatusCode::OK, "ok")
}

fn handle_challenge(body: WebhookBody) -> WR {
    let token = body.token.as_deref().unwrap_or("");
    if !verify_token(token) {
        return reply(StatusCode::UNAUTHORIZED, "invalid token");
    }
    let challenge = body.challenge.unwrap_or_default();
    (StatusCode::OK, Json(serde_json::json!({ "challenge": challenge })))
}

fn verify_token(incoming: &str) -> bool {
    let cfg = load_cfg();
    !cfg.webhook_token.is_empty() && incoming == cfg.webhook_token
}

fn load_cfg() -> crate::feishu_project::types::FeishuProjectConfig {
    crate::feishu_project::config::default_config_path()
        .and_then(|p| crate::feishu_project::config::load_config(&p))
        .unwrap_or_default()
}

fn build_item(
    ev: WebhookEvent,
    uuid: &str,
) -> Option<crate::feishu_project::types::FeishuProjectInboxItem> {
    let pk = ev.project_key.filter(|s| !s.is_empty())?;
    let wid = ev.work_item_id?.to_string();
    Some(crate::feishu_project::types::FeishuProjectInboxItem {
        record_id: format!("{pk}_{wid}"),
        project_key: pk.clone(),
        work_item_id: wid.clone(),
        work_item_type_key: ev.work_item_type_key.unwrap_or_default(),
        title: ev.name.unwrap_or_default(),
        status_label: ev.sub_stage,
        assignee_label: ev.updated_by,
        updated_at: ev.updated_at.unwrap_or(0),
        source_url: format!("https://project.feishu.cn/{pk}/issues/{wid}"),
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: crate::feishu_project::types::IngressSource::Webhook,
        last_event_uuid: Some(uuid.to_string()),
    })
}

fn reply(status: StatusCode, msg: &str) -> WR {
    (status, Json(serde_json::json!({ "msg": msg })))
}

/// Check-and-mark: returns true if this UUID was already processed.
fn is_duplicate(uuid: &str) -> bool {
    if uuid.is_empty() {
        return false;
    }
    let mut seen = SEEN_UUIDS.lock().unwrap();
    if seen.iter().any(|s| s == uuid) {
        return true;
    }
    if seen.len() >= MAX_SEEN_UUIDS {
        seen.pop_front();
    }
    seen.push_back(uuid.to_string());
    false
}

#[cfg(test)]
#[path = "feishu_project_webhook_tests.rs"]
mod tests;
