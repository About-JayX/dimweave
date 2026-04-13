use super::*;
use crate::daemon::task_graph::types::{Provider, SessionStatus};

impl DaemonState {
    fn detach_provider_binding(&mut self, agent: &str) -> Option<String> {
        let connection = self.provider_connection(agent)?;
        let session = self
            .task_graph
            .find_session_by_external_id(connection.provider, &connection.external_session_id)
            .cloned()?;

        let _ = self
            .task_graph
            .update_session_status(&session.session_id, SessionStatus::Paused);
        let _ = self
            .task_graph
            .clear_lead_session_if_matches(&session.task_id, &session.session_id);
        let _ = self
            .task_graph
            .clear_coder_session_if_matches(&session.task_id, &session.session_id);
        self.auto_save_task_graph();
        Some(session.task_id)
    }

    /// Detach only the Claude sessions belonging to a specific task.
    /// Unlike `detach_provider_binding`, this does NOT read the global
    /// `claude_connection` mirror, so it cannot affect another task's binding.
    fn detach_claude_sessions_for_task(&mut self, task_id: &str) {
        let session_ids: Vec<String> = self
            .task_graph
            .sessions_for_task(task_id)
            .into_iter()
            .filter(|s| s.provider == Provider::Claude)
            .map(|s| s.session_id.clone())
            .collect();
        if session_ids.is_empty() {
            return;
        }
        for sid in &session_ids {
            let _ = self.task_graph.update_session_status(sid, SessionStatus::Paused);
            let _ = self.task_graph.clear_lead_session_if_matches(task_id, sid);
            let _ = self.task_graph.clear_coder_session_if_matches(task_id, sid);
        }
        self.auto_save_task_graph();
    }

    pub fn codex_session_epoch(&self) -> u64 {
        self.codex_session_epoch
    }

    pub fn begin_codex_launch(&mut self) -> u64 {
        self.codex_session_epoch = self.codex_session_epoch.wrapping_add(1);
        self.codex_session_epoch
    }

    pub fn invalidate_codex_session(&mut self) -> Option<String> {
        self.begin_codex_launch();
        self.codex_inject_tx = None;
        self.clear_provider_connection("codex")
    }

    pub fn attach_codex_session_if_current(
        &mut self,
        epoch: u64,
        tx: mpsc::Sender<(Vec<serde_json::Value>, bool)>,
    ) -> bool {
        if self.codex_session_epoch != epoch {
            return false;
        }
        self.codex_inject_tx = Some(tx);
        true
    }

    pub fn clear_codex_session_if_current(&mut self, epoch: u64) -> Option<String> {
        if self.codex_session_epoch != epoch {
            return None;
        }
        self.codex_inject_tx = None;
        self.clear_provider_connection("codex")
    }

    fn advance_claude_sdk_epoch(&mut self) -> u64 {
        self.claude_sdk_session_epoch = self.claude_sdk_session_epoch.wrapping_add(1);
        self.claude_sdk_session_epoch
    }

    pub fn begin_claude_sdk_launch(&mut self, launch_nonce: String) -> u64 {
        let epoch = self.advance_claude_sdk_epoch();
        self.claude_sdk_pending_nonce = Some(launch_nonce);
        self.claude_sdk_active_nonce = None;
        self.claude_sdk_ws_tx = None;
        self.clear_claude_preview_batch();
        epoch
    }

    pub fn claude_sdk_epoch(&self) -> u64 {
        self.claude_sdk_session_epoch
    }

    pub fn claude_sdk_accepts_launch_nonce(&self, launch_nonce: &str) -> bool {
        // Primary: scan task-local slots (truth source for multi-task)
        if self.find_task_for_claude_nonce(launch_nonce).is_some() {
            return true;
        }
        // Fallback: singleton mirrors for legacy (non-task-scoped) launches
        self.claude_sdk_pending_nonce.as_deref() == Some(launch_nonce)
            || self.claude_sdk_active_nonce.as_deref() == Some(launch_nonce)
    }

    pub fn attach_claude_sdk_ws(
        &mut self,
        epoch: u64,
        launch_nonce: &str,
        tx: mpsc::Sender<String>,
    ) -> Option<u64> {
        if self.claude_sdk_session_epoch != epoch {
            return None;
        }
        if self.claude_sdk_pending_nonce.as_deref() == Some(launch_nonce) {
            self.claude_sdk_active_nonce = Some(launch_nonce.to_string());
            self.claude_sdk_pending_nonce = None;
        } else if self.claude_sdk_active_nonce.as_deref() != Some(launch_nonce) {
            return None;
        }
        self.claude_sdk_ws_generation = self.claude_sdk_ws_generation.wrapping_add(1);
        self.claude_sdk_ws_tx = Some(tx);
        Some(self.claude_sdk_ws_generation)
    }

