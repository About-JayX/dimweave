/// MCP registration helpers and related Tauri commands.
use crate::DaemonSender;
use tauri::State;

fn resolve_release_bridge_cmd() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe.parent().unwrap_or(std::path::Path::new("."));

    // Tauri 2 puts externalBin in Contents/MacOS/ (same dir as main binary)
    let candidate = exe_dir.join("dimweave-bridge");
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }
    // Fallback: Contents/Resources/
    let resources = exe_dir.join("../Resources/dimweave-bridge");
    if resources.exists() {
        return Ok(resources.to_string_lossy().to_string());
    }
    // Scan both dirs for prefixed name (e.g. with target triple suffix)
    for dir in &[exe_dir.to_path_buf(), exe_dir.join("../Resources")] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("dimweave-bridge") {
                    return Ok(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    Err(format!(
        "dimweave-bridge not found near {}",
        exe_dir.display()
    ))
}

pub(crate) fn resolve_dimweave_bridge_cmd() -> Result<String, String> {
    if cfg!(debug_assertions) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let bridge_bin = project_root
            .join("target")
            .join("debug")
            .join("dimweave-bridge");
        Ok(bridge_bin.to_string_lossy().to_string())
    } else {
        resolve_release_bridge_cmd()
    }
}

fn read_mcp_config(project_dir: &str) -> Result<serde_json::Value, String> {
    let mcp_path = std::path::Path::new(project_dir).join(".mcp.json");
    if !mcp_path.exists() {
        return Ok(serde_json::json!({}));
    }
    let raw = std::fs::read_to_string(&mcp_path).map_err(|e| format!("read error: {e}"))?;
    Ok(serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({})))
}

pub(crate) fn build_inline_mcp_config(command: &str, role: &str) -> Result<String, String> {
    let (config, _) = upsert_mcp_server(serde_json::json!({}), command, &[], role, &[])?;
    serde_json::to_string(&config).map_err(|e| format!("serialize error: {e}"))
}

pub(crate) fn build_project_mcp_config(
    project_dir: &str,
    command: &str,
    role: &str,
) -> Result<String, String> {
    let base = read_mcp_config(project_dir)?;
    let (config, _) = upsert_mcp_server(base, command, &[], role, &[])?;
    serde_json::to_string(&config).map_err(|e| format!("serialize error: {e}"))
}

pub(crate) fn build_dimweave_mcp_config(project_dir: &str, role: &str) -> Result<String, String> {
    let command = resolve_dimweave_bridge_cmd()?;
    let base = read_mcp_config(project_dir)?;
    let (config, _) =
        upsert_mcp_server(base, &command, &[], role, &[("AGENTBRIDGE_SDK_MODE", "1")])?;
    serde_json::to_string(&config).map_err(|e| format!("serialize error: {e}"))
}

#[tauri::command]
pub async fn register_mcp(
    cwd: Option<String>,
    daemon_tx: State<'_, DaemonSender>,
) -> Result<bool, String> {
    let bridge_cmd = resolve_dimweave_bridge_cmd()?;
    let project_dir = cwd.unwrap_or_else(|| ".".to_string());
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    daemon_tx
        .0
        .send(crate::daemon::DaemonCmd::ReadClaudeRole { reply: reply_tx })
        .await
        .map_err(|_| "daemon channel closed".to_string())?;
    let role = reply_rx
        .await
        .map_err(|_| "daemon did not reply".to_string())?;
    eprintln!(
        "[MCP] register dimweave in {project_dir} using absolute command {} role={role}",
        bridge_cmd,
    );
    write_mcp_config(&project_dir, &bridge_cmd, &[], &role)
}

fn write_mcp_config(
    project_dir: &str,
    command: &str,
    args: &[&str],
    role: &str,
) -> Result<bool, String> {
    let mcp_path = std::path::Path::new(project_dir).join(".mcp.json");
    let config = read_mcp_config(project_dir)?;

    let (config, changed) = upsert_mcp_server(config, command, args, role, &[])?;
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
    extra_env: &[(&str, &str)],
) -> Result<(serde_json::Value, bool), String> {
    let before = config.clone();

    let servers = config
        .as_object_mut()
        .ok_or("invalid .mcp.json")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let mut env = serde_json::Map::new();
    env.insert("AGENTBRIDGE_ROLE".into(), serde_json::json!(role));
    for (key, value) in extra_env {
        env.insert((*key).into(), serde_json::json!(value));
    }
    let mut entry = serde_json::json!({
        "command": command,
        "env": env
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

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
