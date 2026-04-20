use crate::daemon::{
    session_manager::SessionManager,
    task_graph::TaskGraphStore,
    task_runtime::TaskRuntime,
    types::{
        AgentRuntimeStatus, BridgeMessage, DaemonStatusSnapshot, OnlineAgentInfo,
        PermissionBehavior, PermissionRequest, PermissionVerdict, ProviderConnectionState,
        RuntimeHealthStatus, ToAgent,
    },
};
use std::{collections::HashMap, sync::Arc};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClaudeSdkDirectTextState {
    Inactive,
    Active,
    CompletedBySdk,
    CompletedByBridge,
}

pub struct DaemonState {
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
    pending_permissions: HashMap<String, PendingPermission>,
    buffered_verdicts: HashMap<String, Vec<PermissionVerdict>>,
    /// Compatibility-only: global Codex inject channel. Task-scoped code should
    /// use `CodexTaskSlot.inject_tx` via `codex_task_inject_tx(task_id)`.
    pub codex_inject_tx: Option<mpsc::Sender<(Vec<serde_json::Value>, bool)>>,
    codex_session_epoch: u64,
    /// Compatibility-only: global Claude SDK WS sender. Task-scoped code should
    /// use `ClaudeTaskSlot.ws_tx` via the task runtime.
    pub claude_sdk_ws_tx: Option<mpsc::Sender<String>>,
    /// Compatibility-only: global Claude SDK event sender.
    pub claude_sdk_event_tx: Option<mpsc::Sender<Vec<serde_json::Value>>>,
    /// Oneshot that fires when Claude connects via WS, carrying the inject sender.
    pub claude_sdk_ready_tx: Option<tokio::sync::oneshot::Sender<mpsc::Sender<String>>>,
    claude_sdk_session_epoch: u64,
    claude_sdk_pending_nonce: Option<String>,
    claude_sdk_active_nonce: Option<String>,
    claude_sdk_ws_generation: u64,
    claude_sdk_direct_text_state: ClaudeSdkDirectTextState,
    claude_preview_buffer: String,
    claude_preview_flush_scheduled: bool,
    /// Compatibility-only: global role label set at connect time.
    /// Not authoritative for task-scoped ownership — use task's
    /// `lead_provider`/`coder_provider` via `resolve_task_provider_agent()`.
    pub claude_role: String,
    /// Compatibility-only: global role label set at connect time.
    /// Not authoritative for task-scoped ownership — use task's
    /// `lead_provider`/`coder_provider` via `resolve_task_provider_agent()`.
    pub codex_role: String,
    /// Compatibility-only: global provider connection mirror.
    /// Task-scoped code should use `task_provider_connection(task_id, agent)`.
    pub claude_connection: Option<ProviderConnectionState>,
    /// Compatibility-only: global provider connection mirror.
    /// Task-scoped code should use `task_provider_connection(task_id, agent)`.
    pub codex_connection: Option<ProviderConnectionState>,
    pub runtime_health: Option<RuntimeHealthStatus>,
    pub session_mgr: Arc<Mutex<SessionManager>>,
    /// Monotonic counter for agent connection generations.
    pub next_agent_gen: u64,
    /// Normalized task/session/artifact graph.
    pub task_graph: TaskGraphStore,
    pub active_task_id: Option<String>,
    /// Per-task runtime state, keyed by task_id.
    pub task_runtimes: HashMap<String, TaskRuntime>,
    pub telegram_outbound_tx: Option<tokio::sync::mpsc::Sender<crate::telegram::types::TelegramOutbound>>,
    pub telegram_paired_chat_id: Option<i64>,
    pub telegram_notifications_enabled: bool,
    pub feishu_project_store: crate::feishu_project::store::FeishuProjectStore,
    pub feishu_issue_view: Vec<crate::feishu_project::types::FeishuProjectInboxItem>,
    pub feishu_project_runtime: Option<crate::feishu_project::types::FeishuProjectRuntimeState>,
    pub feishu_issue_cursor: Option<crate::feishu_project::issue_query::IssueQueryCursor>,
    /// When set, `auto_save_task_graph` enqueues a debounced save via this
    /// channel instead of running `save_to_db` synchronously. The daemon
    /// main loop spawns a saver task that coalesces bursts into a single
    /// SQLite write per ~200ms. None in tests (fallback: sync save), Some
    /// in production after `daemon::run()` wiring.
    pub save_tx: Option<mpsc::UnboundedSender<()>>,
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
            claude_sdk_event_tx: None,
            claude_sdk_ready_tx: None,
            claude_sdk_session_epoch: 0,
            claude_sdk_pending_nonce: None,
            claude_sdk_active_nonce: None,
            claude_sdk_ws_generation: 0,
            claude_sdk_direct_text_state: ClaudeSdkDirectTextState::Inactive,
            claude_preview_buffer: String::new(),
            claude_preview_flush_scheduled: false,
            claude_role: "".into(),
            codex_role: "".into(),
            claude_connection: None,
            codex_connection: None,
            runtime_health: None,
            session_mgr: Arc::new(Mutex::new(SessionManager::new())),
            next_agent_gen: 0,
            task_graph: TaskGraphStore::new(),
            active_task_id: None,
            task_runtimes: HashMap::new(),
            telegram_outbound_tx: None,
            telegram_paired_chat_id: None,
            telegram_notifications_enabled: false,
            feishu_project_store: crate::feishu_project::store::FeishuProjectStore::default(),
            feishu_issue_view: Vec::new(),
            feishu_project_runtime: None,
            feishu_issue_cursor: None,
            save_tx: None,
        }
    }
}

impl DaemonState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[path = "state_delivery.rs"]
mod state_delivery;
#[path = "state_persistence.rs"]
mod state_persistence;
#[path = "state_permission.rs"]
mod state_permission;
#[path = "state_runtime.rs"]
mod state_runtime;
#[path = "state_snapshot.rs"]
mod state_snapshot;
#[cfg(test)]
#[path = "state_snapshot_tests.rs"]
mod state_snapshot_tests;
#[path = "state_task_flow.rs"]
mod state_task_flow;
pub(crate) use state_task_flow::MatchedTaskAgent;
#[cfg(test)]
#[path = "state_task_snapshot_tests.rs"]
mod state_task_snapshot_tests;
#[cfg(test)]
#[path = "state_persistence_tests.rs"]
mod state_persistence_tests;
#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