    pub fn clear_claude_sdk_ws(
        &mut self,
        epoch: u64,
        launch_nonce: &str,
        ws_generation: u64,
    ) -> bool {
        if self.claude_sdk_session_epoch != epoch {
            return false;
        }
        if self.claude_sdk_active_nonce.as_deref() != Some(launch_nonce) {
            return false;
        }
        if self.claude_sdk_ws_generation != ws_generation {
            return false;
        }
        self.claude_sdk_ws_tx = None;
        true
    }

    pub fn invalidate_claude_sdk_session(&mut self) -> Option<String> {
        self.advance_claude_sdk_epoch();
        self.claude_sdk_ws_tx = None;
        self.claude_sdk_event_tx = None;
        self.claude_sdk_ready_tx = None;
        self.claude_sdk_pending_nonce = None;
        self.claude_sdk_active_nonce = None;
        self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Inactive;
        self.clear_claude_preview_batch();
        self.clear_provider_connection("claude")
    }

    pub fn invalidate_claude_sdk_session_if_current(&mut self, epoch: u64) -> Option<String> {
        if self.claude_sdk_session_epoch != epoch {
            return None;
        }
        self.invalidate_claude_sdk_session()
    }

    pub fn append_claude_preview_delta(&mut self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        self.claude_preview_buffer.push_str(text);
        if self.claude_preview_flush_scheduled {
            return false;
        }

        self.claude_preview_flush_scheduled = true;
        true
    }

    pub fn take_claude_preview_batch(&mut self) -> Option<String> {
        self.claude_preview_flush_scheduled = false;
        if self.claude_preview_buffer.is_empty() {
            return None;
        }

        Some(std::mem::take(&mut self.claude_preview_buffer))
    }

    pub fn clear_claude_preview_batch(&mut self) {
        self.claude_preview_buffer.clear();
        self.claude_preview_flush_scheduled = false;
    }

    #[cfg(test)]
    pub fn claude_sdk_pending_nonce(&self) -> Option<&str> {
        self.claude_sdk_pending_nonce.as_deref()
    }

    #[cfg(test)]
    pub fn claude_sdk_active_nonce(&self) -> Option<&str> {
        self.claude_sdk_active_nonce.as_deref()
    }

    // ── Task-scoped Claude SDK lifecycle ─────────────────────

    /// Find which task owns a given Claude launch nonce.
    pub fn find_task_for_claude_nonce(&self, nonce: &str) -> Option<String> {
        self.task_runtimes.iter().find_map(|(tid, rt)| {
            let slot = rt.claude_slot.as_ref()?;
            if slot.pending_nonce.as_deref() == Some(nonce)
                || slot.active_nonce.as_deref() == Some(nonce)
            {
                Some(tid.clone())
            } else {
                None
            }
        })
    }

    /// Begin a Claude launch scoped to a specific task.
    pub fn begin_claude_task_launch(&mut self, task_id: &str, nonce: String) -> Option<u64> {
        let rt = self.task_runtimes.get_mut(task_id)?;
        let slot = rt.claude_slot.get_or_insert_with(
            crate::daemon::task_runtime::ClaudeTaskSlot::new,
        );
        slot.session_epoch = slot.session_epoch.wrapping_add(1);
        slot.pending_nonce = Some(nonce);
        slot.active_nonce = None;
        slot.ws_tx = None;
        slot.preview_buffer.clear();
        slot.preview_flush_scheduled = false;
        Some(slot.session_epoch)
    }

    /// Attach a Claude WS connection to a specific task's slot.
    pub fn attach_claude_task_ws(
        &mut self,
        task_id: &str,
        epoch: u64,
        launch_nonce: &str,
        tx: mpsc::Sender<String>,
    ) -> Option<u64> {
        let slot = self.task_runtimes.get_mut(task_id)?.claude_slot.as_mut()?;
        if slot.session_epoch != epoch {
            return None;
        }
        if slot.pending_nonce.as_deref() == Some(launch_nonce) {
            slot.active_nonce = Some(launch_nonce.to_string());
            slot.pending_nonce = None;
        } else if slot.active_nonce.as_deref() != Some(launch_nonce) {
            return None;
        }
        slot.ws_generation = slot.ws_generation.wrapping_add(1);
        slot.ws_tx = Some(tx.clone());
        let gen = slot.ws_generation;
        // Sync singleton ws_tx — routing.rs / permission.rs read this directly.
        // In single-active-Claude mode, this is always the most recent attach.
        self.claude_sdk_ws_tx = Some(tx);
        Some(gen)
    }

