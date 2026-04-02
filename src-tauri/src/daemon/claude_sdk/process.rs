//! Spawn Claude CLI subprocess with `--sdk-url` for direct WS connection.

use std::path::PathBuf;
use tokio::process::{Child, Command};

/// Options for launching a Claude subprocess.
pub struct ClaudeLaunchOpts {
    /// Path to the `claude` binary (resolved via `which` or explicit).
    pub claude_bin: PathBuf,
    /// Agent role id (e.g. "lead", "coder"). Maps to `--agent nexus-{role}`.
    pub role: Option<String>,
    /// Working directory for the Claude session.
    pub cwd: String,
    /// UUID session id for new sessions.
    pub session_id: String,
    /// Model override (e.g. "claude-sonnet-4-20250514").
    pub model: Option<String>,
    /// Reasoning effort level.
    pub effort: Option<String>,
    /// If set, resume an existing session instead of starting new.
    pub resume: Option<String>,
    /// Daemon WS port (e.g. 4502).
    pub daemon_port: u16,
    /// MCP config JSON string passed via `--strict-mcp-config`.
    pub mcp_config: Option<String>,
}

/// Spawn the Claude subprocess.
///
/// The process connects back to the daemon via `--sdk-url ws://127.0.0.1:{port}/claude`.
/// All output is NDJSON on stdout; events are POSTed to `http://127.0.0.1:{port}/claude/events`.
pub fn sdk_ws_url(daemon_port: u16) -> String {
    format!("ws://127.0.0.1:{daemon_port}/claude")
}

pub fn sdk_events_url(daemon_port: u16) -> String {
    format!("http://127.0.0.1:{daemon_port}/claude/events")
}

pub fn format_launch_trace(opts: &ClaudeLaunchOpts) -> String {
    format!(
        "chain=launch ws={} events={} bridge=reply/get_online_agents role={} session={} resume={} model={} effort={} cwd={}",
        sdk_ws_url(opts.daemon_port),
        sdk_events_url(opts.daemon_port),
        opts.role.as_deref().unwrap_or("lead"),
        opts.session_id,
        opts.resume.as_deref().unwrap_or("-"),
        opts.model.as_deref().unwrap_or("-"),
        opts.effort.as_deref().unwrap_or("-"),
        opts.cwd,
    )
}

fn build_claude_command(opts: &ClaudeLaunchOpts) -> Command {
    let sdk_url = sdk_ws_url(opts.daemon_port);

    let mut cmd = Command::new(&opts.claude_bin);
    cmd.current_dir(&opts.cwd);

    // Required env vars for bridge/SDK mode
    cmd.env("CLAUDE_CODE_ENVIRONMENT_KIND", "bridge");
    cmd.env("CLAUDE_CODE_SESSION_ACCESS_TOKEN", "agentnexus-local");
    cmd.env("CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2", "1");
    cmd.env("CLAUDE_CODE_OAUTH_TOKEN", ""); // clear OAuth
    cmd.env("PATH", crate::claude_cli::enriched_path());

    // CLI arguments
    cmd.arg("--print");
    cmd.arg("--sdk-url").arg(&sdk_url);
    cmd.arg("--input-format").arg("stream-json");
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--replay-user-messages");
    cmd.arg("--dangerously-skip-permissions");
    cmd.arg("--verbose");
    cmd.arg("--include-partial-messages");

    // Session identity: new or resume
    match &opts.resume {
        Some(resume_id) => {
            cmd.arg("--resume").arg(resume_id);
        }
        None => {
            cmd.arg("--session-id").arg(&opts.session_id);
        }
    }

    // MCP config
    let mcp_json = opts.mcp_config.as_deref().unwrap_or("{}");
    cmd.arg("--strict-mcp-config").arg(mcp_json);

    // Inject role prompt as append (preserves Claude's default system prompt + tool docs).
    // --agent file discovery doesn't work in bridge mode, so we use --append-system-prompt.
    // The prompt uses strong mandatory language to enforce role behavior.
    if let Some(role) = &opts.role {
        let prompt = crate::daemon::role_config::claude_prompt::claude_system_prompt(role);
        cmd.arg("--append-system-prompt").arg(prompt);
    }

    // Optional model
    if let Some(model) = &opts.model {
        cmd.arg("--model").arg(model);
    }

    // Optional reasoning effort
    if let Some(effort) = &opts.effort {
        cmd.arg("--effort").arg(effort);
    }

    // Pipe all stdio
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    cmd
}

