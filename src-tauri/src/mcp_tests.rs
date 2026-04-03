use super::{build_inline_mcp_config, build_project_mcp_config, upsert_mcp_server};

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
        upsert_mcp_server(config.clone(), "/tmp/bridge", &["--foo"], "lead", &[]).unwrap();
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

    let (next, changed) = upsert_mcp_server(config, "/tmp/new", &[], "lead", &[]).unwrap();
    assert!(changed);
    assert_eq!(next["mcpServers"]["agentnexus"]["command"], "/tmp/new");
}

#[test]
fn upsert_mcp_server_marks_changed_when_role_differs() {
    let config = serde_json::json!({
        "mcpServers": {
            "agentnexus": {
                "command": "/tmp/bridge",
                "env": { "AGENTBRIDGE_ROLE": "lead" }
            }
        }
    });

    let (next, changed) = upsert_mcp_server(config, "/tmp/bridge", &[], "reviewer", &[]).unwrap();
    assert!(changed);
    assert_eq!(
        next["mcpServers"]["agentnexus"]["env"]["AGENTBRIDGE_ROLE"],
        "reviewer"
    );
}

#[test]
fn build_inline_mcp_config_serializes_dimweave_server() {
    let raw = build_inline_mcp_config("/tmp/dimweave-bridge", "reviewer").unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        value["mcpServers"]["agentnexus"]["command"],
        "/tmp/dimweave-bridge"
    );
    assert_eq!(
        value["mcpServers"]["agentnexus"]["env"]["AGENTBRIDGE_ROLE"],
        "reviewer"
    );
}

#[test]
fn build_project_mcp_config_preserves_existing_servers() {
    let temp = std::env::temp_dir().join(format!("dimweave-mcp-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).unwrap();
    let path = temp.join(".mcp.json");
    std::fs::write(
        &path,
        serde_json::json!({
            "mcpServers": {
                "other": {
                    "command": "/tmp/other-bridge"
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let raw =
        build_project_mcp_config(temp.to_str().unwrap(), "/tmp/dimweave-bridge", "lead")
            .unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["mcpServers"]["other"]["command"], "/tmp/other-bridge");
    assert_eq!(
        value["mcpServers"]["agentnexus"]["env"]["AGENTBRIDGE_ROLE"],
        "lead"
    );
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn upsert_mcp_server_can_add_sdk_mode_env() {
    let config = serde_json::json!({});
    let (next, changed) = upsert_mcp_server(
        config,
        "/tmp/bridge",
        &[],
        "lead",
        &[("AGENTBRIDGE_SDK_MODE", "1")],
    )
    .unwrap();

    assert!(changed);
    assert_eq!(
        next["mcpServers"]["agentnexus"]["env"]["AGENTBRIDGE_SDK_MODE"],
        "1"
    );
}
