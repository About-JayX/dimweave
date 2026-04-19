//! Spawn Claude CLI subprocess with `--sdk-url` for direct WS connection.

use std::path::PathBuf;
use tokio::process::{Child, Command};

use crate::daemon::task_graph::types::ProviderAuthConfig;

/// Options for launching a Claude subprocess.
#[derive(Clone)]
pub struct ClaudeLaunchOpts {
    /// Path to the `claude` binary (resolved via `which` or explicit).
    pub claude_bin: PathBuf,
    /// Agent role id (e.g. "lead", "coder"). Maps to `--agent dimweave-{role}`.
    pub role: Option<String>,
    /// Working directory for the Claude session.
    pub cwd: String,
    /// UUID session id for new sessions.
    pub session_id: String,
    /// Launch-bound nonce carried in the SDK URL query string.
    pub launch_nonce: String,
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
    /// Optional third-party endpoint override. When `api_key` is set we
    /// inject `ANTHROPIC_API_KEY` (x-api-key mode) or `ANTHROPIC_AUTH_TOKEN`
    /// (Bearer mode, default); when `base_url` is also set we inject
    /// `ANTHROPIC_BASE_URL` on top.
    pub provider_auth: Option<ProviderAuthConfig>,
}

/// Spawn the Claude subprocess.
///
/// The process connects back to the daemon via `--sdk-url ws://127.0.0.1:{port}/claude`.
/// All output is NDJSON on stdout; events are POSTed to `http://127.0.0.1:{port}/claude/events`.
pub fn sdk_ws_url(daemon_port: u16, launch_nonce: Option<&str>) -> String {
    let base = format!("ws://127.0.0.1:{daemon_port}/claude");
    match launch_nonce {
        Some(launch_nonce) => format!("{base}?launch_nonce={launch_nonce}"),
        None => base,
    }
}

pub fn sdk_events_url(daemon_port: u16, launch_nonce: Option<&str>) -> String {
    let base = format!("http://127.0.0.1:{daemon_port}/claude/events");
    match launch_nonce {
        Some(launch_nonce) => format!("{base}?launch_nonce={launch_nonce}"),
        None => base,
    }
}

pub fn redact_launch_nonce(launch_nonce: &str) -> String {
    let visible = launch_nonce.chars().take(8).collect::<String>();
    if launch_nonce.chars().count() <= 8 {
        visible
    } else {
        format!("{visible}...")
    }
}

pub fn format_launch_trace(opts: &ClaudeLaunchOpts) -> String {
    format!(
        "chain=launch ws={} events={} launch_nonce={} bridge=reply/get_online_agents role={} session={} resume={} model={} effort={} cwd={}",
        sdk_ws_url(opts.daemon_port, None),
        sdk_events_url(opts.daemon_port, None),
        redact_launch_nonce(&opts.launch_nonce),
        opts.role.as_deref().unwrap_or("lead"),
        opts.session_id,
        opts.resume.as_deref().unwrap_or("-"),
        opts.model.as_deref().unwrap_or("-"),
        opts.effort.as_deref().unwrap_or("-"),
        opts.cwd,
    )
}

fn build_claude_command(opts: &ClaudeLaunchOpts) -> Command {
    let sdk_url = sdk_ws_url(opts.daemon_port, Some(&opts.launch_nonce));

    let mut cmd = Command::new(&opts.claude_bin);
    cmd.current_dir(&opts.cwd);

    // Required env vars for bridge/SDK mode
    cmd.env("CLAUDE_CODE_ENVIRONMENT_KIND", "bridge");
    cmd.env("CLAUDE_CODE_SESSION_ACCESS_TOKEN", "agentnexus-local");
    cmd.env("CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2", "1");
    cmd.env("CLAUDE_CODE_OAUTH_TOKEN", ""); // clear OAuth
    cmd.env("PATH", crate::claude_cli::enriched_path());

    apply_provider_auth(&mut cmd, opts.provider_auth.as_ref());

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

    // MCP config: --strict-mcp-config flag MUST come before --mcp-config
    // because --mcp-config accepts variadic args and would swallow subsequent flags.
    if let Some(ref mcp_json) = opts.mcp_config {
        cmd.arg("--strict-mcp-config");
        cmd.arg("--mcp-config").arg(mcp_json);
    }

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

/// Translate a `ProviderAuthConfig` into the Claude env vars.
/// `api_key` missing or empty → no-op (subscription/OAuth path unchanged).
pub(crate) fn apply_provider_auth(
    cmd: &mut Command,
    auth: Option<&ProviderAuthConfig>,
) {
    let Some(a) = auth else { return };
    let Some(api_key) = a.api_key.as_deref() else { return };
    if api_key.trim().is_empty() {
        return;
    }
    let mode = a.auth_mode.as_deref().unwrap_or("bearer");
    if mode == "api_key" {
        cmd.env("ANTHROPIC_API_KEY", api_key);
    } else {
        cmd.env("ANTHROPIC_AUTH_TOKEN", api_key);
    }
    if let Some(base_url) = a.base_url.as_deref() {
        if !base_url.trim().is_empty() {
            cmd.env("ANTHROPIC_BASE_URL", base_url);
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
