use crate::daemon::{
    orchestrator::review_gate::ReviewGate,
    session_manager::SessionManager,
    task_graph::TaskGraphStore,
    types::{
        AgentRuntimeStatus, BridgeMessage, DaemonStatusSnapshot, OnlineAgentInfo,
        PermissionBehavior, PermissionRequest, PermissionVerdict, ProviderConnectionState, ToAgent,
    },
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, Mutex};

pub const PERMISSION_TTL_MS: u64 = 10 * 60 * 1000;

/// Agent connection with generation ID to prevent stale disconnect from removing new connections.
#[derive(Clone)]
pub struct AgentSender {
    pub tx: mpsc::Sender<ToAgent>,
    pub gen: u64,
}

impl AgentSender {
    pub fn new(tx: mpsc::Sender<ToAgent>, gen: u64) -> Self {
        Self { tx, gen }
    }
}

struct PendingPermission {
    agent_id: String,
    created_at: u64,
    #[allow(dead_code)]
    request: PermissionRequest,
}

pub struct DaemonState {
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
    pending_permissions: HashMap<String, PendingPermission>,
    buffered_verdicts: HashMap<String, Vec<PermissionVerdict>>,
    pub codex_inject_tx: Option<mpsc::Sender<(String, bool)>>,
    codex_session_epoch: u64,
    // Claude SDK connection (hybrid WS + HTTP POST mode)
    pub claude_sdk_ws_tx: Option<mpsc::Sender<String>>,
    /// Oneshot that fires when Claude connects via WS, carrying the inject sender.
    pub claude_sdk_ready_tx: Option<tokio::sync::oneshot::Sender<mpsc::Sender<String>>>,
    claude_sdk_session_epoch: u64,
    pub claude_role: String,
    pub codex_role: String,
    pub claude_connection: Option<ProviderConnectionState>,
    pub codex_connection: Option<ProviderConnectionState>,
    pub session_mgr: Arc<Mutex<SessionManager>>,
    /// Monotonic counter for agent connection generations.
    pub next_agent_gen: u64,
    /// Normalized task/session/artifact graph.
    pub task_graph: TaskGraphStore,
    pub active_task_id: Option<String>,
    pub(crate) review_gate: ReviewGate,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            attached_agents: HashMap::new(),
            buffered_messages: Vec::new(),
            pending_permissions: HashMap::new(),
            buffered_verdicts: HashMap::new(),
            codex_inject_tx: None,
            codex_session_epoch: 0,
            claude_sdk_ws_tx: None,
            claude_sdk_ready_tx: None,
            claude_sdk_session_epoch: 0,
            claude_role: "lead".into(),
            codex_role: "coder".into(),
            claude_connection: None,
            codex_connection: None,
            session_mgr: Arc::new(Mutex::new(SessionManager::new())),
            next_agent_gen: 0,
            task_graph: TaskGraphStore::new(),
            active_task_id: None,
            review_gate: ReviewGate::new(),
        }
    }
}

