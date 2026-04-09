//! **DEPRECATED** — Legacy Feishu Project OpenAPI polling client.
//!
//! This module used the old plugin-token / user-key REST API path.
//! The active data path is now `mcp_sync.rs` via direct HTTP MCP.
//! Kept only for backwards-compatible deserialization of persisted
//! config/store data and for existing test coverage of pagination logic.

use super::types::{FeishuProjectConfig, FeishuProjectInboxItem, IngressSource};
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://project.feishu.cn";
const MAX_PAGE_SIZE: u32 = 50;
const FILTER_MAX_RESULTS: u64 = 2000;

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub err_code: i64,
    #[serde(default)]
    pub err_msg: Option<String>,
    pub data: Option<T>,
    #[serde(default)]
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub total: u64,
}

#[derive(Debug, Deserialize)]
pub struct WorkItemTypeInfo {
    pub type_key: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkItemRaw {
    pub id: i64,
    pub name: String,
    pub work_item_type_key: String,
    #[serde(default)]
    pub sub_stage: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub updated_by: Option<String>,
    #[serde(default)]
    pub updated_at: Option<u64>,
    #[serde(default)]
    pub simple_name: Option<String>,
}

// ── API functions ────────────────────────────────────────────────────────────

pub async fn list_work_item_type_keys(
    client: &Client,
    cfg: &FeishuProjectConfig,
) -> anyhow::Result<Vec<String>> {
    let url = format!("{BASE_URL}/open_api/{}/work_item/all-types", cfg.project_key);
    let resp: ApiResponse<Vec<WorkItemTypeInfo>> = client
        .get(&url)
        .header("X-PLUGIN-TOKEN", &cfg.plugin_token)
        .header("X-USER-KEY", &cfg.user_key)
        .send()
        .await?
        .json()
        .await?;
    if resp.err_code != 0 {
        anyhow::bail!(
            "list types err_code={}: {}",
            resp.err_code,
            resp.err_msg.unwrap_or_default()
        );
    }
    Ok(resp
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.type_key)
        .collect())
}

pub async fn fetch_work_items_page(
    client: &Client,
    cfg: &FeishuProjectConfig,
    type_keys: &[String],
    page_num: u32,
) -> anyhow::Result<(Vec<WorkItemRaw>, u64)> {
    let url = format!(
        "{BASE_URL}/open_api/{}/work_item/filter",
        cfg.project_key
    );
    let body = serde_json::json!({
        "work_item_type_keys": type_keys,
        "page_num": page_num,
        "page_size": MAX_PAGE_SIZE,
    });
    let resp: ApiResponse<Vec<WorkItemRaw>> = client
        .post(&url)
        .header("X-PLUGIN-TOKEN", &cfg.plugin_token)
        .header("X-USER-KEY", &cfg.user_key)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if resp.err_code != 0 {
        anyhow::bail!(
            "filter err_code={}: {}",
            resp.err_code,
            resp.err_msg.unwrap_or_default()
        );
    }
    let total = resp.pagination.map(|p| p.total).unwrap_or(0);
    let items = resp.data.unwrap_or_default();
    Ok((items, total))
}

/// Result of a full poll cycle.
pub struct PollResult {
    pub items: Vec<FeishuProjectInboxItem>,
    /// True if the API reported more items than the filter endpoint can return.
    pub truncated: bool,
    /// The total reported by the API (may exceed items.len() if truncated).
    pub api_total: u64,
}

/// Fetch all work items from a project space using paginated polling.
pub async fn poll_all_work_items(
    client: &Client,
    cfg: &FeishuProjectConfig,
) -> anyhow::Result<PollResult> {
    let type_keys = list_work_item_type_keys(client, cfg).await?;
    if type_keys.is_empty() {
        return Ok(PollResult { items: Vec::new(), truncated: false, api_total: 0 });
    }
    let mut all_items = Vec::new();
    let mut page_num: u32 = 1;
    let mut last_total: u64 = 0;
    loop {
        let (raw_items, total) =
            fetch_work_items_page(client, cfg, &type_keys, page_num).await?;
        last_total = total;
        let page_empty = raw_items.is_empty();
        for raw in raw_items {
            all_items.push(raw_to_inbox_item(raw, &cfg.project_key));
        }
        if !should_continue_paging(page_num, MAX_PAGE_SIZE, total, page_empty) {
            break;
        }
        page_num += 1;
    }
    let truncated = last_total > FILTER_MAX_RESULTS;
    Ok(PollResult { items: all_items, truncated, api_total: last_total })
}

/// Pure pagination termination logic, extracted for testability.
pub fn should_continue_paging(page_num: u32, page_size: u32, total: u64, page_empty: bool) -> bool {
    if page_empty {
        return false;
    }
    let fetched = (page_num as u64) * (page_size as u64);
    fetched < total
}

fn raw_to_inbox_item(raw: WorkItemRaw, project_key: &str) -> FeishuProjectInboxItem {
    let wid = raw.id.to_string();
    let space = raw.simple_name.as_deref().unwrap_or(project_key);
    FeishuProjectInboxItem {
        record_id: format!("{project_key}_{wid}"),
        project_key: project_key.to_string(),
        work_item_id: wid.clone(),
        work_item_type_key: raw.work_item_type_key,
        title: raw.name,
        status_label: raw.sub_stage,
        assignee_label: raw.updated_by.or(raw.created_by),
        updated_at: raw.updated_at.unwrap_or(0),
        source_url: format!("https://project.feishu.cn/{space}/issues/{wid}"),
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Poll,
        last_event_uuid: None,
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
