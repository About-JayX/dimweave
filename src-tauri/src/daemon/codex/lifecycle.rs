use std::path::{Path, PathBuf};
use tokio::process::{Child, Command};

use crate::daemon::task_graph::types::ProviderAuthConfig;

/// Resolve `codex` binary.
/// Priority: bundled sidecar > PATH > common install paths.
fn resolve_codex_bin() -> PathBuf {
    // 1. Bundled sidecar (inside .app/Contents/MacOS/)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join("codex");
            if sidecar.exists() {
                return sidecar;
            }
        }
    }
    // 2. System PATH
    if let Ok(p) = which::which("codex") {
        return p;
    }
    // 3. Common install paths (macOS .app has minimal PATH)
    let home = std::env::var("HOME").unwrap_or_default();
    let nvm_dir = PathBuf::from(&home).join(".nvm/versions/node");
    if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
        let mut versions: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path().join("bin/codex")))
            .filter(|p| p.exists())
            .collect();
        versions.sort();
        if let Some(p) = versions.pop() {
            return p;
        }
    }
    for dir in &[".bun/bin", ".local/bin"] {
        let p = PathBuf::from(&home).join(dir).join("codex");
        if p.exists() {
            return p;
        }
    }
    for p in &["/usr/local/bin/codex", "/opt/homebrew/bin/codex"] {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }
    "codex".into()
}

/// Spawn a `codex app-server --listen ws://127.0.0.1:{port}` process.
///
/// `auth` is the optional third-party endpoint override from the
/// `provider_auth` table. When `api_key + base_url` are set, we append
/// `--config model_providers.<name>.*` entries and inject the matching
/// env var so Codex routes through the custom endpoint. When only
/// `api_key` is set, we inject `OPENAI_API_KEY` as a fallback override.
pub async fn start(
    port: u16,
    codex_home: &Path,
    cwd: &str,
    sandbox_mode: &str,
    approval_policy: &str,
    auth: Option<&ProviderAuthConfig>,
) -> anyhow::Result<Child> {
    let codex_bin = resolve_codex_bin();
    let path = crate::claude_cli::enriched_path();
    eprintln!("[Codex] using binary: {}", codex_bin.display());

    let mut cmd = Command::new(&codex_bin);
    cmd.arg("app-server")
        .arg("--listen")
        .arg(format!("ws://127.0.0.1:{port}"))
        .arg("--config")
        .arg(format!("sandbox_mode=\"{sandbox_mode}\""))
        .arg("--config")
        .arg(format!("approval_policy=\"{approval_policy}\""))
        .arg("--config")
        .arg("features.apply_patch_freeform=false")
        .env("CODEX_HOME", codex_home)
        .env("PATH", &path)
        .current_dir(cwd)
        .kill_on_drop(true);
    // Only inject RUST_LOG when the user explicitly opted into debug tracing
    // via DIMWEAVE_CODEX_DEBUG=1. An empty RUST_LOG string was breaking the
    // Codex subprocess's own tracing init on some builds (banner never prints,
    // no WS listener comes up). Leave inheritance to tokio's default so any
    // RUST_LOG the user set on the parent shell still propagates.
    if std::env::var("DIMWEAVE_CODEX_DEBUG").is_ok()
        && std::env::var("RUST_LOG").is_err()
    {
        cmd.env("RUST_LOG", "codex=debug,reqwest=debug,hyper=info");
    }

    apply_provider_auth(&mut cmd, auth);

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn codex: {e}"))?;

    Ok(child)
}

