use super::*;
use crate::daemon::gui_task::{build_task_change_events, TaskUiEvent};
use crate::daemon::orchestrator::task_flow;
use crate::daemon::task_graph::types::{Provider, SessionHandle};
use crate::daemon::types::BridgeMessage;

/// A concrete task agent matched during role resolution.
/// Preserves the `agent_id` from `TaskAgent` so routing and snapshots
/// can distinguish same-provider same-role agents (AC1/AC2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedTaskAgent {
    pub agent_id: String,
    pub runtime: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRoutingDecision {
    pub is_allowed: bool,
    pub buffer_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskFlowEffect {
    pub released: Vec<BridgeMessage>,
    pub ui_events: Vec<TaskUiEvent>,
}

impl DaemonState {
    fn task_session_for_role<'a>(&'a self, task_id: &str, role: &str) -> Option<&'a SessionHandle> {
        let task = self.task_graph.get_task(task_id)?;
        let session_id = match role {
            "lead" => task.lead_session_id.as_deref()?,
            "coder" => task.current_coder_session_id.as_deref()?,
            _ => return None,
        };
        self.task_graph.get_session(session_id)
    }

    fn bound_session_for_message<'a>(
        &'a self,
        message: &BridgeMessage,
    ) -> Option<&'a SessionHandle> {
        crate::daemon::routing_target_session::resolve_target_bound_session(
            &self.task_graph,
            message,
        )
    }

    fn agent_matches_bound_session(&self, agent: &str, session: &SessionHandle) -> bool {
        let expected_provider = match agent {
            "claude" => Provider::Claude,
            "codex" => Provider::Codex,
            _ => return false,
        };
        if session.provider != expected_provider {
            return false;
        }

        match self.provider_connection(agent) {
            Some(connection) => {
                session.external_session_id.as_deref()
                    == Some(connection.external_session_id.as_str())
            }
            None => session.external_session_id.is_none(),
        }
    }

    pub fn role_has_compatible_online_agent(&self, role: &str) -> bool {
        if role != "lead" && role != "coder" {
            return (self.is_agent_online("claude") && self.claude_role == role)
                || (self.is_agent_online("codex") && self.codex_role == role);
        }

        let Some(task_id) = self.active_task_id.as_deref() else {
            return (self.is_agent_online("claude") && self.claude_role == role)
                || (self.is_agent_online("codex") && self.codex_role == role);
        };
        let Some(session) = self.task_session_for_role(task_id, role) else {
            return false;
        };

        (self.is_agent_online("claude")
            && self.claude_role == role
            && self.agent_matches_bound_session("claude", session))
            || (self.is_agent_online("codex")
                && self.codex_role == role
                && self.agent_matches_bound_session("codex", session))
    }

    pub fn agent_matches_task_message(&self, agent: &str, message: &BridgeMessage) -> bool {
        if message.to != "lead" && message.to != "coder" {
            return true;
        }

        // Legacy/system messages without stamped task context still rely on
        // plain role routing until every caller carries task/session ownership.
        if message.session_id.is_none() && message.task_id.is_none() {
            return true;
        }

        let Some(session) = self.bound_session_for_message(message) else {
            return false;
        };

        self.agent_matches_bound_session(agent, session)
    }

    pub fn route_buffer_reason(&self, message: &BridgeMessage) -> Option<&'static str> {
        if message.to != "lead" && message.to != "coder" {
            return Some("target_agent_offline");
        }

        if self.bound_session_for_message(message).is_none() {
            return Some("target_session_missing");
        }

        let claude_online_mismatch =
            self.claude_role == message.to
                && self.is_agent_online("claude")
                && !self.agent_matches_task_message("claude", message);
        let codex_online_mismatch =
            self.codex_role == message.to
                && self.is_agent_online("codex")
                && !self.agent_matches_task_message("codex", message);

        if claude_online_mismatch || codex_online_mismatch {
            return Some("task_session_mismatch");
        }

        Some("target_agent_offline")
    }

    pub fn set_active_task(&mut self, task_id: Option<String>) {
        self.active_task_id = task_id;
    }

    pub fn preferred_auto_target(&self) -> Option<String> {
        let task = self
            .active_task_id
            .as_deref()
            .and_then(|id| self.task_graph.get_task(id))?;
        task_flow::preferred_auto_target(task)
    }

    pub fn stamp_message_context(&self, role: &str, message: &mut BridgeMessage) {
        let Some(task_id) = self.active_task_id.as_deref() else {
            return;
        };
        self.stamp_message_context_for_task(task_id, role, message);
    }

    /// Stamp a message with a specific task's context instead of the active task.
    /// Used by provider-originated messages to bind to the owning task runtime.
    pub fn stamp_message_context_for_task(
        &self,
        task_id: &str,
        role: &str,
        message: &mut BridgeMessage,
    ) {
        let Some(task) = self.task_graph.get_task(task_id) else {
            return;
        };
        message.task_id = Some(task.task_id.clone());
        message.session_id = match role {
            "lead" => task.lead_session_id.clone(),
            "coder" => task.current_coder_session_id.clone(),
            _ => None,
        };
    }

    /// Find the task_id that owns the currently active Codex connection.
    /// Scans all per-task Codex slots (default + extras), falls back to active_task_id.
    pub fn codex_owning_task_id(&self) -> Option<String> {
        for (task_id, rt) in &self.task_runtimes {
            if rt.all_codex_slots().any(|s| s.is_online()) {
                return Some(task_id.clone());
            }
        }
        self.active_task_id.clone()
    }

    /// Find the task_id that owns the currently active Claude connection.
    /// Scans all per-task Claude slots (default + extras), falls back to active_task_id.
    pub fn claude_owning_task_id(&self) -> Option<String> {
        for (task_id, rt) in &self.task_runtimes {
            if rt.all_claude_slots().any(|s| s.is_online()) {
                return Some(task_id.clone());
            }
        }
        self.active_task_id.clone()
    }

    /// Find the owning task for a specific agent ("claude" or "codex").
    pub fn agent_owning_task_id(&self, agent: &str) -> Option<String> {
        match agent {
            "claude" => self.claude_owning_task_id(),
            "codex" => self.codex_owning_task_id(),
            _ => self.active_task_id.clone(),
        }
    }

    /// Resolve all matched task agents serving a given role, preserving
    /// concrete `agent_id` from `task_agents[]` (AC1/AC2).
    /// Falls back to singleton `lead_provider`/`coder_provider` when no
    /// task_agents exist (transitional compat for pre-migration tasks).
    pub fn resolve_task_role_providers(
        &self,
        task_id: &str,
        role: &str,
    ) -> Vec<MatchedTaskAgent> {
        let agents = self.task_graph.agents_for_task(task_id);
        if agents.is_empty() {
            return self.resolve_role_providers_legacy(task_id, role);
        }
        agents
            .iter()
            .filter(|a| a.role == role)
            .map(|a| MatchedTaskAgent {
                agent_id: a.agent_id.clone(),
                runtime: match a.provider {
                    Provider::Claude => "claude",
                    Provider::Codex => "codex",
                },
            })
            .collect()
    }

    /// Legacy singleton fallback for tasks without task_agents.
    fn resolve_role_providers_legacy(
        &self,
        task_id: &str,
        role: &str,
    ) -> Vec<MatchedTaskAgent> {
        let Some(task) = self.task_graph.get_task(task_id) else {
            return vec![];
        };
        let (_provider, runtime) = match role {
            "lead" => (&task.lead_provider, match task.lead_provider {
                Provider::Claude => "claude",
                Provider::Codex => "codex",
            }),
            "coder" => (&task.coder_provider, match task.coder_provider {
                Provider::Claude => "claude",
                Provider::Codex => "codex",
            }),
            _ => return vec![],
        };
        vec![MatchedTaskAgent {
            agent_id: runtime.to_string(),
            runtime,
        }]
    }

    /// Compat wrapper: resolve the first runtime name for a role.
    pub fn resolve_task_provider_agent(&self, task_id: &str, role: &str) -> Option<&'static str> {
        self.resolve_task_role_providers(task_id, role)
            .into_iter()
            .next()
            .map(|m| m.runtime)
    }

    pub fn prepare_task_routing(&mut self, msg: &BridgeMessage) -> TaskRoutingDecision {
        let _ = msg;
        TaskRoutingDecision {
            is_allowed: true,
            buffer_reason: None,
        }
    }

    pub fn observe_task_message(&mut self, msg: &BridgeMessage) -> Vec<BridgeMessage> {
        self.observe_task_message_effects(msg).released
    }

    pub fn observe_task_message_effects(&mut self, msg: &BridgeMessage) -> TaskFlowEffect {
        let Some(task_id) = self.active_task_id.clone() else {
            return TaskFlowEffect::default();
        };
        let before = self.task_graph.get_task(&task_id).cloned();
        let released = task_flow::process_message(&mut self.task_graph, &task_id, msg);
        self.auto_save_task_graph();
        let after = self.task_graph.get_task(&task_id).cloned();
        TaskFlowEffect {
            released,
            ui_events: build_task_change_events(before.as_ref(), after.as_ref()),
        }
    }
}
