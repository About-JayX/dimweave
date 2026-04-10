//! Team-membership-based assignee option discovery.
//! Uses `list_project_team` → `list_team_members` → `search_user_info`.

use super::issue_query_parse::extract_first_text;
use super::mcp_client::McpClient;
use serde_json::Value;

/// Fetch team-member names from project team membership APIs.
pub async fn fetch_team_member_names(
    client: &McpClient,
    workspace: &str,
) -> Result<Vec<String>, String> {
    let teams_args = serde_json::json!({"project_key": workspace});
    let teams_result = client.call_tool("list_project_team", teams_args).await?;
    let team_ids = parse_team_ids(&teams_result);
    if team_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut all_user_keys = Vec::new();
    for team_id in &team_ids {
        let mut page_token: Option<String> = None;
        loop {
            let mut args = serde_json::json!({
                "project_key": workspace,
                "team_id": team_id,
            });
            if let Some(ref pt) = page_token {
                args["page_token"] = serde_json::json!(pt);
            }
            let Ok(result) = client.call_tool("list_team_members", args).await else {
                break;
            };
            let (keys, next) = parse_team_members_page(&result);
            all_user_keys.extend(keys);
            match next {
                Some(pt) if !pt.is_empty() => page_token = Some(pt),
                _ => break,
            }
        }
    }
    if all_user_keys.is_empty() {
        return Ok(Vec::new());
    }
    let user_args = serde_json::json!({
        "project_key": workspace,
        "user_keys": all_user_keys,
    });
    let Ok(user_result) = client.call_tool("search_user_info", user_args).await else {
        return Ok(Vec::new());
    };
    Ok(parse_user_names(&user_result))
}

fn parse_team_ids(result: &Value) -> Vec<String> {
    let Some(text) = extract_first_text(result) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let Some(teams) = parsed.get("data").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    teams
        .iter()
        .filter_map(|t| {
            t.get("team_id")
                .or_else(|| t.get("id"))
                .and_then(|v| {
                    v.as_str()
                        .map(String::from)
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
        })
        .collect()
}

fn parse_team_members_page(result: &Value) -> (Vec<String>, Option<String>) {
    let Some(text) = extract_first_text(result) else {
        return (Vec::new(), None);
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        return (Vec::new(), None);
    };
    let keys: Vec<String> = parsed
        .get("data")
        .and_then(|d| d.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|m| {
            m.get("user_key")
                .or_else(|| m.get("user_id"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();
    let next = parsed
        .get("page_token")
        .or_else(|| parsed.get("next_page_token"))
        .and_then(|v| v.as_str())
        .map(String::from);
    (keys, next)
}

fn parse_user_names(result: &Value) -> Vec<String> {
    let Some(text) = extract_first_text(result) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let Some(users) = parsed.get("data").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = users
        .iter()
        .filter_map(|u| {
            u.get("name")
                .or_else(|| u.get("name_cn"))
                .and_then(|n| n.as_str())
                .filter(|n| !n.is_empty())
                .map(String::from)
        })
        .collect();
    names.sort();
    names.dedup();
    names
}