    /// Clear Claude WS for a specific task's slot.
    pub fn clear_claude_task_ws(
        &mut self,
        task_id: &str,
        epoch: u64,
        launch_nonce: &str,
        ws_generation: u64,
    ) -> bool {
        let Some(slot) = self.task_runtimes.get_mut(task_id)
            .and_then(|rt| rt.claude_slot.as_mut())
        else {
            return false;
        };
        if slot.session_epoch != epoch
            || slot.active_nonce.as_deref() != Some(launch_nonce)
            || slot.ws_generation != ws_generation
        {
            return false;
        }
        slot.ws_tx = None;
        // Recompute singleton ws_tx from surviving task slots
        self.recompute_claude_singleton_ws_tx();
        true
    }

    /// Recompute singleton `claude_sdk_ws_tx` from task slots.
    /// Sets it to the first online slot's ws_tx, or None if none online.
    fn recompute_claude_singleton_ws_tx(&mut self) {
        self.claude_sdk_ws_tx = self.task_runtimes.values()
            .filter_map(|rt| rt.claude_slot.as_ref())
            .find_map(|slot| slot.ws_tx.clone());
    }

    /// Invalidate Claude session for a specific task.
    /// Only clears this task's slot; does NOT blindly wipe global state.
    pub fn invalidate_claude_task_session(&mut self, task_id: &str) -> Option<String> {
        if let Some(slot) = self.task_runtimes.get_mut(task_id)
            .and_then(|rt| rt.claude_slot.as_mut())
        {
            slot.session_epoch = slot.session_epoch.wrapping_add(1);
            slot.ws_tx = None;
            slot.event_tx = None;
            slot.ready_tx = None;
            slot.pending_nonce = None;
            slot.active_nonce = None;
            slot.preview_buffer.clear();
            slot.preview_flush_scheduled = false;
        }
        // Only clear global state if NO other task has an active Claude slot
        if !self.any_claude_task_online() {
            self.invalidate_claude_sdk_session()
        } else {
            // Detach only the requested task's Claude sessions in task_graph;
            // do NOT touch the global claude_connection mirror (it may belong to another task).
            self.detach_claude_sessions_for_task(task_id);
            self.recompute_claude_singleton_ws_tx();
            Some(task_id.to_string())
        }
    }

    /// True if any task runtime has an online Claude slot.
    fn any_claude_task_online(&self) -> bool {
        self.task_runtimes.values().any(|rt| {
            rt.claude_slot.as_ref().map_or(false, |s| s.is_online())
        })
    }

    /// Conditionally invalidate if task epoch matches.
    pub fn invalidate_claude_task_session_if_current(
        &mut self,
        task_id: &str,
        epoch: u64,
    ) -> Option<String> {
        let matches = self.task_runtimes.get(task_id)
            .and_then(|rt| rt.claude_slot.as_ref())
            .map_or(false, |slot| slot.session_epoch == epoch);
        if !matches {
            return None;
        }
        self.invalidate_claude_task_session(task_id)
    }

    /// Get the session epoch for a task's Claude slot.
    pub fn claude_task_epoch(&self, task_id: &str) -> Option<u64> {
        self.task_runtimes.get(task_id)?
            .claude_slot.as_ref()
            .map(|s| s.session_epoch)
    }

    /// Check if a specific task's Claude slot is online.
    pub fn is_claude_task_online(&self, task_id: &str) -> bool {
        self.task_runtimes.get(task_id)
            .and_then(|rt| rt.claude_slot.as_ref())
            .map_or(false, |slot| slot.is_online())
    }

    /// Store ready_tx and event_tx in a task's Claude slot.
    pub fn set_claude_task_channels(
        &mut self,
        task_id: &str,
        ready_tx: tokio::sync::oneshot::Sender<mpsc::Sender<String>>,
        event_tx: mpsc::Sender<Vec<serde_json::Value>>,
    ) {
        if let Some(slot) = self.task_runtimes.get_mut(task_id)
            .and_then(|rt| rt.claude_slot.as_mut())
        {
            slot.ready_tx = Some(ready_tx);
            slot.event_tx = Some(event_tx);
        }
        // Singleton mirrors set by callers for backward compat
    }

