use crate::claude_cli::ensure_claude_channel_ready;
use crate::claude_session::{self, ClaudeSessionManager};
use crate::daemon::role_config;
use std::sync::Arc;
use tauri::AppHandle;

/// Core logic for launching Claude Code in channel preview mode via a managed
/// PTY.  Extracted from `mcp.rs` to keep that module under the 200-line limit.
#[allow(clippy::too_many_arguments)]
pub async fn launch(
    dir: &str,
    model: Option<String>,
    effort: Option<String>,
    role: &str,
    cols: Option<u16>,
    rows: Option<u16>,
    session: Arc<ClaudeSessionManager>,
    app: AppHandle,
) -> Result<(), String> {
    let version = ensure_claude_channel_ready()?;
    let claude_bin =
        which::which("claude").map_err(|_| "Claude CLI not found in PATH".to_string())?;

    let mut extra_args: Vec<String> = Vec::new();
    if let Some(m) = &model {
        if !m.is_empty() {
            extra_args.push("--model".into());
            extra_args.push(m.clone());
        }
    }
    if let Some(e) = &effort {
        if !e.is_empty() {
            extra_args.push("--effort".into());
            extra_args.push(e.clone());
        }
    }

    // Inject role-specific system prompt
    let system_prompt = role_config::claude_system_prompt(role);
    extra_args.push("--append-system-prompt".into());
    extra_args.push(system_prompt);

    eprintln!(
        "[MCP] launching Claude channel {version} in managed PTY model={model:?} effort={effort:?} role={role}"
    );
    let emit_debug_logs = cfg!(debug_assertions);
    claude_session::launch(session, dir, &claude_bin, &extra_args, cols, rows, app, emit_debug_logs).await
}