/// Translate a `ProviderAuthConfig` into the `--config` flags and env
/// variables Codex needs. Extracted so we can unit-test the shape.
///
/// The caller is expected to pass `None` when `active_mode == "subscription"`
/// so this function just has to handle the API-key branch. We defensively
/// short-circuit here too — if the legacy pre-v4 row has `active_mode=None`
/// we still apply based on key presence (backward compat).
pub(crate) fn apply_provider_auth(cmd: &mut Command, auth: Option<&ProviderAuthConfig>) {
    let Some(a) = auth else { return };
    if matches!(a.active_mode.as_deref(), Some("subscription")) {
        return;
    }
    let Some(api_key) = a.api_key.as_deref() else { return };
    if api_key.trim().is_empty() {
        return;
    }
    match a.base_url.as_deref() {
        Some(base_url) if !base_url.trim().is_empty() => {
            let name = a
                .provider_name
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("dimweave-custom");
            let env_key = format!(
                "DIMWEAVE_{}_KEY",
                name.to_uppercase().replace(['-', '.', ' '], "_")
            );
            // Codex app-server 弃用 `wire_api = "chat"`，只接受 `responses`。
            // 空值 / "chat" 均升级到 "responses"；其它值（极少见）透传。
            let raw_wire = a.wire_api.as_deref().unwrap_or("").trim();
            let wire = if raw_wire.is_empty() {
                "responses"
            } else if raw_wire == "chat" {
                eprintln!(
                    "[Codex][auth] wire_api=\"chat\" is deprecated by Codex; \
                     auto-upgrading to \"responses\" (update provider auth \
                     settings to silence this)"
                );
                "responses"
            } else {
                raw_wire
            };
            cmd.arg("--config")
                .arg(format!("model_provider=\"{name}\""))
                .arg("--config")
                // Codex schema requires `model_providers.<name>.name` as a
                // human-readable label; omitting it trips `missing field 'name'`
                // at startup. Reuse the TOML key as the display name.
                .arg(format!("model_providers.{name}.name=\"{name}\""))
                .arg("--config")
                .arg(format!("model_providers.{name}.base_url=\"{base_url}\""))
                .arg("--config")
                .arg(format!("model_providers.{name}.env_key=\"{env_key}\""))
                .arg("--config")
                .arg(format!("model_providers.{name}.wire_api=\"{wire}\""));
            cmd.env(env_key, api_key);
        }
        _ => {
            // Official endpoint with overridden API key.
            cmd.env("OPENAI_API_KEY", api_key);
        }
    }
}

/// Kill the Codex process and wait for it to fully exit.
/// Codex CLI may fork/exec the real app-server, so we also kill the process group.
pub async fn stop(child: &mut Child, port: u16) {
    // Kill the direct child
    child.start_kill().ok();
    tokio::select! {
        _ = child.wait() => {}
        _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
            child.kill().await.ok();
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                child.wait(),
            ).await;
        }
    }
    // Codex may have forked the real app-server (PPID=1 orphan).
    // Kill any process still holding the port.
    kill_port_holder(port).await;
}

