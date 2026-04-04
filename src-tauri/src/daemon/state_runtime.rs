use super::*;

impl DaemonState {
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
        tx: mpsc::Sender<(Vec<serde_json::Value>, bool)>,
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

    pub fn invalidate_claude_sdk_session(&mut self) {
        self.advance_claude_sdk_epoch();
        self.claude_sdk_ws_tx = None;
        self.claude_sdk_event_tx = None;
        self.claude_sdk_ready_tx = None;
        self.claude_sdk_pending_nonce = None;
        self.claude_sdk_active_nonce = None;
        self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Inactive;
        self.clear_claude_preview_batch();
        self.claude_connection = None;
    }

    pub fn invalidate_claude_sdk_session_if_current(&mut self, epoch: u64) -> bool {
        if self.claude_sdk_session_epoch != epoch {
            return false;
        }
        self.invalidate_claude_sdk_session();
        true
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

    pub fn is_claude_sdk_online(&self) -> bool {
        self.claude_sdk_ws_tx.is_some()
    }

    pub fn is_agent_online(&self, agent: &str) -> bool {
        match agent {
            "claude" => self.attached_agents.contains_key("claude") || self.is_claude_sdk_online(),
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

    pub fn set_runtime_health(&mut self, health: RuntimeHealthStatus) {
        self.runtime_health = Some(health);
    }

    pub fn clear_runtime_health(&mut self) {
        self.runtime_health = None;
    }
}
