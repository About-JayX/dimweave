use std::path::Path;
use tokio::process::{Child, Command};

/// Spawn a `codex app-server --listen ws://127.0.0.1:{port}` process.
/// `codex_home` is set as the `CODEX_HOME` environment variable.
pub async fn start(port: u16, codex_home: &Path, cwd: &str) -> anyhow::Result<Child> {
    let codex_bin = which::which("codex").unwrap_or_else(|_| "codex".into());

    let child = Command::new(&codex_bin)
        .arg("app-server")
        .arg("--listen")
        .arg(format!("ws://127.0.0.1:{port}"))
        .env("CODEX_HOME", codex_home)
        .current_dir(cwd)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn codex: {e}"))?;

    Ok(child)
}

/// Kill the Codex process, waiting up to 3 s before forcing SIGKILL.
pub async fn stop(child: &mut Child) {
    // Request kill (SIGKILL on Unix, TerminateProcess on Windows)
    child.start_kill().ok();
    tokio::select! {
        _ = child.wait() => {}
        _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
            child.kill().await.ok();
        }
    }
}
