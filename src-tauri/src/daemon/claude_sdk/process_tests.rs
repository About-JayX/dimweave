use super::{build_claude_command, format_launch_trace, ClaudeLaunchOpts};
use std::path::PathBuf;
#[test]
fn build_claude_command_sets_sdk_args_and_env() {
    let opts = ClaudeLaunchOpts {
        claude_bin: PathBuf::from("/usr/local/bin/claude"),
        role: Some("coder".into()),
        cwd: "/tmp/workspace".into(),
        session_id: "session-123".into(),
        launch_nonce: "nonce-123".into(),
        model: Some("claude-sonnet-4-6".into()),
        effort: Some("high".into()),
        resume: None,
        daemon_port: 4502,
        mcp_config: Some("{\"mcpServers\":{}}".into()),
    };

    let cmd = build_claude_command(&opts);
    let std_cmd = cmd.as_std();
    let args: Vec<String> = std_cmd
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let expected_path = crate::claude_cli::enriched_path();
    let envs: Vec<(String, Option<String>)> = std_cmd
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    assert_eq!(
        std_cmd.get_current_dir(),
        Some(std::path::Path::new("/tmp/workspace"))
    );
    assert!(args.windows(2).any(
        |w| w[0] == "--sdk-url" && w[1] == "ws://127.0.0.1:4502/claude?launch_nonce=nonce-123"
    ));
    assert!(args
        .windows(2)
        .any(|w| w[0] == "--append-system-prompt" && w[1].contains("coder")));
    assert!(args
        .windows(2)
        .any(|w| w[0] == "--model" && w[1] == "claude-sonnet-4-6"));
    assert!(args
        .windows(2)
        .any(|w| w[0] == "--effort" && w[1] == "high"));
    assert!(args
        .windows(2)
        .any(|w| w[0] == "--session-id" && w[1] == "session-123"));
    assert!(args
        .windows(2)
        .any(|w| w[0] == "--mcp-config" && w[1] == "{\"mcpServers\":{}}"));
    assert!(args.contains(&"--strict-mcp-config".to_string()));
    assert!(envs
        .iter()
        .any(|(key, value)| key == "PATH" && value.as_deref() == Some(expected_path.as_str())));
    assert!(envs.iter().any(|(key, value)| {
        key == "CLAUDE_CODE_ENVIRONMENT_KIND" && value.as_deref() == Some("bridge")
    }));
}
#[test]
fn build_claude_command_uses_resume_without_new_session_id() {
    let opts = ClaudeLaunchOpts {
        claude_bin: PathBuf::from("/usr/local/bin/claude"),
        role: None,
        cwd: "/tmp/workspace".into(),
        session_id: "session-ignored".into(),
        launch_nonce: "nonce-456".into(),
        model: None,
        effort: None,
        resume: Some("resume-456".into()),
        daemon_port: 4502,
        mcp_config: None,
    };

    let cmd = build_claude_command(&opts);
    let args: Vec<String> = cmd
        .as_std()
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args
        .windows(2)
        .any(|w| w[0] == "--resume" && w[1] == "resume-456"));
    assert!(!args.iter().any(|arg| arg == "--session-id"));
}

#[test]
fn build_claude_command_includes_stream_and_permission_flags() {
    let opts = ClaudeLaunchOpts {
        claude_bin: PathBuf::from("/usr/local/bin/claude"),
        role: None,
        cwd: "/tmp".into(),
        session_id: "s".into(),
        launch_nonce: "nonce-a".into(),
        model: None,
        effort: None,
        resume: None,
        daemon_port: 4502,
        mcp_config: None,
    };
    let cmd = build_claude_command(&opts);
    let args: Vec<String> = cmd
        .as_std()
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args.contains(&"--verbose".to_string()), "missing --verbose");
    assert!(
        args.contains(&"--include-partial-messages".to_string()),
        "missing --include-partial-messages (required for stream_event)"
    );
    assert!(
        args.contains(&"--dangerously-skip-permissions".to_string()),
        "missing --dangerously-skip-permissions"
    );
    assert!(
        args.contains(&"--replay-user-messages".to_string()),
        "missing --replay-user-messages"
    );
    assert!(
        args.windows(2)
            .any(|w| w[0] == "--input-format" && w[1] == "stream-json"),
        "missing --input-format stream-json"
    );
    assert!(
        args.windows(2)
            .any(|w| w[0] == "--output-format" && w[1] == "stream-json"),
        "missing --output-format stream-json"
    );
}

#[test]
fn build_claude_command_sets_all_required_env_vars() {
    let opts = ClaudeLaunchOpts {
        claude_bin: PathBuf::from("/usr/local/bin/claude"),
        role: None,
        cwd: "/tmp".into(),
        session_id: "s".into(),
        launch_nonce: "nonce-a".into(),
        model: None,
        effort: None,
        resume: None,
        daemon_port: 4502,
        mcp_config: None,
    };
    let cmd = build_claude_command(&opts);
    let envs: Vec<(String, Option<String>)> = cmd
        .as_std()
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into(),
                v.map(|v| v.to_string_lossy().into()),
            )
        })
        .collect();

    let get_env = |name: &str| -> Option<String> {
        envs.iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| v.clone())
    };
    assert_eq!(
        get_env("CLAUDE_CODE_ENVIRONMENT_KIND"),
        Some("bridge".into())
    );
    assert_eq!(
        get_env("CLAUDE_CODE_SESSION_ACCESS_TOKEN"),
        Some("agentnexus-local".into())
    );
    assert_eq!(
        get_env("CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2"),
        Some("1".into())
    );
    assert_eq!(
        get_env("CLAUDE_CODE_OAUTH_TOKEN"),
        Some("".into()),
        "OAuth must be cleared"
    );
}

#[test]
fn build_claude_command_no_role_omits_system_prompt() {
    let opts = ClaudeLaunchOpts {
        claude_bin: PathBuf::from("/usr/local/bin/claude"),
        role: None,
        cwd: "/tmp".into(),
        session_id: "s".into(),
        launch_nonce: "nonce-a".into(),
        model: None,
        effort: None,
        resume: None,
        daemon_port: 4502,
        mcp_config: None,
    };
    let cmd = build_claude_command(&opts);
    let args: Vec<String> = cmd
        .as_std()
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(!args.contains(&"--append-system-prompt".to_string()));
}

#[test]
fn launch_trace_describes_sdk_transport_chain() {
    let opts = ClaudeLaunchOpts {
        claude_bin: PathBuf::from("/usr/local/bin/claude"),
        role: Some("lead".into()),
        cwd: "/tmp/workspace".into(),
        session_id: "session-123".into(),
        launch_nonce: "nonce-123".into(),
        model: Some("claude-sonnet-4-6".into()),
        effort: Some("high".into()),
        resume: None,
        daemon_port: 4502,
        mcp_config: Some("{\"mcpServers\":{}}".into()),
    };
    let trace = format_launch_trace(&opts);
    assert!(trace.contains("chain=launch"));
    assert!(trace.contains("ws://127.0.0.1:4502/claude"));
    assert!(trace.contains("http://127.0.0.1:4502/claude/events"));
    assert!(trace.contains("launch_nonce=nonce-12..."));
    assert!(trace.contains("bridge=reply/get_online_agents"));
    assert!(trace.contains("role=lead"));
    assert!(trace.contains("model=claude-sonnet-4-6"));
    assert!(trace.contains("effort=high"));
}
