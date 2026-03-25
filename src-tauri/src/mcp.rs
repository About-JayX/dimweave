/// MCP registration helpers and related Tauri commands.
use crate::claude_session::ClaudeSessionManager;
use std::sync::Arc;
use tauri::State;

fn resolve_release_bridge_cmd() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let resources_dir = exe
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("../Resources");

    let direct = resources_dir.join("agent-bridge-bridge");
    if direct.exists() {
        return Ok(direct.to_string_lossy().to_string());
    }

    let entries = std::fs::read_dir(&resources_dir).map_err(|e| {
        format!(
            "failed to read resources dir {}: {e}",
            resources_dir.display()
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("agent-bridge-bridge") {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    Err(format!(
        "agent-bridge-bridge not found in {}",
        resources_dir.display()
    ))
}

#[tauri::command]
pub fn register_mcp(cwd: Option<String>) -> Result<bool, String> {
    let bridge_cmd = if cfg!(debug_assertions) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let bridge_bin = project_root
            .join("target")
            .join("debug")
            .join("agent-bridge-bridge");
        bridge_bin.to_string_lossy().to_string()
    } else {
        resolve_release_bridge_cmd()?
    };
    let project_dir = cwd.unwrap_or_else(|| ".".to_string());
    eprintln!(
        "[MCP] register agentbridge in {project_dir} using absolute command {}",
        bridge_cmd
    );
    write_mcp_config(&project_dir, &bridge_cmd, &[])
}

fn write_mcp_config(project_dir: &str, command: &str, args: &[&str]) -> Result<bool, String> {
    let mcp_path = std::path::Path::new(project_dir).join(".mcp.json");

    let config: serde_json::Value = if mcp_path.exists() {
        let raw = std::fs::read_to_string(&mcp_path).map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let (config, changed) = upsert_mcp_server(config, command, args)?;
    if mcp_path.exists() && !changed {
        return Ok(true);
    }

    let json =
        serde_json::to_string_pretty(&config).map_err(|e| format!("serialize error: {e}"))?;
    std::fs::write(&mcp_path, json).map_err(|e| format!("write error: {e}"))?;
    Ok(true)
}

fn upsert_mcp_server(
    mut config: serde_json::Value,
    command: &str,
    args: &[&str],
) -> Result<(serde_json::Value, bool), String> {
    let before = config.clone();

    let servers = config
        .as_object_mut()
        .ok_or("invalid .mcp.json")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let mut entry = serde_json::json!({ "command": command });
    if !args.is_empty() {
        entry["args"] = serde_json::json!(args);
    }

    servers
        .as_object_mut()
        .ok_or("invalid mcpServers")?
        .insert("agentbridge".to_string(), entry);

    Ok((config.clone(), config != before))
}

#[tauri::command]
pub fn check_mcp_registered(cwd: Option<String>) -> bool {
    let project_dir = cwd.unwrap_or_else(|| ".".to_string());
    let mcp_path = std::path::Path::new(&project_dir).join(".mcp.json");
    let raw = match std::fs::read_to_string(mcp_path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let config: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(c) => c,
        Err(_) => return false,
    };
    config.pointer("/mcpServers/agentbridge").is_some()
}

/// Launch Claude Code channel preview.
/// Runs Claude in a managed hidden PTY so the local development prompt can be
/// auto-confirmed for `server:agentbridge`.
#[tauri::command]
pub async fn launch_claude_terminal(
    cwd: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    session: State<'_, Arc<ClaudeSessionManager>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = cwd.unwrap_or_else(|| ".".to_string());
    crate::claude_launch::launch(&dir, model, effort, session.inner().as_ref(), app).await
}

#[cfg(test)]
mod tests {
    use super::upsert_mcp_server;

    #[test]
    fn upsert_mcp_server_marks_unchanged_when_entry_matches() {
        let config = serde_json::json!({
            "mcpServers": {
                "agentbridge": {
                    "command": "/tmp/bridge",
                    "args": ["--foo"]
                }
            }
        });

        let (next, changed) =
            upsert_mcp_server(config.clone(), "/tmp/bridge", &["--foo"]).unwrap();
        assert_eq!(next, config);
        assert!(!changed);
    }

    #[test]
    fn upsert_mcp_server_marks_changed_when_command_differs() {
        let config = serde_json::json!({
            "mcpServers": {
                "agentbridge": {
                    "command": "/tmp/old"
                }
            }
        });

        let (next, changed) = upsert_mcp_server(config, "/tmp/new", &[]).unwrap();
        assert!(changed);
        assert_eq!(next["mcpServers"]["agentbridge"]["command"], "/tmp/new");
    }
}