    /// Take the ready_tx from a task's Claude slot.
    pub fn take_claude_task_ready_tx(
        &mut self,
        task_id: &str,
    ) -> Option<tokio::sync::oneshot::Sender<mpsc::Sender<String>>> {
        self.task_runtimes.get_mut(task_id)?
            .claude_slot.as_mut()?
            .ready_tx.take()
    }

    /// Get the event_tx for whichever task owns this launch nonce.
    pub fn claude_task_event_tx_for_nonce(
        &self,
        nonce: &str,
    ) -> Option<mpsc::Sender<Vec<serde_json::Value>>> {
        let task_id = self.find_task_for_claude_nonce(nonce)?;
        self.task_runtimes.get(&task_id)?
            .claude_slot.as_ref()?
            .event_tx.clone()
    }

    /// Get the ws_tx for a specific task's Claude slot.
    pub fn claude_task_ws_tx(&self, task_id: &str) -> Option<mpsc::Sender<String>> {
        self.task_runtimes.get(task_id)?
            .claude_slot.as_ref()?
            .ws_tx.clone()
    }

    pub fn is_claude_sdk_online(&self) -> bool {
        // Primary: any task slot with an active WS
        if self.task_runtimes.values().any(|rt| {
            rt.claude_slot.as_ref().map_or(false, |s| s.is_online())
        }) {
            return true;
        }
        // Fallback: singleton mirror for legacy callers
        self.claude_sdk_ws_tx.is_some()
    }

    // ── Task-scoped Codex lifecycle ────────────────────────────

    /// Allocate a port and begin a Codex launch scoped to a specific task.
    pub fn begin_codex_task_launch(&mut self, task_id: &str, port: u16) -> Option<u64> {
        let rt = self.task_runtimes.get_mut(task_id)?;
        let slot = rt.codex_slot.get_or_insert_with(|| {
            crate::daemon::task_runtime::CodexTaskSlot::new(port)
        });
        slot.session_epoch = slot.session_epoch.wrapping_add(1);
        slot.inject_tx = None;
        slot.port = port;
        Some(slot.session_epoch)
    }

    /// Attach a Codex inject channel and connection to a specific task's slot.
    pub fn attach_codex_task_session(
        &mut self,
        task_id: &str,
        epoch: u64,
        tx: mpsc::Sender<(Vec<serde_json::Value>, bool)>,
        connection: Option<crate::daemon::types::ProviderConnectionState>,
    ) -> bool {
        let Some(slot) = self.task_runtimes.get_mut(task_id)
            .and_then(|rt| rt.codex_slot.as_mut())
        else {
            return false;
        };
        if slot.session_epoch != epoch {
            return false;
        }
        slot.inject_tx = Some(tx.clone());
        slot.connection = connection.clone();
        // Sync singleton mirrors — routing.rs reads codex_inject_tx directly
        self.codex_inject_tx = Some(tx);
        if let Some(conn) = connection {
            self.codex_connection = Some(conn);
        }
        true
    }

    /// Clear Codex session for a specific task if epoch matches.
    /// Uses task-specific graph cleanup to avoid cross-task pollution
    /// through the singleton `codex_connection` mirror.
    pub fn clear_codex_task_session(
        &mut self,
        task_id: &str,
        epoch: u64,
    ) -> Option<String> {
        let Some(slot) = self.task_runtimes.get_mut(task_id)
            .and_then(|rt| rt.codex_slot.as_mut())
        else {
            return None;
        };
        if slot.session_epoch != epoch {
            return None;
        }
        slot.inject_tx = None;
        slot.connection = None;
        self.recompute_codex_singleton_inject_tx();
        self.recompute_codex_singleton_connection();
        if !self.any_codex_task_online() {
            self.clear_provider_connection("codex")
        } else {
            self.detach_codex_sessions_for_task(task_id);
            Some(task_id.to_string())
        }
    }

    /// Invalidate Codex session for a specific task.
    pub fn invalidate_codex_task_session(&mut self, task_id: &str) -> Option<String> {
        if let Some(slot) = self.task_runtimes.get_mut(task_id)
            .and_then(|rt| rt.codex_slot.as_mut())
        {
            slot.session_epoch = slot.session_epoch.wrapping_add(1);
            slot.inject_tx = None;
            slot.connection = None;
        }
        if !self.any_codex_task_online() {
            self.invalidate_codex_session()
        } else {
            self.detach_codex_sessions_for_task(task_id);
            self.recompute_codex_singleton_inject_tx();
            self.recompute_codex_singleton_connection();
            Some(task_id.to_string())
        }
    }

