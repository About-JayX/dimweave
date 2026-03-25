/// MCP registration helpers and related Tauri commands.

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

    let entries = std::fs::read_dir(&resources_dir)
        .map_err(|e| format!("failed to read resources dir {}: {e}", resources_dir.display()))?;

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

/// Register the agentbridge MCP server into the project-local `.mcp.json`.
/// `cwd` is the project directory; falls back to current dir if not provided.
#[tauri::command]
pub fn register_mcp(cwd: Option<String>) -> Result<bool, String> {
    let bridge_cmd = if cfg!(debug_assertions) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root =
            std::path::Path::new(manifest_dir).parent().unwrap_or(std::path::Path::new("."));
        let bridge_bin = project_root.join("target").join("debug").join("agent-bridge-bridge");
        bridge_bin.to_string_lossy().to_string()
    } else {
        resolve_release_bridge_cmd()?
    };
    let project_dir = cwd.unwrap_or_else(|| ".".to_string());
    write_mcp_config(&project_dir, &bridge_cmd, &[])
}

/// Write agentbridge entry into `<project_dir>/.mcp.json` (project-local scope).
fn write_mcp_config(project_dir: &str, command: &str, args: &[&str]) -> Result<bool, String> {
    let mcp_path = std::path::Path::new(project_dir).join(".mcp.json");

    let mut config: serde_json::Value = if mcp_path.exists() {
        let raw = std::fs::read_to_string(&mcp_path).map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

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

    let json =
        serde_json::to_string_pretty(&config).map_err(|e| format!("serialize error: {e}"))?;
    std::fs::write(&mcp_path, json).map_err(|e| format!("write error: {e}"))?;
    Ok(true)
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

#[tauri::command]
pub fn launch_claude_terminal(cwd: Option<String>) -> Result<(), String> {
    let dir = cwd.unwrap_or_else(|| ".".to_string());

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "Terminal"
                activate
                do script "cd '{}' && claude"
            end tell"#,
            dir.replace("'", "'\\''")
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .map_err(|e| format!("failed: {e}"))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("cd '{}' && claude", dir))
            .spawn()
            .map_err(|e| format!("failed: {e}"))?;
    }

    Ok(())
}
