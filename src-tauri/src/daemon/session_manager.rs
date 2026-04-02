use std::{collections::HashMap, fs, path::PathBuf};

/// Tracks Dimweave-owned Codex launches while reusing a stable shared
/// `CODEX_HOME` so provider-native thread history remains resumable.
pub struct SessionManager {
    sessions: HashMap<String, PathBuf>,
    next_id: u64,
    codex_home_override: Option<PathBuf>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::cleanup_stale();
        Self {
            sessions: HashMap::new(),
            next_id: 0,
            codex_home_override: None,
        }
    }

    #[cfg(test)]
    pub fn with_codex_home(path: PathBuf) -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 0,
            codex_home_override: Some(path),
        }
    }

    /// Generate a unique, monotonically increasing session ID.
    pub fn next_session_id(&mut self) -> String {
        self.next_id += 1;
        format!("{}", self.next_id)
    }

    /// Create a session directory and return the CODEX_HOME path.
    /// Role constraints are passed at launch time via `--config`, so the
    /// managed CODEX_HOME can stay stable and preserve thread history.
    pub fn create_session(
        &mut self,
        session_id: &str,
        _sandbox_mode: &str,
        _approval_policy: &str,
    ) -> anyhow::Result<PathBuf> {
        let home = self
            .codex_home_override
            .clone()
            .unwrap_or_else(default_codex_home);
        fs::create_dir_all(&home)?;
        self.sessions.insert(session_id.to_string(), home.clone());
        Ok(home)
    }

    /// Drop launch bookkeeping for `session_id`. The shared history home is
    /// intentionally preserved so Codex threads remain listable/resumable.
    pub fn cleanup_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Drop all launch bookkeeping while keeping the shared home intact.
    pub fn cleanup_all(&mut self) {
        self.sessions.clear();
    }

    /// On startup, remove any leftover directories for the current PID.
    fn cleanup_stale() {
        let prefix = format!("agentnexus-{}-", std::process::id());
        if let Ok(entries) = fs::read_dir("/tmp") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&prefix) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }
    }
}

fn default_codex_home() -> PathBuf {
    if let Ok(path) = std::env::var("CODEX_HOME") {
        return PathBuf::from(path);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
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

#[cfg(test)]
mod tests {
    use super::SessionManager;

    #[test]
    fn create_session_reuses_stable_codex_home() {
        let root =
            std::env::temp_dir().join(format!("agentnexus-codex-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let mut mgr = SessionManager::with_codex_home(root.clone());
        let first = mgr.create_session("1", "workspace-write", "never").unwrap();
        let second = mgr.create_session("2", "workspace-write", "never").unwrap();

        assert_eq!(first, root);
        assert_eq!(second, root);

        mgr.cleanup_session("1");
        assert!(root.exists(), "stable CODEX_HOME must not be deleted");

        let _ = std::fs::remove_dir_all(&root);
    }
}