pub fn spawn_claude(opts: &ClaudeLaunchOpts) -> anyhow::Result<Child> {
    let mut cmd = build_claude_command(opts);

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!(
            "failed to spawn claude at {}: {e}",
            opts.claude_bin.display()
        )
    })?;
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::{build_claude_command, format_launch_trace, ClaudeLaunchOpts};
    use std::path::PathBuf;

    #[test]
    fn build_claude_command_sets_sdk_args_and_env() {
        let opts = ClaudeLaunchOpts {
            claude_bin: PathBuf::from("/usr/local/bin/claude"),
            role: Some("reviewer".into()),
            cwd: "/tmp/workspace".into(),
            session_id: "session-123".into(),
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

        assert_eq!(std_cmd.get_current_dir(), Some(std::path::Path::new("/tmp/workspace")));
        assert!(args.windows(2).any(|w| w[0] == "--sdk-url" && w[1] == "ws://127.0.0.1:4502/claude"));
        assert!(args.windows(2).any(|w| w[0] == "--append-system-prompt" && w[1].contains("reviewer")));
        assert!(args.windows(2).any(|w| w[0] == "--model" && w[1] == "claude-sonnet-4-6"));
        assert!(args.windows(2).any(|w| w[0] == "--effort" && w[1] == "high"));
        assert!(args.windows(2).any(|w| w[0] == "--session-id" && w[1] == "session-123"));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--strict-mcp-config" && w[1] == "{\"mcpServers\":{}}"));

        assert!(envs.iter().any(|(key, value)| {
            key == "PATH" && value.as_deref() == Some(expected_path.as_str())
        }));
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

        assert!(args.windows(2).any(|w| w[0] == "--resume" && w[1] == "resume-456"));
        assert!(!args.iter().any(|arg| arg == "--session-id"));
    }

    #[test]
    fn build_claude_command_includes_stream_and_permission_flags() {
        let opts = ClaudeLaunchOpts {
            claude_bin: PathBuf::from("/usr/local/bin/claude"),
            role: None,
            cwd: "/tmp".into(),
            session_id: "s".into(),
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
            args.windows(2).any(|w| w[0] == "--input-format" && w[1] == "stream-json"),
            "missing --input-format stream-json"
        );
        assert!(
            args.windows(2).any(|w| w[0] == "--output-format" && w[1] == "stream-json"),
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
            .map(|(k, v)| (k.to_string_lossy().into(), v.map(|v| v.to_string_lossy().into())))
            .collect();

        let get_env = |name: &str| -> Option<String> {
            envs.iter()
                .find(|(k, _)| k == name)
                .and_then(|(_, v)| v.clone())
        };

        assert_eq!(get_env("CLAUDE_CODE_ENVIRONMENT_KIND"), Some("bridge".into()));
        assert_eq!(get_env("CLAUDE_CODE_SESSION_ACCESS_TOKEN"), Some("agentnexus-local".into()));
        assert_eq!(get_env("CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2"), Some("1".into()));
        assert_eq!(get_env("CLAUDE_CODE_OAUTH_TOKEN"), Some("".into()), "OAuth must be cleared");
    }

    #[test]
    fn build_claude_command_no_role_omits_system_prompt() {
        let opts = ClaudeLaunchOpts {
            claude_bin: PathBuf::from("/usr/local/bin/claude"),
            role: None,
            cwd: "/tmp".into(),
            session_id: "s".into(),
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
        assert!(trace.contains("bridge=reply/get_online_agents"));
        assert!(trace.contains("role=lead"));
        assert!(trace.contains("model=claude-sonnet-4-6"));
        assert!(trace.contains("effort=high"));
    }
}
