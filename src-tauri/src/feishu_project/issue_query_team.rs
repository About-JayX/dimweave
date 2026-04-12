//! Team-membership-based assignee option discovery.
//! Uses `list_project_team` → select single team → `list_team_members` → `search_user_info`.

use super::issue_query_parse::extract_first_text;
use super::mcp_client::McpClient;
use serde_json::Value;

/// Fetch team-member display names for the current workspace.
///
/// Flow (3 MCP tool calls):
/// 1. `list_project_team` → parse team names + ids
/// 2. Select the single team matching `project_name` suffix
/// 3. `list_team_members(page_size=200)` → user_keys
/// 4. `search_user_info(user_keys)` → display names
pub async fn fetch_team_member_names(
    client: &McpClient,
    workspace: &str,
    project_name: Option<&str>,
) -> Result<Vec<String>, String> {
    let teams_args = serde_json::json!({"project_key": workspace});
    let teams_result = client.call_tool("list_project_team", teams_args).await?;
    let teams = parse_teams(&teams_result);
    let team_id = match select_team(&teams, project_name) {
        Some(id) => id,
        None => return Ok(Vec::new()),
    };
    let args = serde_json::json!({
        "project_key": workspace,
        "team_id": team_id,
        "page_size": 200,
    });
    let result = client.call_tool("list_team_members", args).await?;
    let user_keys = parse_team_members(&result);
    if user_keys.is_empty() {
        return Ok(Vec::new());
    }
    let user_args = serde_json::json!({
        "project_key": workspace,
        "user_keys": user_keys,
    });
    let Ok(user_result) = client.call_tool("search_user_info", user_args).await else {
        return Ok(Vec::new());
    };
    Ok(parse_user_names(&user_result))
}

/// Parse `list_project_team` response into (name, id) pairs.
fn parse_teams(result: &Value) -> Vec<(String, String)> {
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
            let name = t.get("team_name")
                .or_else(|| t.get("name"))
                .and_then(|v| v.as_str())?.to_string();
            let id = t
                .get("team_id")
                .or_else(|| t.get("id"))
                .and_then(|v| {
                    v.as_str()
                        .map(String::from)
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })?;
            Some((name, id))
        })
        .collect()
}

/// Select the team whose name starts with the project name suffix (after `--`).
fn select_team(teams: &[(String, String)], project_name: Option<&str>) -> Option<String> {
    let suffix = project_name?
        .rsplit_once("--")
        .map(|(_, s)| s)?;
    teams
        .iter()
        .find(|(name, _)| name.starts_with(suffix))
        .map(|(_, id)| id.clone())
}

/// Parse `list_team_members` response: top-level `members` array.
fn parse_team_members(result: &Value) -> Vec<String> {
    let Some(text) = extract_first_text(result) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    parsed
        .get("members")
        .and_then(|m| m.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|m| {
            // Real payload: members is a string array of user keys
            m.as_str().map(String::from).or_else(|| {
                // Object fallback: {user_key: "..."} or {user_id: "..."}
                m.get("user_key")
                    .or_else(|| m.get("user_id"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
        })
        .collect()
}

/// Parse `search_user_info` response: top-level JSON array.
fn parse_user_names(result: &Value) -> Vec<String> {
    let Some(text) = extract_first_text(result) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let users = match parsed.as_array() {
        Some(arr) => arr,
        None => return Vec::new(),
    };
    let mut names: Vec<String> = users
        .iter()
        .filter_map(|u| {
            u.get("name_cn")
                .or_else(|| u.get("name"))
                .and_then(|n| n.as_str())
                .filter(|n| !n.is_empty())
                .map(String::from)
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn select_team_matches_project_suffix() {
        let teams = vec![
            ("娱乐站--基座".into(), "team_1".into()),
            ("UXD".into(), "team_2".into()),
            ("基座-前端".into(), "team_3".into()),
        ];
        let id = select_team(&teams, Some("极光矩阵--娱乐站"));
        assert_eq!(id, Some("team_1".to_string()));
    }

    #[test]
    fn parse_teams_reads_real_team_name_field() {
        let payload = json!({
            "content": [{"type": "text", "text": r#"{"data":[{"team_id":"7612468934737956047","team_name":"娱乐站--基座"},{"team_id":"123","team_name":"UXD"}]}"#}]
        });
        let teams = parse_teams(&payload);
        assert_eq!(teams.len(), 2);
        assert_eq!(teams[0].0, "娱乐站--基座");
        assert_eq!(teams[0].1, "7612468934737956047");
        assert_eq!(teams[1].0, "UXD");
    }

    #[test]
    fn select_team_returns_none_without_match() {
        let teams = vec![("UXD".into(), "team_2".into())];
        let id = select_team(&teams, Some("极光矩阵--娱乐站"));
        assert_eq!(id, None);
    }

    #[test]
    fn parse_team_members_uses_members_field() {
        let payload = json!({
            "content": [{"type": "text", "text": r#"{"members":[{"user_key":"u1"},{"user_key":"u2"}]}"#}]
        });
        let keys = parse_team_members(&payload);
        assert_eq!(keys, vec!["u1", "u2"]);
    }

    #[test]
    fn parse_team_members_string_array() {
        // Real Feishu payload: members is an array of user key strings, not objects
        let payload = json!({
            "content": [{"type": "text", "text": r#"{"members":["7620253762535378105","7611423493078535394"]}"#}]
        });
        let keys = parse_team_members(&payload);
        assert_eq!(keys, vec!["7620253762535378105", "7611423493078535394"]);
    }

    #[test]
    fn parse_user_names_from_top_level_array() {
        let payload = json!({
            "content": [{"type": "text", "text": r#"[{"name_cn":"橙子"},{"name_cn":"铃铛"}]"#}]
        });
        let names = parse_user_names(&payload);
        assert_eq!(names, vec!["橙子", "铃铛"]);
    }
}
