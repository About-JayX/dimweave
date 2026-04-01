use crate::daemon::task_graph::types::Task;
use crate::daemon::types::{
    self, HistoryEntry, PermissionBehavior, SessionTreeSnapshot, TaskSnapshot,
};
use tokio::sync::{mpsc, oneshot};

pub enum DaemonCmd {
    SendUserInput {
        content: String,
        target: String,
    },
    LaunchCodex {
        role_id: String,
        cwd: String,
        model: Option<String>,
        reasoning_effort: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    StopCodex,
    Shutdown {
        reply: oneshot::Sender<()>,
    },
    ReadStatusSnapshot {
        reply: oneshot::Sender<types::DaemonStatusSnapshot>,
    },
    ReadClaudeRole {
        reply: oneshot::Sender<String>,
    },
    SetClaudeRole {
        role: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SetCodexRole {
        role: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    RespondPermission {
        request_id: String,
        behavior: PermissionBehavior,
    },
    ForceDisconnectAgent {
        agent_id: String,
    },
    // ── Task management ───────────────────────────────────────
    CreateTask {
        workspace: String,
        title: String,
        reply: oneshot::Sender<Task>,
    },
    ListTasks {
        workspace: Option<String>,
        reply: oneshot::Sender<Vec<Task>>,
    },
    SelectTask {
        task_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    GetTaskSnapshot {
        reply: oneshot::Sender<Option<TaskSnapshot>>,
    },
    ApproveReview {
        reply: oneshot::Sender<Result<(), String>>,
    },
    ListSessionTree {
        task_id: String,
        reply: oneshot::Sender<Option<SessionTreeSnapshot>>,
    },
    ListHistory {
        workspace: Option<String>,
        reply: oneshot::Sender<Vec<HistoryEntry>>,
    },
    ResumeSession {
        session_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

pub fn channel() -> (mpsc::Sender<DaemonCmd>, mpsc::Receiver<DaemonCmd>) {
    mpsc::channel(64)
}

const AGENT_ROLES: &[&str] = &["lead", "coder", "reviewer"];

pub fn is_valid_agent_role(role: &str) -> bool {
    AGENT_ROLES.contains(&role)
}
