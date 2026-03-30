use super::*;

impl DaemonState {
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
