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

/// Whether we are starting a new session or resuming.
pub enum LaunchMode {
    New,
    Resume,
}

impl ClaudeLaunchOpts {
    pub fn launch_mode(&self) -> LaunchMode {
        if self.resume.is_some() {
            LaunchMode::Resume
        } else {
            LaunchMode::New
        }
    }
}

/// Spawn the Claude subprocess.
///
/// The process connects back to the daemon via `--sdk-url ws://127.0.0.1:{port}/claude`.
/// All output is NDJSON on stdout; events are POSTed to `http://127.0.0.1:{port}/claude/events`.
pub fn spawn_claude(opts: &ClaudeLaunchOpts) -> anyhow::Result<Child> {
    let sdk_url = format!("ws://127.0.0.1:{}/claude", opts.daemon_port);

    let mut cmd = Command::new(&opts.claude_bin);
    cmd.current_dir(&opts.cwd);

    // Required env vars for bridge/SDK mode
    cmd.env("CLAUDE_CODE_ENVIRONMENT_KIND", "bridge");
    cmd.env("CLAUDE_CODE_SESSION_ACCESS_TOKEN", "agentnexus-local");
    cmd.env("CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2", "1");
    cmd.env("CLAUDE_CODE_OAUTH_TOKEN", ""); // clear OAuth

    // CLI arguments
    cmd.arg("--print");
    cmd.arg("--sdk-url").arg(&sdk_url);
    cmd.arg("--input-format").arg("stream-json");
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--replay-user-messages");

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

    // Optional agent role
    if let Some(role) = &opts.role {
        cmd.arg("--agent").arg(format!("nexus-{role}"));
    }

    // Optional model
    if let Some(model) = &opts.model {
        cmd.arg("--model").arg(model);
    }

    // Pipe all stdio
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!(
            "failed to spawn claude at {}: {e}",
            opts.claude_bin.display()
        )
    })?;
    Ok(child)
}
