/// MCP registration helpers and related Tauri commands.
use crate::claude_session::ClaudeSessionManager;
use crate::daemon::types::DaemonStatusSnapshot;
use std::sync::Arc;
use tauri::State;

fn resolve_release_bridge_cmd() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe.parent().unwrap_or(std::path::Path::new("."));

    // Tauri 2 puts externalBin in Contents/MacOS/ (same dir as main binary)
    let candidate = exe_dir.join("agent-nexus-bridge");
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }
    // Fallback: Contents/Resources/
    let resources = exe_dir.join("../Resources/agent-nexus-bridge");
    if resources.exists() {
        return Ok(resources.to_string_lossy().to_string());
    }
    // Scan both dirs for prefixed name (e.g. with target triple suffix)
    for dir in &[exe_dir.to_path_buf(), exe_dir.join("../Resources")] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("agent-nexus-bridge") {
                    return Ok(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    Err(format!("agent-nexus-bridge not found near {}", exe_dir.display()))
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
            .join("agent-nexus-bridge");
        bridge_bin.to_string_lossy().to_string()
    } else {
        resolve_release_bridge_cmd()?
    };
    let project_dir = cwd.unwrap_or_else(|| ".".to_string());
    eprintln!(
        "[MCP] register agentnexus in {project_dir} using absolute command {}",
        bridge_cmd
    );
    write_mcp_config(&project_dir, &bridge_cmd, &[], "lead")
}

fn write_mcp_config(
    project_dir: &str,
    command: &str,
    args: &[&str],
    role: &str,
) -> Result<bool, String> {
    let mcp_path = std::path::Path::new(project_dir).join(".mcp.json");

    let config: serde_json::Value = if mcp_path.exists() {
        let raw = std::fs::read_to_string(&mcp_path).map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let (config, changed) = upsert_mcp_server(config, command, args, role)?;
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
    role: &str,
) -> Result<(serde_json::Value, bool), String> {
    let before = config.clone();

    let servers = config
        .as_object_mut()
        .ok_or("invalid .mcp.json")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let mut entry = serde_json::json!({
        "command": command,
        "env": { "AGENTBRIDGE_ROLE": role }
    });
    if !args.is_empty() {
        entry["args"] = serde_json::json!(args);
    }

    servers
        .as_object_mut()
        .ok_or("invalid mcpServers")?
        .insert("agentnexus".to_string(), entry);

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
    config.pointer("/mcpServers/agentnexus").is_some()
}

/// Launch Claude Code channel preview.
/// Runs Claude in a managed hidden PTY so the local development prompt can be
/// auto-confirmed for `server:agentnexus`.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn launch_claude_terminal(
    cwd: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    session: State<'_, Arc<ClaudeSessionManager>>,
    daemon_tx: State<'_, crate::DaemonSender>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = cwd.unwrap_or_else(|| ".".to_string());
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    daemon_tx.0
        .send(crate::daemon::DaemonCmd::ReadClaudeRole { reply: reply_tx })
        .await
        .map_err(|_| "daemon channel closed".to_string())?;
    let role = reply_rx.await.map_err(|_| "daemon did not reply".to_string())?;
    let (snapshot_tx, snapshot_rx) = tokio::sync::oneshot::channel();
    daemon_tx.0
        .send(crate::daemon::DaemonCmd::ReadStatusSnapshot { reply: snapshot_tx })
        .await
        .map_err(|_| "daemon channel closed".to_string())?;
    let snapshot: DaemonStatusSnapshot =
        snapshot_rx.await.map_err(|_| "daemon did not reply".to_string())?;
    let codex_online = snapshot
        .agents
        .iter()
        .any(|agent| agent.agent == "codex" && agent.online);
    if codex_online && snapshot.codex_role == role {
        return Err(format!(
            "role '{role}' already in use by online codex"
        ));
    }
    crate::claude_launch::launch(&dir, model, effort, &role, cols, rows, session.inner().clone(), app).await
}

#[cfg(test)]
mod tests {
    use super::upsert_mcp_server;

    #[test]
    fn upsert_mcp_server_marks_unchanged_when_entry_matches() {
        let config = serde_json::json!({
            "mcpServers": {
                "agentnexus": {
                    "command": "/tmp/bridge",
                    "args": ["--foo"],
                    "env": { "AGENTBRIDGE_ROLE": "lead" }
                }
            }
        });

        let (next, changed) =
            upsert_mcp_server(config.clone(), "/tmp/bridge", &["--foo"], "lead").unwrap();
        assert_eq!(next, config);
        assert!(!changed);
    }

    #[test]
    fn upsert_mcp_server_marks_changed_when_command_differs() {
        let config = serde_json::json!({
            "mcpServers": {
                "agentnexus": {
                    "command": "/tmp/old"
                }
            }
        });

        let (next, changed) = upsert_mcp_server(config, "/tmp/new", &[], "lead").unwrap();
        assert!(changed);
        assert_eq!(next["mcpServers"]["agentnexus"]["command"], "/tmp/new");
    }
}
