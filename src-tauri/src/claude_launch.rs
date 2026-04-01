use crate::claude_cli::{ensure_claude_channel_ready, resolve_claude_bin};
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
    let claude_bin = resolve_claude_bin()?;

    let extra_args = build_launch_args(model.as_deref(), effort.as_deref(), role);

    eprintln!(
        "[MCP] launching Claude channel {version} in managed PTY model={model:?} effort={effort:?} role={role}"
    );
    let emit_debug_logs = cfg!(debug_assertions);
    claude_session::launch(
        session,
        dir,
        &claude_bin,
        &extra_args,
        cols,
        rows,
        app,
        emit_debug_logs,
    )
    .await
}

fn build_launch_args(model: Option<&str>, effort: Option<&str>, role: &str) -> Vec<String> {
    let mut extra_args: Vec<String> = Vec::new();
    if let Some(m) = model {
        if !m.is_empty() {
            extra_args.push("--model".into());
            extra_args.push(m.to_string());
        }
    }
    if let Some(e) = effort {
        if !e.is_empty() {
            extra_args.push("--effort".into());
            extra_args.push(e.to_string());
        }
    }

    extra_args.push("--system-prompt".into());
    extra_args.push(role_config::claude_system_prompt(role));
    extra_args.push("--append-system-prompt".into());
    extra_args.push(role_config::claude_append_system_prompt(role));
    extra_args
}

#[cfg(test)]
mod tests {
    use super::build_launch_args;

    #[test]
    fn launch_args_use_system_and_append_prompt_layers() {
        let args = build_launch_args(Some("sonnet"), Some("high"), "coder");
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--system-prompt" && !w[1].is_empty()));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--append-system-prompt" && !w[1].is_empty()));
    }

    #[test]
    fn launch_args_preserve_optional_model_and_effort() {
        let args = build_launch_args(Some("sonnet"), Some("high"), "lead");
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--model" && w[1] == "sonnet"));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--effort" && w[1] == "high"));
    }
}
