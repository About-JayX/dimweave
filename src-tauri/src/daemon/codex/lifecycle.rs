use std::path::{Path, PathBuf};
use tokio::process::{Child, Command};

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
pub async fn start(
    port: u16,
    codex_home: &Path,
    cwd: &str,
    sandbox_mode: &str,
    approval_policy: &str,
) -> anyhow::Result<Child> {
    let codex_bin = resolve_codex_bin();
    let path = crate::claude_cli::enriched_path();
    eprintln!("[Codex] using binary: {}", codex_bin.display());

    let child = Command::new(&codex_bin)
        .arg("app-server")
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
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn codex: {e}"))?;

    Ok(child)
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
