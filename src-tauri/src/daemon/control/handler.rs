use crate::daemon::{
    gui::{self, ClaudeStreamPayload},
    routing,
    types::{FromAgent, MessageStatus, ToAgent},
    SharedState,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;

fn is_allowed_agent(agent_id: &str) -> bool {
    matches!(agent_id, "claude" | "codex")
}

fn claude_terminal_reply_claims_visible_result(status: MessageStatus, content: &str) -> bool {
    status.is_terminal() && !content.trim().is_empty()
}

fn summarize_bridge_message_shape(message: &crate::daemon::types::BridgeMessage) -> String {
    format!(
        "BridgeMessage{{id,from,display_source,to,content,timestamp,reply_to,priority,status,task_id,session_id,sender_agent_id}} from={} to={} status={} content_len={} task_id={} session_id={} sender_agent_id={}",
        message.from,
        message.to,
        message.status.map(MessageStatus::as_str).unwrap_or("none"),
        message.content.len(),
        message.task_id.as_deref().unwrap_or("-"),
        message.session_id.as_deref().unwrap_or("-"),
        message.sender_agent_id.as_deref().unwrap_or("-"),
    )
}

pub async fn handle_connection(socket: WebSocket, state: SharedState, app: AppHandle) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ToAgent>(64);
    let mut agent_id: Option<String> = None;
    let mut my_gen: u64 = 0;

    // Forward outbound messages to WS sink
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let Ok(payload) = serde_json::to_string(&msg) else {
                eprintln!("[Control] failed to serialize outbound message");
                continue;
            };
            if sink.send(Message::Text(payload)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(txt) = msg else { continue };
        let Ok(from_agent) = serde_json::from_str::<FromAgent>(&txt) else {
            continue;
        };

        match from_agent {
            FromAgent::AgentConnect { agent_id: id } => {
                if !is_allowed_agent(&id) {
                    gui::emit_system_log(&app, "warn", &format!("[Control] rejected agent {id}"));
                    break;
                }
                agent_id = Some(id.clone());
                let (buffered_messages, buffered_verdicts) = {
                    let mut daemon = state.write().await;
                    let role = match id.as_str() {
                        "claude" => Some(daemon.claude_role.clone()),
                        "codex" => Some(daemon.codex_role.clone()),
                        _ => None,
                    };
                    if let Some(conflict_role) = role.as_deref() {
                        if let Some(conflict_agent) =
                            daemon.online_role_conflict(&id, conflict_role)
                        {
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!(
                                    "[Control] rejected {id} connection: role '{conflict_role}' already in use by online {conflict_agent}"
                                ),
                            );
                            break;
                        }
                    }
                    let gen = daemon.next_agent_gen;
                    daemon.next_agent_gen += 1;
                    my_gen = gen;
                    daemon.attached_agents.insert(
                        id.clone(),
                        crate::daemon::state::AgentSender::new(tx.clone(), gen),
                    );
                    (
                        role.map(|role_id| daemon.take_buffered_for(&role_id))
                            .unwrap_or_default(),
                        daemon.take_buffered_verdicts_for(&id),
                    )
                };
                replay_messages(&tx, &state, buffered_messages).await;
                replay_verdicts(&tx, &state, &id, buffered_verdicts).await;
                let provider_session = state.read().await.provider_connection(&id);
                gui::emit_agent_status(&app, &id, true, None, provider_session);
                gui::emit_system_log(&app, "info", &format!("[Control] {id} connected"));
            }
            FromAgent::AgentReply { mut message } => {
                // Bind message.from to the authenticated agent's role
                // (prevents spoofing — bridge can't claim to be a different sender)
                let mut suppress_message = false;
                if let Some(id) = agent_id.as_deref() {
                    let role = {
                        let s = state.read().await;
                        match id {
                            "claude" => s.claude_role.clone(),
                            "codex" => s.codex_role.clone(),
                            _ => id.to_string(),
                        }
                    };
                    message.from = role.clone();
                    message.display_source = Some(id.to_string());
                    message.sender_agent_id = Some(id.to_string());
                    let status = message.status.unwrap_or(MessageStatus::Done);
                    message.status = Some(status);
                    state
                        .read()
                        .await
                        .stamp_message_context(&role, &mut message);
                    if id == "claude" {
                        gui::emit_system_log(
                            &app,
                            "info",
                            &format!(
                                "[Claude Trace] chain=bridge_reply {}",
                                summarize_bridge_message_shape(&message)
                            ),
                        );
                    }
                    if id == "claude" && status.is_terminal() {
                        if claude_terminal_reply_claims_visible_result(status, &message.content) {
                            let should_route = state
                                .write()
                                .await
                                .claim_claude_bridge_terminal_delivery();
                            if should_route {
                                gui::emit_claude_stream(&app, ClaudeStreamPayload::Done);
                            } else {
                                suppress_message = true;
                                state.write().await.finish_claude_sdk_direct_text_turn();
                                gui::emit_system_log(
                                    &app,
                                    "info",
                                    "[Control] suppressed duplicate Claude terminal reply after SDK fallback",
                                );
                            }
                        } else {
                            // Empty terminal replies are allowed to end the visible
                            // thinking state without stealing final-message ownership
                            // from an in-flight SDK fallback turn.
                            gui::emit_claude_stream(&app, ClaudeStreamPayload::Done);
                        }
                    }
                }
                if suppress_message || message.content.trim().is_empty() {
                    continue;
                }
                routing::route_message(&state, &app, message).await;
            }
            FromAgent::PermissionRequest { request } => {
                let Some(id) = agent_id.as_deref() else {
                    continue;
                };
                let created_at = chrono::Utc::now().timestamp_millis() as u64;
                state
                    .write()
                    .await
                    .store_permission_request(id, request.clone(), created_at);
                gui::emit_permission_prompt(&app, id, &request, created_at);
                gui::emit_system_log(
                    &app,
                    "info",
                    &format!(
                        "[Control] permission request {} from {} for {}",
                        request.request_id, id, request.tool_name
                    ),
                );
            }
            FromAgent::GetOnlineAgents => {
                let snapshot = state.read().await.online_agents_snapshot();
                let payload = serde_json::to_value(&snapshot).unwrap_or_default();
                let _ = tx
                    .send(ToAgent::OnlineAgentsResponse {
                        online_agents: payload,
                    })
                    .await;
            }
            FromAgent::AgentDisconnect => break,
        }
    }

    if let Some(id) = &agent_id {
        let mut daemon = state.write().await;
        let is_ours = daemon
            .attached_agents
            .get(id.as_str())
            .is_some_and(|s| s.gen == my_gen);
        if is_ours {
            let claude_sdk_still_online = id == "claude" && daemon.is_claude_sdk_online();
            daemon.attached_agents.remove(id);
            if !claude_sdk_still_online {
                daemon.clear_provider_connection(id);
            }
            drop(daemon);
            if id == "claude" && !claude_sdk_still_online {
                gui::emit_claude_stream(&app, ClaudeStreamPayload::Reset);
            }
            if claude_sdk_still_online {
                gui::emit_system_log(
                    &app,
                    "info",
                    "[Control] claude MCP bridge disconnected; SDK session still online",
                );
            } else {
                gui::emit_agent_status(&app, id, false, None, None);
                gui::emit_system_log(&app, "info", &format!("[Control] {id} disconnected"));
            }
        } else {
            drop(daemon);
            gui::emit_system_log(
                &app,
                "info",
                &format!("[Control] {id} stale connection closed"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{claude_terminal_reply_claims_visible_result, summarize_bridge_message_shape};
    use crate::daemon::types::{BridgeMessage, MessageStatus};

    #[test]
    fn empty_terminal_claude_reply_only_ends_thinking() {
        assert!(!claude_terminal_reply_claims_visible_result(
            MessageStatus::Done,
            "   "
        ));
        assert!(!claude_terminal_reply_claims_visible_result(
            MessageStatus::Error,
            ""
        ));
    }

    #[test]
    fn non_empty_terminal_claude_reply_claims_visible_result() {
        assert!(claude_terminal_reply_claims_visible_result(
            MessageStatus::Done,
            "final reply"
        ));
        assert!(claude_terminal_reply_claims_visible_result(
            MessageStatus::Error,
            "blocked"
        ));
    }

    #[test]
    fn summarize_bridge_reply_reports_shape_and_lengths() {
        let message = BridgeMessage {
            id: "msg-1".into(),
            from: "lead".into(),
            display_source: Some("claude".into()),
            to: "user".into(),
            content: "final answer".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: Some(MessageStatus::Done),
            task_id: Some("task-1".into()),
            session_id: Some("session-1".into()),
            sender_agent_id: Some("claude".into()),
        };

        let summary = summarize_bridge_message_shape(&message);

        assert!(summary.contains("BridgeMessage{id,from,display_source,to,content,timestamp,reply_to,priority,status,task_id,session_id,sender_agent_id}"));
        assert!(summary.contains("from=lead"));
        assert!(summary.contains("to=user"));
        assert!(summary.contains("status=done"));
        assert!(summary.contains("content_len=12"));
        assert!(summary.contains("task_id=task-1"));
        assert!(summary.contains("session_id=session-1"));
    }
}

async fn replay_messages(
    tx: &mpsc::Sender<ToAgent>,
    state: &SharedState,
    mut msgs: Vec<crate::daemon::types::BridgeMessage>,
) {
    let mut i = 0;
    while i < msgs.len() {
        if tx
            .send(ToAgent::RoutedMessage {
                message: msgs[i].clone(),
            })
            .await
            .is_err()
        {
            let mut s = state.write().await;
            for m in msgs.drain(i..) {
                s.buffer_message(m);
            }
            eprintln!(
                "[Control] replay failed, re-buffered {} messages",
                msgs.len() - i
            );
            return;
        }
        i += 1;
    }
}

async fn replay_verdicts(
    tx: &mpsc::Sender<ToAgent>,
    state: &SharedState,
    agent_id: &str,
    mut verdicts: Vec<crate::daemon::types::PermissionVerdict>,
) {
    let mut i = 0;
    while i < verdicts.len() {
        if tx
            .send(ToAgent::PermissionVerdict {
                verdict: verdicts[i].clone(),
            })
            .await
            .is_err()
        {
            let mut s = state.write().await;
            for v in verdicts.drain(i..) {
                s.buffer_permission_verdict(agent_id, v);
            }
            return;
        }
        i += 1;
    }
}
