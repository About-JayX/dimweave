use crate::daemon::{
    session_manager::SessionManager,
    types::{
        AgentRuntimeStatus, BridgeMessage, DaemonStatusSnapshot, PermissionBehavior,
        PermissionRequest, PermissionVerdict, ToAgent,
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

pub struct DaemonState {
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
    pending_permissions: HashMap<String, PendingPermission>,
    buffered_verdicts: HashMap<String, Vec<PermissionVerdict>>,
    pub codex_inject_tx: Option<mpsc::Sender<(String, bool)>>,
    pub claude_role: String,
    pub codex_role: String,
    pub session_mgr: Arc<Mutex<SessionManager>>,
    /// Monotonic counter for agent connection generations.
    pub next_agent_gen: u64,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            attached_agents: HashMap::new(),
            buffered_messages: Vec::new(),
            pending_permissions: HashMap::new(),
            buffered_verdicts: HashMap::new(),
            codex_inject_tx: None,
            claude_role: "lead".into(),
            codex_role: "coder".into(),
            session_mgr: Arc::new(Mutex::new(SessionManager::new())),
            next_agent_gen: 0,
        }
    }
}

impl DaemonState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_agent_online(&self, agent: &str) -> bool {
        match agent {
            "claude" => self.attached_agents.contains_key("claude"),
            "codex" => self.codex_inject_tx.is_some(),
            other => self.attached_agents.contains_key(other),
        }
    }

    pub fn online_role_conflict(&self, agent: &str, role: &str) -> Option<&'static str> {
        match agent {
            "claude" if self.is_agent_online("codex") && self.codex_role == role => Some("codex"),
            "codex" if self.is_agent_online("claude") && self.claude_role == role => Some("claude"),
            _ => None,
        }
    }

    pub fn status_snapshot(&self) -> DaemonStatusSnapshot {
        let mut agents = vec![
            AgentRuntimeStatus {
                agent: "claude".into(),
                online: self.is_agent_online("claude"),
            },
            AgentRuntimeStatus {
                agent: "codex".into(),
                online: self.is_agent_online("codex"),
            },
        ];

        let mut other_agents: Vec<_> = self
            .attached_agents
            .keys()
            .filter(|agent| agent.as_str() != "claude" && agent.as_str() != "codex")
            .cloned()
            .collect();
        other_agents.sort();
        agents.extend(other_agents.into_iter().map(|agent| AgentRuntimeStatus {
            agent,
            online: true,
        }));

        DaemonStatusSnapshot {
            agents,
            claude_role: self.claude_role.clone(),
            codex_role: self.codex_role.clone(),
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
        let mut ready = Vec::new();
        self.buffered_messages.retain(|msg| {
            if msg.to == role {
                ready.push(msg.clone());
                false
            } else {
                true
            }
        });
        ready
    }

    pub fn store_permission_request(
        &mut self,
        agent_id: &str,
        request: PermissionRequest,
        created_at: u64,
    ) {
        self.prune_expired_permissions(created_at);
        self.pending_permissions.insert(
            request.request_id.clone(),
            PendingPermission {
                agent_id: agent_id.to_string(),
                created_at,
                request,
            },
        );
    }

    pub fn resolve_permission(
        &mut self,
        request_id: &str,
        behavior: PermissionBehavior,
        now_ms: u64,
    ) -> Option<(String, ToAgent)> {
        self.prune_expired_permissions(now_ms);
        let pending = self.pending_permissions.remove(request_id)?;
        Some((
            pending.agent_id,
            ToAgent::PermissionVerdict {
                verdict: PermissionVerdict {
                    request_id: request_id.to_string(),
                    behavior,
                },
            },
        ))
    }

    pub fn buffer_permission_verdict(&mut self, agent_id: &str, verdict: PermissionVerdict) {
        let entry = self
            .buffered_verdicts
            .entry(agent_id.to_string())
            .or_default();
        entry.push(verdict);
        if entry.len() > 50 {
            entry.drain(0..25);
        }
    }

    pub fn take_buffered_verdicts_for(&mut self, agent_id: &str) -> Vec<PermissionVerdict> {
        self.buffered_verdicts.remove(agent_id).unwrap_or_default()
    }

    fn prune_expired_permissions(&mut self, now_ms: u64) {
        self.pending_permissions
            .retain(|_, pending| now_ms.saturating_sub(pending.created_at) <= PERMISSION_TTL_MS);
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