    /// Detach only the Codex sessions belonging to a specific task.
    fn detach_codex_sessions_for_task(&mut self, task_id: &str) {
        let session_ids: Vec<String> = self
            .task_graph
            .sessions_for_task(task_id)
            .into_iter()
            .filter(|s| s.provider == Provider::Codex)
            .map(|s| s.session_id.clone())
            .collect();
        if session_ids.is_empty() {
            return;
        }
        for sid in &session_ids {
            let _ = self.task_graph.update_session_status(sid, SessionStatus::Paused);
            let _ = self.task_graph.clear_lead_session_if_matches(task_id, sid);
            let _ = self.task_graph.clear_coder_session_if_matches(task_id, sid);
        }
        self.auto_save_task_graph();
    }

    /// True if any task runtime has an online Codex slot.
    fn any_codex_task_online(&self) -> bool {
        self.task_runtimes.values().any(|rt| {
            rt.codex_slot.as_ref().map_or(false, |s| s.is_online())
        })
    }

    /// Recompute singleton `codex_inject_tx` from task slots.
    fn recompute_codex_singleton_inject_tx(&mut self) {
        self.codex_inject_tx = self.task_runtimes.values()
            .filter_map(|rt| rt.codex_slot.as_ref())
            .find_map(|slot| slot.inject_tx.clone());
    }

    /// Recompute singleton `codex_connection` from task slots.
    fn recompute_codex_singleton_connection(&mut self) {
        self.codex_connection = self.task_runtimes.values()
            .filter_map(|rt| rt.codex_slot.as_ref())
            .filter(|slot| slot.is_online())
            .find_map(|slot| slot.connection.clone());
    }

    pub fn codex_task_epoch(&self, task_id: &str) -> Option<u64> {
        self.task_runtimes.get(task_id)?
            .codex_slot.as_ref()
            .map(|s| s.session_epoch)
    }

    pub fn is_codex_task_online(&self, task_id: &str) -> bool {
        self.task_runtimes.get(task_id)
            .and_then(|rt| rt.codex_slot.as_ref())
            .map_or(false, |slot| slot.is_online())
    }

    /// Check if an agent's task-local slot is online.
    pub fn is_task_agent_online(&self, task_id: &str, agent: &str) -> bool {
        match agent {
            "claude" => self.is_claude_task_online(task_id),
            "codex" => self.is_codex_task_online(task_id),
            _ => false,
        }
    }

    pub fn codex_task_inject_tx(
        &self,
        task_id: &str,
    ) -> Option<mpsc::Sender<(Vec<serde_json::Value>, bool)>> {
        self.task_runtimes.get(task_id)?
            .codex_slot.as_ref()?
            .inject_tx.clone()
    }

    /// Collect the set of ports currently allocated to *online* Codex task slots.
    pub fn codex_used_ports(&self) -> std::collections::HashSet<u16> {
        self.task_runtimes.values()
            .filter_map(|rt| rt.codex_slot.as_ref())
            .filter(|slot| slot.is_online())
            .map(|slot| slot.port)
            .collect()
    }

    pub fn is_codex_online(&self) -> bool {
        if self.task_runtimes.values().any(|rt| {
            rt.codex_slot.as_ref().map_or(false, |s| s.is_online())
        }) {
            return true;
        }
        self.codex_inject_tx.is_some()
    }

    pub fn is_agent_online(&self, agent: &str) -> bool {
        match agent {
            "claude" => self.is_claude_sdk_online(),
            "codex" => self.is_codex_online(),
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

    pub fn clear_provider_connection(&mut self, agent: &str) -> Option<String> {
        let task_id = self.detach_provider_binding(agent);
        match agent {
            "claude" => self.claude_connection = None,
            "codex" => self.codex_connection = None,
            _ => {}
        }
        task_id
    }

    pub fn set_runtime_health(&mut self, health: RuntimeHealthStatus) {
        self.runtime_health = Some(health);
    }

    pub fn clear_runtime_health(&mut self) {
        self.runtime_health = None;
    }

    pub fn teardown_runtime_handles_for_shutdown(&mut self) {
        self.attached_agents.clear();
        self.buffered_messages.clear();
        self.pending_permissions.clear();
        self.buffered_verdicts.clear();
        self.invalidate_codex_session();
        self.invalidate_claude_sdk_session();
        self.telegram_outbound_tx = None;
        self.telegram_paired_chat_id = None;
        self.clear_runtime_health();
    }
}
