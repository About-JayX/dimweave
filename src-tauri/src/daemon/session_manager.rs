use std::{collections::HashMap, fs, path::PathBuf};

/// Manages temporary CODEX_HOME directories for Codex sessions.
/// Each session gets `/tmp/agentbridge-<id>/` with auth.json symlinked.
///
/// This struct is a singleton held in `DaemonState`.  Creating per-launch
/// instances would call `cleanup_stale()` on each launch, destroying live sessions.
pub struct SessionManager {
    sessions: HashMap<String, PathBuf>,
    next_id: u64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::cleanup_stale();
        Self { sessions: HashMap::new(), next_id: 0 }
    }

    /// Generate a unique, monotonically increasing session ID.
    pub fn next_session_id(&mut self) -> String {
        self.next_id += 1;
        format!("{}", self.next_id)
    }

    /// Create a session directory and return the CODEX_HOME path.
    /// Writes `config.toml` with the supplied role constraints.
    pub fn create_session(
        &mut self,
        session_id: &str,
        sandbox_mode: &str,
        approval_policy: &str,
    ) -> anyhow::Result<PathBuf> {
        let tmp = PathBuf::from(format!("/tmp/agentbridge-{session_id}"));
        fs::create_dir_all(&tmp)?;

        // Transparent auth pass-through via symlink (no credential copy)
        if let Some(home) = dirs::home_dir() {
            let src = home.join(".codex").join("auth.json");
            let dst = tmp.join("auth.json");
            if src.exists() && !dst.exists() {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&src, &dst).ok();
            }
        }

        // Write config.toml so Codex loads role constraints from CODEX_HOME
        let config_toml = format!(
            "sandbox_mode = \"{sandbox_mode}\"\napproval_policy = \"{approval_policy}\"\n\n[features]\napply_patch_freeform = false\n"
        );
        fs::write(tmp.join("config.toml"), &config_toml)?;

        self.sessions.insert(session_id.to_string(), tmp.clone());
        Ok(tmp)
    }

    /// Remove the session directory for `session_id`.
    pub fn cleanup_session(&mut self, session_id: &str) {
        if let Some(path) = self.sessions.remove(session_id) {
            fs::remove_dir_all(&path).ok();
        }
    }

    /// Remove all managed session directories.
    pub fn cleanup_all(&mut self) {
        for (_, path) in self.sessions.drain() {
            fs::remove_dir_all(&path).ok();
        }
    }

    /// On startup, remove any leftover `/tmp/agentbridge-*` directories.
    fn cleanup_stale() {
        if let Ok(entries) = fs::read_dir("/tmp") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("agentbridge-") {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        self.cleanup_all();
    }
}
