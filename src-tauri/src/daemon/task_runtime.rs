use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

/// Per-task Claude SDK connection state.
pub struct ClaudeTaskSlot {
    /// Concrete agent identity from TaskAgent.agent_id.
    pub agent_id: Option<String>,
    pub session_epoch: u64,
    pub pending_nonce: Option<String>,
    pub active_nonce: Option<String>,
    pub ws_generation: u64,
    pub ws_tx: Option<mpsc::Sender<String>>,
    pub event_tx: Option<mpsc::Sender<Vec<serde_json::Value>>>,
    pub ready_tx: Option<oneshot::Sender<mpsc::Sender<String>>>,
    pub preview_buffer: String,
    pub preview_flush_scheduled: bool,
    pub connection: Option<crate::daemon::types::ProviderConnectionState>,
}

impl ClaudeTaskSlot {
    pub fn new() -> Self {
        Self {
            agent_id: None,
            session_epoch: 0,
            pending_nonce: None,
            active_nonce: None,
            ws_generation: 0,
            ws_tx: None,
            event_tx: None,
            ready_tx: None,
            preview_buffer: String::new(),
            preview_flush_scheduled: false,
            connection: None,
        }
    }

    pub fn is_online(&self) -> bool {
        self.ws_tx.is_some()
    }
}

/// Per-task Codex app-server connection state.
pub struct CodexTaskSlot {
    /// Concrete agent identity from TaskAgent.agent_id.
    pub agent_id: Option<String>,
    pub session_epoch: u64,
    pub inject_tx: Option<mpsc::Sender<(Vec<serde_json::Value>, bool)>>,
    pub port: u16,
    pub connection: Option<crate::daemon::types::ProviderConnectionState>,
}

impl CodexTaskSlot {
    pub fn new(port: u16) -> Self {
        Self {
            agent_id: None,
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
    /// Default Claude slot (compat for callers not yet on agent_id).
    pub claude_slot: Option<ClaudeTaskSlot>,
    /// Default Codex slot (compat for callers not yet on agent_id).
    pub codex_slot: Option<CodexTaskSlot>,
    /// Additional Claude slots keyed by agent_id (multi-agent).
    pub(crate) extra_claude_slots: HashMap<String, ClaudeTaskSlot>,
    /// Additional Codex slots keyed by agent_id (multi-agent).
    pub(crate) extra_codex_slots: HashMap<String, CodexTaskSlot>,
}

impl TaskRuntime {
    pub fn new(task_id: String, workspace_root: PathBuf) -> Self {
        Self {
            task_id,
            workspace_root,
            claude_slot: None,
            codex_slot: None,
            extra_claude_slots: HashMap::new(),
            extra_codex_slots: HashMap::new(),
        }
    }

    /// Get or create a Claude slot for a given agent_id.
    pub fn get_or_create_claude_slot(&mut self, agent_id: &str) -> &mut ClaudeTaskSlot {
        // Check if default slot belongs to this agent_id (or is unclaimed)
        if self.claude_slot.is_none()
            || self.claude_slot.as_ref().unwrap().agent_id.as_deref() == Some(agent_id)
            || self.claude_slot.as_ref().unwrap().agent_id.is_none()
        {
            let slot = self.claude_slot.get_or_insert_with(ClaudeTaskSlot::new);
            if slot.agent_id.is_none() {
                slot.agent_id = Some(agent_id.to_string());
            }
            return self.claude_slot.as_mut().unwrap();
        }
        self.extra_claude_slots
            .entry(agent_id.to_string())
            .or_insert_with(|| {
                let mut s = ClaudeTaskSlot::new();
                s.agent_id = Some(agent_id.to_string());
                s
            })
    }

    /// Find a Claude slot by agent_id.
    pub fn claude_slot_by_agent(&self, agent_id: &str) -> Option<&ClaudeTaskSlot> {
        if let Some(slot) = &self.claude_slot {
            if slot.agent_id.as_deref() == Some(agent_id) {
                return Some(slot);
            }
        }
        self.extra_claude_slots.get(agent_id)
    }

    /// Find a Claude slot mutably by agent_id.
    pub fn claude_slot_by_agent_mut(
        &mut self,
        agent_id: &str,
    ) -> Option<&mut ClaudeTaskSlot> {
        let is_default = self
            .claude_slot
            .as_ref()
            .and_then(|s| s.agent_id.as_deref())
            == Some(agent_id);
        if is_default {
            return self.claude_slot.as_mut();
        }
        self.extra_claude_slots.get_mut(agent_id)
    }

    /// Get or create a Codex slot for a given agent_id.
    pub fn get_or_create_codex_slot(
        &mut self,
        agent_id: &str,
        port: u16,
    ) -> &mut CodexTaskSlot {
        if self.codex_slot.is_none()
            || self.codex_slot.as_ref().unwrap().agent_id.as_deref() == Some(agent_id)
            || self.codex_slot.as_ref().unwrap().agent_id.is_none()
        {
            let slot = self
                .codex_slot
                .get_or_insert_with(|| CodexTaskSlot::new(port));
            if slot.agent_id.is_none() {
                slot.agent_id = Some(agent_id.to_string());
            }
            return self.codex_slot.as_mut().unwrap();
        }
        self.extra_codex_slots
            .entry(agent_id.to_string())
            .or_insert_with(|| {
                let mut s = CodexTaskSlot::new(port);
                s.agent_id = Some(agent_id.to_string());
                s
            })
    }

    /// Find a Codex slot by agent_id.
    pub fn codex_slot_by_agent(&self, agent_id: &str) -> Option<&CodexTaskSlot> {
        if let Some(slot) = &self.codex_slot {
            if slot.agent_id.as_deref() == Some(agent_id) {
                return Some(slot);
            }
        }
        self.extra_codex_slots.get(agent_id)
    }

    /// Find a Codex slot mutably by agent_id.
    pub fn codex_slot_by_agent_mut(
        &mut self,
        agent_id: &str,
    ) -> Option<&mut CodexTaskSlot> {
        let is_default = self
            .codex_slot
            .as_ref()
            .and_then(|s| s.agent_id.as_deref())
            == Some(agent_id);
        if is_default {
            return self.codex_slot.as_mut();
        }
        self.extra_codex_slots.get_mut(agent_id)
    }

    /// Iterate all Claude slots (default + extras).
    pub fn all_claude_slots(&self) -> impl Iterator<Item = &ClaudeTaskSlot> {
        self.claude_slot.iter().chain(self.extra_claude_slots.values())
    }

    /// Iterate all Codex slots (default + extras).
    pub fn all_codex_slots(&self) -> impl Iterator<Item = &CodexTaskSlot> {
        self.codex_slot.iter().chain(self.extra_codex_slots.values())
    }

    /// Find which agent_id owns a given Claude nonce in this runtime.
    pub fn find_claude_agent_for_nonce(&self, nonce: &str) -> Option<&str> {
        for slot in self.all_claude_slots() {
            if slot.pending_nonce.as_deref() == Some(nonce)
                || slot.active_nonce.as_deref() == Some(nonce)
            {
                return slot.agent_id.as_deref();
            }
        }
        None
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