pub(super) async fn kill_port_holder(port: u16) {
    let self_pid = std::process::id() as i32;
    let Ok(output) = tokio::process::Command::new("lsof")
        .arg(format!("-ti:{port}"))
        .output()
        .await
    else {
        return;
    };
    let pids = String::from_utf8_lossy(&output.stdout);
    for pid_str in pids.split_whitespace() {
        if let Ok(pid) = pid_str.parse::<i32>() {
            if pid == self_pid || pid <= 1 {
                continue;
            }
            eprintln!("[Codex] killing orphan process {pid} on port {port}");
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}

#[cfg(test)]
mod apply_auth_tests {
    use super::*;
    use tokio::process::Command as TokioCommand;

    fn args(cmd: &TokioCommand) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn envs(cmd: &TokioCommand) -> std::collections::HashMap<String, String> {
        cmd.as_std()
            .get_envs()
            .filter_map(|(k, v)| {
                Some((
                    k.to_string_lossy().into_owned(),
                    v?.to_string_lossy().into_owned(),
                ))
            })
            .collect()
    }

    #[test]
    fn apply_auth_noop_when_absent() {
        let mut cmd = TokioCommand::new("noop");
        apply_provider_auth(&mut cmd, None);
        assert!(args(&cmd).is_empty());
        assert!(envs(&cmd).is_empty());
    }

    #[test]
    fn apply_auth_noop_when_api_key_empty() {
        let mut cmd = TokioCommand::new("noop");
        let cfg = ProviderAuthConfig {
            provider: "codex".into(),
            api_key: Some("   ".into()),
            base_url: None,
            wire_api: None,
            auth_mode: None,
            provider_name: None,
            active_mode: None,
            updated_at: 0,
        };
        apply_provider_auth(&mut cmd, Some(&cfg));
        assert!(args(&cmd).is_empty());
    }

    #[test]
    fn apply_auth_key_only_sets_openai_api_key_env() {
        let mut cmd = TokioCommand::new("noop");
        let cfg = ProviderAuthConfig {
            provider: "codex".into(),
            api_key: Some("sk-test".into()),
            base_url: None,
            wire_api: None,
            auth_mode: None,
            provider_name: None,
            active_mode: None,
            updated_at: 0,
        };
        apply_provider_auth(&mut cmd, Some(&cfg));
        assert_eq!(envs(&cmd).get("OPENAI_API_KEY").map(String::as_str), Some("sk-test"));
        assert!(args(&cmd).is_empty());
    }

    #[test]
    fn apply_auth_with_base_url_emits_model_provider_configs() {
        let mut cmd = TokioCommand::new("noop");
        let cfg = ProviderAuthConfig {
            provider: "codex".into(),
            api_key: Some("sk-or-abc".into()),
            base_url: Some("https://openrouter.ai/api/v1".into()),
            wire_api: Some("chat".into()),
            auth_mode: None,
            provider_name: Some("dimweave-openrouter".into()),
            active_mode: None,
            updated_at: 0,
        };
        apply_provider_auth(&mut cmd, Some(&cfg));
        let args = args(&cmd);
        assert!(args.iter().any(|a| a == "model_provider=\"dimweave-openrouter\""));
        assert!(args
            .iter()
            .any(|a| a == "model_providers.dimweave-openrouter.base_url=\"https://openrouter.ai/api/v1\""));
        assert!(args
            .iter()
            .any(|a| a == "model_providers.dimweave-openrouter.env_key=\"DIMWEAVE_DIMWEAVE_OPENROUTER_KEY\""));
        // Legacy "chat" wire_api is auto-upgraded to "responses" (Codex
        // app-server removed support for "chat").
        assert!(args
            .iter()
            .any(|a| a == "model_providers.dimweave-openrouter.wire_api=\"responses\""));
        assert!(args
            .iter()
            .any(|a| a == "model_providers.dimweave-openrouter.name=\"dimweave-openrouter\""));
        let envs = envs(&cmd);
        assert_eq!(
            envs.get("DIMWEAVE_DIMWEAVE_OPENROUTER_KEY").map(String::as_str),
            Some("sk-or-abc")
        );
        assert!(!envs.contains_key("OPENAI_API_KEY"));
    }

    #[test]
    fn apply_auth_subscription_mode_short_circuits_even_with_key_set() {
        let mut cmd = TokioCommand::new("noop");
        let cfg = ProviderAuthConfig {
            provider: "codex".into(),
            api_key: Some("sk-x".into()),
            base_url: Some("https://example.com/v1".into()),
            wire_api: None,
            auth_mode: None,
            provider_name: None,
            active_mode: Some("subscription".into()),
            updated_at: 0,
        };
        apply_provider_auth(&mut cmd, Some(&cfg));
        assert!(args(&cmd).is_empty());
        let envs = envs(&cmd);
        assert!(!envs.contains_key("OPENAI_API_KEY"));
    }

    #[test]
    fn apply_auth_with_base_url_defaults_to_dimweave_custom_when_name_missing() {
        let mut cmd = TokioCommand::new("noop");
        let cfg = ProviderAuthConfig {
            provider: "codex".into(),
            api_key: Some("sk-x".into()),
            base_url: Some("https://example.com/v1".into()),
            wire_api: None,
            auth_mode: None,
            provider_name: None,
            active_mode: None,
            updated_at: 0,
        };
        apply_provider_auth(&mut cmd, Some(&cfg));
        let args = args(&cmd);
        assert!(args.iter().any(|a| a == "model_provider=\"dimweave-custom\""));
        // Empty wire_api defaults to "responses" (Codex app-server rejects "chat").
        assert!(args
            .iter()
            .any(|a| a == "model_providers.dimweave-custom.wire_api=\"responses\""));
        assert!(args
            .iter()
            .any(|a| a == "model_providers.dimweave-custom.name=\"dimweave-custom\""));
    }
}