impl DaemonState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with task graph loaded from the given path.
    pub fn with_task_graph_path(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            task_graph: TaskGraphStore::load(&path)?,
            ..Self::default()
        })
    }

    /// Persist task graph to disk (no-op if no path configured).
    pub fn save_task_graph(&self) -> anyhow::Result<()> {
        self.task_graph.save()
    }

    /// Best-effort auto-save after mutations.
    pub(crate) fn auto_save_task_graph(&self) {
        if let Err(e) = self.task_graph.save() {
            eprintln!("[Daemon] task graph auto-save failed: {e}");
        }
    }

    pub fn begin_codex_launch(&mut self) -> u64 {
        self.codex_session_epoch = self.codex_session_epoch.wrapping_add(1);
        self.codex_session_epoch
    }

    pub fn invalidate_codex_session(&mut self) {
        self.begin_codex_launch();
        self.codex_inject_tx = None;
        self.codex_connection = None;
    }

    pub fn attach_codex_session_if_current(
        &mut self,
        epoch: u64,
        tx: mpsc::Sender<(String, bool)>,
    ) -> bool {
        if self.codex_session_epoch != epoch {
            return false;
        }
        self.codex_inject_tx = Some(tx);
        true
    }

    pub fn clear_codex_session_if_current(&mut self, epoch: u64) -> bool {
        if self.codex_session_epoch != epoch {
            return false;
        }
        self.codex_inject_tx = None;
        self.codex_connection = None;
        true
    }

    // ── Claude SDK session lifecycle ────────────────────────

    pub fn begin_claude_sdk_launch(&mut self) -> u64 {
        self.claude_sdk_session_epoch = self.claude_sdk_session_epoch.wrapping_add(1);
        self.claude_sdk_session_epoch
    }

    pub fn claude_sdk_epoch(&self) -> u64 {
        self.claude_sdk_session_epoch
    }

    pub fn attach_claude_sdk_ws(&mut self, epoch: u64, tx: mpsc::Sender<String>) -> bool {
        if self.claude_sdk_session_epoch != epoch {
            return false;
        }
        self.claude_sdk_ws_tx = Some(tx);
        true
    }

    pub fn clear_claude_sdk_ws(&mut self, epoch: u64) -> bool {
        if self.claude_sdk_session_epoch != epoch {
            return false;
        }
        self.claude_sdk_ws_tx = None;
        true
    }

    pub fn invalidate_claude_sdk_session(&mut self) {
        self.begin_claude_sdk_launch();
        self.claude_sdk_ws_tx = None;
        self.claude_sdk_ready_tx = None;
        self.claude_connection = None;
    }

    pub fn is_claude_sdk_online(&self) -> bool {
        self.claude_sdk_ws_tx.is_some()
    }

    pub fn is_agent_online(&self, agent: &str) -> bool {
        match agent {
            "claude" => {
                self.attached_agents.contains_key("claude") || self.is_claude_sdk_online()
            }
            "codex" => self.codex_inject_tx.is_some(),
            other => self.attached_agents.contains_key(other),
        }
    }

    pub fn provider_connection(&self, agent: &str) -> Option<ProviderConnectionState> {
        match agent {
            "claude" => self.claude_connection.clone(),
            "codex" => self.codex_connection.clone(),
            _ => None,
        }
    }

    pub fn set_provider_connection(&mut self, agent: &str, connection: ProviderConnectionState) {
        match agent {
            "claude" => self.claude_connection = Some(connection),
            "codex" => self.codex_connection = Some(connection),
            _ => {}
        }
    }

    pub fn clear_provider_connection(&mut self, agent: &str) {
        match agent {
            "claude" => self.claude_connection = None,
            "codex" => self.codex_connection = None,
            _ => {}
        }
    }

    pub fn online_role_conflict(&self, agent: &str, role: &str) -> Option<&'static str> {
        match agent {
            "claude" if self.is_agent_online("codex") && self.codex_role == role => Some("codex"),
            "codex" if self.is_agent_online("claude") && self.claude_role == role => Some("claude"),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn flush_buffered(&mut self) -> Vec<BridgeMessage> {
        std::mem::take(&mut self.buffered_messages)
    }

    pub fn buffer_message(&mut self, msg: BridgeMessage) {
        self.buffered_messages.push(msg);
        if self.buffered_messages.len() > 200 {
            self.buffered_messages.drain(0..100);
            eprintln!("[Daemon] buffer overflow: 100 oldest messages dropped");
        }
    }

    /// Re-target buffered messages from old_role to new_role when a role changes.
    pub fn migrate_buffered_role(&mut self, old_role: &str, new_role: &str) {
        for msg in &mut self.buffered_messages {
            if msg.to == old_role {
                msg.to = new_role.to_string();
            }
        }
    }

    pub fn take_buffered_for(&mut self, role: &str) -> Vec<BridgeMessage> {
        self.take_buffered_for_task(role, None)
    }

    pub fn take_buffered_for_task(
        &mut self,
        role: &str,
        task_id: Option<&str>,
    ) -> Vec<BridgeMessage> {
        let mut ready = Vec::new();
        self.buffered_messages.retain(|msg| {
            let same_task = match (task_id, msg.task_id.as_deref()) {
                (Some(expected), Some(actual)) => expected == actual,
                (Some(_), None) => false,
                _ => true,
            };
            if msg.to == role && same_task {
                ready.push(msg.clone());
                false
            } else {
                true
            }
        });
        ready
    }
}

#[path = "state_permission.rs"]
mod state_permission;
#[path = "state_snapshot.rs"]
mod state_snapshot;
#[cfg(test)]
#[path = "state_snapshot_tests.rs"]
mod state_snapshot_tests;
#[path = "state_task_flow.rs"]
mod state_task_flow;
#[cfg(test)]
#[path = "state_task_snapshot_tests.rs"]
mod state_task_snapshot_tests;
#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
