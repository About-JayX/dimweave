use crate::daemon::{gui, types, SharedState};
use tauri::AppHandle;

pub async fn handle_permission_verdict(
    state: &SharedState,
    app: &AppHandle,
    request_id: String,
    behavior: types::PermissionBehavior,
) {
    let resolved = {
        let mut daemon = state.write().await;
        daemon.resolve_permission(
            &request_id,
            behavior,
            chrono::Utc::now().timestamp_millis() as u64,
        )
    };
    let Some((agent_id, outbound)) = resolved else {
        gui::emit_system_log(
            app,
            "warn",
            &format!("[Daemon] permission {request_id} unknown/expired"),
        );
        return;
    };
    let verdict = match &outbound {
        types::ToAgent::PermissionVerdict { verdict } => Some(verdict.clone()),
        _ => None,
    };

    // Try Claude SDK WS path first (if this is a "claude" agent)
    let sdk_delivered = if agent_id == "claude" {
        if let Some(v) = &verdict {
            let sdk_tx = state.read().await.claude_sdk_ws_tx.clone();
            if let Some(tx) = sdk_tx {
                let ndjson = crate::daemon::claude_sdk::protocol::format_control_response(
                    &v.request_id,
                    v.behavior == types::PermissionBehavior::Allow,
                );
                tx.send(ndjson).await.is_ok()
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if sdk_delivered {
        gui::emit_system_log(
            app,
            "info",
            &format!("[Daemon] verdict delivered to {agent_id} via SDK WS"),
        );
        return;
    }

    // Fallback: bridge path via attached_agents
    let sender_tx = state
        .read()
        .await
        .attached_agents
        .get(&agent_id)
        .map(|s| s.tx.clone());
    match sender_tx {
        Some(tx) if tx.send(outbound).await.is_ok() => {
            gui::emit_system_log(
                app,
                "info",
                &format!("[Daemon] verdict delivered to {agent_id}"),
            );
        }
        _ => {
            if let Some(v) = verdict {
                state.write().await.buffer_permission_verdict(&agent_id, v);
            }
            gui::emit_system_log(
                app,
                "warn",
                &format!("[Daemon] {agent_id} offline, buffered verdict {request_id}"),
            );
        }
    }
}
