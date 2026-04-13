use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

/// Per-task Claude SDK connection state.
pub struct ClaudeTaskSlot {
    pub session_epoch: u64,
    pub pending_nonce: Option<String>,
    pub active_nonce: Option<String>,
    pub ws_generation: u64,
    pub ws_tx: Option<mpsc::Sender<String>>,
    pub event_tx: Option<mpsc::Sender<Vec<serde_json::Value>>>,
    pub ready_tx: Option<oneshot::Sender<mpsc::Sender<String>>>,
    pub preview_buffer: String,
    pub preview_flush_scheduled: bool,
}

impl ClaudeTaskSlot {
    pub fn new() -> Self {
        Self {
            session_epoch: 0,
            pending_nonce: None,
            active_nonce: None,
            ws_generation: 0,
            ws_tx: None,
            event_tx: None,
            ready_tx: None,
            preview_buffer: String::new(),
            preview_flush_scheduled: false,
        }
    }

    pub fn is_online(&self) -> bool {
        self.ws_tx.is_some()
    }
}

/// Per-task Codex app-server connection state.
pub struct CodexTaskSlot {
    pub session_epoch: u64,
    pub inject_tx: Option<mpsc::Sender<(Vec<serde_json::Value>, bool)>>,
    pub port: u16,
    pub connection: Option<crate::daemon::types::ProviderConnectionState>,
}

impl CodexTaskSlot {
    pub fn new(port: u16) -> Self {
        Self {
            session_epoch: 0,
            inject_tx: None,
            port,
            connection: None,
        }
    }

    pub fn is_online(&self) -> bool {
        self.inject_tx.is_some()
    }
}

/// Per-task runtime state. Each active task owns one of these.
pub struct TaskRuntime {
    pub task_id: String,
    pub workspace_root: PathBuf,
    pub claude_slot: Option<ClaudeTaskSlot>,
    pub codex_slot: Option<CodexTaskSlot>,
}

impl TaskRuntime {
    pub fn new(task_id: String, workspace_root: PathBuf) -> Self {
        Self {
            task_id,
            workspace_root,
            claude_slot: None,
            codex_slot: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_runtime_construction() {
        let rt = TaskRuntime::new("task_1".into(), PathBuf::from("/ws/tasks/task_1"));
        assert_eq!(rt.task_id, "task_1");
        assert_eq!(rt.workspace_root, PathBuf::from("/ws/tasks/task_1"));
    }
}
