use crate::daemon::{
    gui::{self, ClaudeStreamPayload},
    routing,
    types::{FromAgent, MessageSource, MessageStatus, ToAgent},
    SharedState,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;

fn is_allowed_agent(agent_id: &str) -> bool {
    matches!(agent_id, "claude" | "codex")
}

/// Validate an inbound `sender_agent_id` claim against the task's agent
/// records. Returns `(role, agent_id, display_source)` if the claimed id
/// matches a registered agent whose provider matches the runtime.
fn validate_claimed_agent_id(
    s: &crate::daemon::state::DaemonState,
    runtime_id: &str,
    claimed: &str,
) -> Option<(String, String, String)> {
    let task_id = s.agent_owning_task_id(runtime_id)?;
    let expected = match runtime_id {
        "claude" => crate::daemon::task_graph::types::Provider::Claude,
        "codex" => crate::daemon::task_graph::types::Provider::Codex,
        _ => return None,
    };
    let agents = s.task_graph.agents_for_task(&task_id);
    let agent = agents
        .iter()
        .find(|a| a.agent_id == claimed && a.provider == expected)?;
    Some((
        agent.role.clone(),
        agent.agent_id.clone(),
        runtime_id.to_string(),
    ))
}

/// Resolve (role, agent_id, display_source) directly from the task identity
/// the bridge declared in its AgentConnect handshake. This is the only
/// multi-task-safe path — it never scans task_runtimes and never relies
/// on HashMap iteration order.
fn resolve_from_connection_task(
    s: &crate::daemon::state::DaemonState,
    runtime_id: &str,
    task_id: &str,
    task_agent_id: &str,
) -> Option<(String, String, String)> {
    let expected = match runtime_id {
        "claude" => crate::daemon::task_graph::types::Provider::Claude,
        "codex" => crate::daemon::task_graph::types::Provider::Codex,
        _ => return None,
    };
    let agents = s.task_graph.agents_for_task(task_id);
    let agent = agents
        .iter()
        .find(|a| a.agent_id == task_agent_id && a.provider == expected)?;
    Some((
        agent.role.clone(),
        agent.agent_id.clone(),
        runtime_id.to_string(),
    ))
}

/// Resolve (role, agent_id, display_source) from the concrete online slot's
/// agent_id, then task_agents, then global singletons.
fn resolve_agent_identity(
    s: &crate::daemon::state::DaemonState,
    runtime_id: &str,
) -> (String, String, String) {
    if let Some(task_id) = s.agent_owning_task_id(runtime_id) {
        // Look up the concrete agent_id from the online runtime slot
        let concrete_id = match runtime_id {
            "claude" => s
                .task_runtimes
                .get(&task_id)
                .and_then(|rt| {
                    rt.all_claude_slots()
                        .find(|slot| slot.is_online())
                        .and_then(|slot| slot.agent_id.clone())
                }),
            "codex" => s
                .task_runtimes
                .get(&task_id)
                .and_then(|rt| {
                    rt.all_codex_slots()
                        .find(|slot| slot.is_online())
                        .and_then(|slot| slot.agent_id.clone())
                }),
            _ => None,
        };
        let agents = s.task_graph.agents_for_task(&task_id);
        // Match by concrete slot agent_id first; only fall back to provider
        // match when exactly one agent uses this provider (avoids ambiguity
        // when multiple same-provider agents exist).
        let matched = concrete_id
            .as_deref()
            .and_then(|cid| agents.iter().find(|a| a.agent_id == cid))
            .or_else(|| {
                let prov = match runtime_id {
                    "claude" => Some(crate::daemon::task_graph::types::Provider::Claude),
                    "codex" => Some(crate::daemon::task_graph::types::Provider::Codex),
                    _ => None,
                };
                prov.and_then(|p| {
                    let matching: Vec<_> =
                        agents.iter().filter(|a| a.provider == p).collect();
                    if matching.len() == 1 {
                        Some(matching[0])
                    } else {
                        None
                    }
                })
            });
        if let Some(agent) = matched {
            return (
                agent.role.clone(),
                agent.agent_id.clone(),
                runtime_id.to_string(),
            );
        }
    }
    let role = match runtime_id {
        "claude" => s.claude_role.clone(),
        "codex" => s.codex_role.clone(),
        _ => runtime_id.to_string(),
    };
    (role, runtime_id.to_string(), runtime_id.to_string())
}

fn claude_terminal_reply_claims_visible_result(
    message: &crate::daemon::types::BridgeMessage,
) -> bool {
    message.is_to_user()
        && message.status.is_some_and(|s| s.is_terminal())
        && !message.message.trim().is_empty()
}

fn summarize_bridge_message_shape(message: &crate::daemon::types::BridgeMessage) -> String {
    format!(
        "BridgeMessage{{id,source,target,reply_target,message,timestamp,reply_to,priority,status,task_id,session_id,attachments}} source={} target={} status={} message_len={} task_id={} session_id={} agent_id={} attachments={}",
        message.source_role(),
        message.target_str(),
        message.status.map(MessageStatus::as_str).unwrap_or("none"),
        message.message.len(),
        message.task_id.as_deref().unwrap_or("-"),
        message.session_id.as_deref().unwrap_or("-"),
        message.source_agent_id().unwrap_or("-"),
        message.attachments.as_ref().map_or(0, |a| a.len()),
    )
}

pub async fn handle_connection(socket: WebSocket, state: SharedState, app: AppHandle) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ToAgent>(64);
    let mut agent_id: Option<String> = None;
    // Task identity this WS instance reported during AgentConnect. When set,
    // it is authoritative for stamping AgentReply / PermissionRequest events,
    // so multi-task deployments don't fall back to the racy
    // agent_owning_task_id(runtime_id) scan.
    let mut connection_task: Option<(String, String)> = None;
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
            FromAgent::AgentConnect {
                agent_id: id,
                task_id: reported_task_id,
                task_agent_id: reported_task_agent_id,
            } => {
                if !is_allowed_agent(&id) {
                    gui::emit_system_log(&app, "warn", &format!("[Control] rejected agent {id}"));
                    break;
                }
                agent_id = Some(id.clone());
                if let (Some(tid), Some(aid)) = (
                    reported_task_id.as_deref(),
                    reported_task_agent_id.as_deref(),
                ) {
                    connection_task = Some((tid.to_string(), aid.to_string()));
                    gui::emit_system_log(
                        &app,
                        "info",
                        &format!(
                            "[Control] {id} handshake declared task_id={tid} task_agent_id={aid}"
                        ),
                    );
                } else if reported_task_id.is_some() || reported_task_agent_id.is_some() {
                    gui::emit_system_log(
                        &app,
                        "warn",
                        &format!(
                            "[Control] {id} handshake only partial: task_id={:?} task_agent_id={:?}",
                            reported_task_id, reported_task_agent_id,
                        ),
                    );
                }
                let (buffered_messages, buffered_verdicts, runtime_role) = {
                    let mut daemon = state.write().await;
                    let role = match id.as_str() {
                        "claude" => Some(daemon.claude_role.clone()),
                        "codex" => Some(daemon.codex_role.clone()),
                        _ => None,
                    };
                    let gen = daemon.next_agent_gen;
                    daemon.next_agent_gen += 1;
                    my_gen = gen;
                    if daemon.attached_agents.contains_key(&id) {
                        gui::emit_system_log(
                            &app,
                            "warn",
                            &format!(
                                "[Control] attached_agents['{id}'] overwritten by new \
                                 bridge connection (multi-task scenario); previous \
                                 slot loses singleton sender"
                            ),
                        );
                    }
                    daemon.attached_agents.insert(
                        id.clone(),
                        crate::daemon::state::AgentSender::new(tx.clone(), gen),
                    );
                    let mut buffered = role
                        .as_deref()
                        .map(|role_id| daemon.take_buffered_for(role_id))
                        .unwrap_or_default();
                    // Also take agent-targeted messages (msg.to == agent_id)
                    if role.as_deref() != Some(id.as_str()) {
                        buffered.extend(daemon.take_buffered_for(&id));
                    }
                    let verdicts = daemon.take_buffered_verdicts_for(&id);
                    (buffered, verdicts, role)
                };
                replay_messages(&tx, &state, buffered_messages).await;
                replay_verdicts(&tx, &state, &id, buffered_verdicts).await;
                let (provider_session, surfaced_online) = {
                    let daemon = state.read().await;
                    (daemon.provider_connection(&id), daemon.is_agent_online(&id))
                };
                if surfaced_online {
                    if let Some(role) = runtime_role {
                        gui::emit_agent_status_online(&app, &id, provider_session, role);
                    } else {
                        gui::emit_agent_status(&app, &id, true, None, provider_session);
                    }
                }
                gui::emit_system_log(&app, "info", &format!("[Control] {id} connected"));
            }
            FromAgent::AgentReply { mut message } => {
                // Bind message.from to the authenticated agent's role
                // (prevents spoofing — bridge can't claim to be a different sender)
                let mut suppress_message = false;
                let mut bridge_claimed_delivery = false;
                if let Some(id) = agent_id.as_deref() {
                    // Preserve inbound sender_agent_id when it validates against
                    // the task's agents; otherwise resolve from runtime state.
                    let claimed_agent_id = message.source_agent_id().map(|s| s.to_string());
                    let (role, real_agent_id, display_src) = {
                        let s = state.read().await;
                        // Priority 1: handshake-declared task identity
                        // (multi-task correct; never falls back to racy scan).
                        connection_task
                            .as_ref()
                            .and_then(|(tid, aid)| {
                                resolve_from_connection_task(&s, id, tid, aid)
                            })
                            // Priority 2: inbound message claim validated against
                            // the runtime-guessed task (legacy single-task path).
                            .or_else(|| {
                                claimed_agent_id
                                    .as_deref()
                                    .and_then(|claimed| validate_claimed_agent_id(&s, id, claimed))
                            })
                            // Priority 3: first-online scan (pre-handshake-fix fallback;
                            // correct only when at most one task is online).
                            .unwrap_or_else(|| resolve_agent_identity(&s, id))
                    };
                    let provider = if id == "claude" {
                        crate::daemon::task_graph::types::Provider::Claude
                    } else {
                        crate::daemon::task_graph::types::Provider::Codex
                    };
                    message.source = MessageSource::Agent {
                        agent_id: real_agent_id,
                        role: role.clone(),
                        provider,
                        display_source: Some(display_src),
                    };
                    let status = message.status.unwrap_or(MessageStatus::Done);
                    message.status = Some(status);
                    {
                        let s = state.read().await;
                        // Prefer handshake-declared task_id; fall back to legacy
                        // scan only when the connection didn't report one.
                        let stamp_task_id = connection_task
                            .as_ref()
                            .map(|(tid, _)| tid.clone())
                            .or_else(|| s.agent_owning_task_id(id));
                        if let Some(task_id) = stamp_task_id {
                            s.stamp_message_context_for_task(&task_id, &role, &mut message);
                        } else {
                            s.stamp_message_context(&role, &mut message);
                        }
                    }
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
                        if claude_terminal_reply_claims_visible_result(&message) {
                            let should_route =
                                state.write().await.claim_claude_bridge_terminal_delivery();
                            if should_route {
                                // Defer Done until after route_message so the
                                // durable bubble arrives before the draft clears.
                                bridge_claimed_delivery = true;
                            } else {
                                suppress_message = true;
                                state.write().await.finish_claude_sdk_direct_text_turn();
                                let (ctid, caid) = connection_task
                                    .as_ref()
                                    .map(|(t, a)| (Some(t.as_str()), Some(a.as_str())))
                                    .unwrap_or((None, None));
                                gui::emit_claude_stream(&app, ctid, caid, ClaudeStreamPayload::Done);
                                gui::emit_system_log(
                                    &app,
                                    "info",
                                    "[Control] suppressed duplicate Claude terminal reply after SDK fallback",
                                );
                            }
                        } else {
                            // Non-user-targeted or empty terminal replies end the
                            // visible thinking state without claiming final-message
                            // ownership — SDK result can still deliver to the user.
                            let (ctid, caid) = connection_task
                                .as_ref()
                                .map(|(t, a)| (Some(t.as_str()), Some(a.as_str())))
                                .unwrap_or((None, None));
                            gui::emit_claude_stream(&app, ctid, caid, ClaudeStreamPayload::Done);
                        }
                    }
                }
                if suppress_message || message.message.trim().is_empty() {
                    continue;
                }
                routing::route_message(&state, &app, message).await;
                if bridge_claimed_delivery {
                    let (ctid, caid) = connection_task
                        .as_ref()
                        .map(|(t, a)| (Some(t.as_str()), Some(a.as_str())))
                        .unwrap_or((None, None));
                    gui::emit_claude_stream(&app, ctid, caid, ClaudeStreamPayload::Done);
                }
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
                // Prefer the task identity this bridge declared at handshake
                // over the legacy scan; scan races between concurrent tasks.
                let owning_task_id = match connection_task.as_ref() {
                    Some((tid, _)) => Some(tid.clone()),
                    None => state.read().await.agent_owning_task_id(id),
                };
                let owning_agent_id = connection_task.as_ref().map(|(_, aid)| aid.clone());
                gui::emit_permission_prompt(
                    &app,
                    id,
                    owning_task_id.as_deref(),
                    owning_agent_id.as_deref(),
                    &request,
                    created_at,
                );
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
            let task_id = if !claude_sdk_still_online {
                daemon.clear_provider_connection(id)
            } else {
                None
            };
            drop(daemon);
            if id == "claude" && !claude_sdk_still_online {
                let (ctid, caid) = connection_task
                    .as_ref()
                    .map(|(t, a)| (Some(t.as_str()), Some(a.as_str())))
                    .unwrap_or((None, None));
                gui::emit_claude_stream(&app, ctid, caid, ClaudeStreamPayload::Reset);
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
            if let Some(task_id) = task_id {
                crate::daemon::gui_task::emit_task_context_events(
                    &state, &app, &task_id,
                ).await;
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
    use super::{
        claude_terminal_reply_claims_visible_result, resolve_agent_identity,
        summarize_bridge_message_shape, validate_claimed_agent_id,
    };
    use crate::daemon::state::DaemonState;
    use crate::daemon::task_graph::types::Provider;
    use crate::daemon::types::{BridgeMessage, MessageSource, MessageStatus, MessageTarget};

    fn make_msg(to: &str, content: &str, status: MessageStatus) -> BridgeMessage {
        let target = if to == "user" {
            MessageTarget::User
        } else {
            MessageTarget::Role { role: to.to_string() }
        };
        BridgeMessage {
            id: "test".into(),
            source: MessageSource::Agent {
                agent_id: "claude".into(),
                role: "lead".into(),
                provider: Provider::Claude,
                display_source: Some("claude".into()),
            },
            target,
            reply_target: None,
            message: content.to_string(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: Some(status),
            task_id: None,
            session_id: None,
            attachments: None,
        }
    }

    #[test]
    fn empty_terminal_claude_reply_only_ends_thinking() {
        assert!(!claude_terminal_reply_claims_visible_result(&make_msg(
            "user",
            "   ",
            MessageStatus::Done
        )));
        assert!(!claude_terminal_reply_claims_visible_result(&make_msg(
            "user",
            "",
            MessageStatus::Error
        )));
    }

    #[test]
    fn non_empty_terminal_claude_reply_claims_visible_result() {
        assert!(claude_terminal_reply_claims_visible_result(&make_msg(
            "user",
            "final reply",
            MessageStatus::Done
        )));
        assert!(claude_terminal_reply_claims_visible_result(&make_msg(
            "user",
            "blocked",
            MessageStatus::Error
        )));
    }

    #[test]
    fn terminal_bridge_reply_to_user_claims_visible_result() {
        // A terminal bridge reply addressed to the user with non-empty content
        // must claim visible-result ownership.
        assert!(claude_terminal_reply_claims_visible_result(&make_msg(
            "user",
            "Final answer for the user.",
            MessageStatus::Done,
        )));
    }

    #[test]
    fn terminal_bridge_handoff_to_worker_does_not_claim_visible_result() {
        // A terminal bridge reply routed to a worker role (lead/coder, not user)
        // must NOT claim visible-result ownership — the SDK result should still
        // be able to deliver the user-visible message.
        assert!(!claude_terminal_reply_claims_visible_result(&make_msg(
            "coder",
            "Implementation complete, reporting to lead.",
            MessageStatus::Done,
        )));
        assert!(!claude_terminal_reply_claims_visible_result(&make_msg(
            "lead",
            "Implementation complete.",
            MessageStatus::Done,
        )));
    }

    #[test]
    fn summarize_bridge_reply_reports_shape_and_lengths() {
        let message = BridgeMessage {
            id: "msg-1".into(),
            source: MessageSource::Agent {
                agent_id: "claude".into(),
                role: "lead".into(),
                provider: Provider::Claude,
                display_source: Some("claude".into()),
            },
            target: MessageTarget::User,
            reply_target: None,
            message: "final answer".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: Some(MessageStatus::Done),
            task_id: Some("task-1".into()),
            session_id: Some("session-1".into()),
            attachments: None,
        };

        let summary = summarize_bridge_message_shape(&message);

        assert!(summary.contains("BridgeMessage{id,source,target,reply_target,message,timestamp,reply_to,priority,status,task_id,session_id,attachments}"));
        assert!(summary.contains("source=lead"));
        assert!(summary.contains("target=user"));
        assert!(summary.contains("status=done"));
        assert!(summary.contains("message_len=12"));
        assert!(summary.contains("task_id=task-1"));
        assert!(summary.contains("session_id=session-1"));
    }

    #[test]
    fn validate_claimed_agent_id_accepts_matching_task_agent() {
        let mut s = DaemonState::new();
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        let agent = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        s.init_task_runtime(&task.task_id, "/ws".into());
        let slot = s
            .task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&agent.agent_id);
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        slot.ws_tx = Some(tx);

        let result = validate_claimed_agent_id(&s, "claude", &agent.agent_id);
        assert!(result.is_some(), "valid claimed agent_id should validate");
        let (role, aid, disp) = result.unwrap();
        assert_eq!(role, "coder");
        assert_eq!(aid, agent.agent_id);
        assert_eq!(disp, "claude");
    }

    #[test]
    fn validate_claimed_agent_id_rejects_wrong_provider() {
        let mut s = DaemonState::new();
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        let agent = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "lead");
        s.init_task_runtime(&task.task_id, "/ws".into());
        let slot = s
            .task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&agent.agent_id);
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        slot.ws_tx = Some(tx);

        // Claim a Claude agent_id via "codex" runtime — should reject
        let result = validate_claimed_agent_id(&s, "codex", &agent.agent_id);
        assert!(
            result.is_none(),
            "agent_id on wrong provider runtime must not validate"
        );
    }

    #[test]
    fn validate_claimed_agent_id_rejects_unknown_id() {
        let mut s = DaemonState::new();
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        s.task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "lead");
        s.init_task_runtime(&task.task_id, "/ws".into());
        let slot = s
            .task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot("real-agent");
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        slot.ws_tx = Some(tx);

        let result = validate_claimed_agent_id(&s, "claude", "bogus-agent-id");
        assert!(result.is_none(), "unknown agent_id must not validate");
    }

    /// Two Claude agents in the same task. `resolve_agent_identity` picks first
    /// online slot, but `validate_claimed_agent_id` distinguishes them.
    #[test]
    fn validate_distinguishes_two_same_provider_agents() {
        let mut s = DaemonState::new();
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        let agent_a = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "lead");
        let agent_b = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        s.init_task_runtime(&task.task_id, "/ws".into());
        let (tx_a, _rx_a) = tokio::sync::mpsc::channel::<String>(1);
        let (tx_b, _rx_b) = tokio::sync::mpsc::channel::<String>(1);
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&agent_a.agent_id)
            .ws_tx = Some(tx_a);
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&agent_b.agent_id)
            .ws_tx = Some(tx_b);

        // Claiming agent_a returns lead role
        let (role_a, id_a, _) =
            validate_claimed_agent_id(&s, "claude", &agent_a.agent_id).unwrap();
        assert_eq!(role_a, "lead");
        assert_eq!(id_a, agent_a.agent_id);

        // Claiming agent_b returns coder role
        let (role_b, id_b, _) =
            validate_claimed_agent_id(&s, "claude", &agent_b.agent_id).unwrap();
        assert_eq!(role_b, "coder");
        assert_eq!(id_b, agent_b.agent_id);

        // When slots have concrete agent_ids, resolve finds the first online
        // slot by agent_id.  validate_claimed_agent_id is the precise path.
        let (resolved_role, _, _) = resolve_agent_identity(&s, "claude");
        assert!(!resolved_role.is_empty());
    }

    /// Two Claude agents, legacy slot without agent_id — ambiguous provider
    /// fallback must NOT pick one arbitrarily; falls to global singleton.
    #[test]
    fn resolve_agent_identity_ambiguous_falls_to_global() {
        let mut s = DaemonState::new();
        s.claude_role = "lead".into();
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        s.task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "lead");
        s.task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        s.init_task_runtime(&task.task_id, "/ws".into());
        // Legacy slot: online but agent_id is None
        let slot = s
            .task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot("__default");
        slot.agent_id = None;
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        slot.ws_tx = Some(tx);

        let (role, aid, _) = resolve_agent_identity(&s, "claude");
        // Two agents match Claude — ambiguous.  Must fall to global singleton.
        assert_eq!(role, "lead");
        assert_eq!(aid, "claude");
    }

    /// Single Claude agent in task — unambiguous provider fallback works.
    #[test]
    fn resolve_agent_identity_single_provider_agent() {
        let mut s = DaemonState::new();
        s.claude_role = "lead".into();
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        let agent = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        s.init_task_runtime(&task.task_id, "/ws".into());
        // Legacy slot: online but agent_id is None
        let slot = s
            .task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot("__default");
        slot.agent_id = None;
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        slot.ws_tx = Some(tx);

        // Only one Claude agent — unambiguous, should resolve to it
        let (role, aid, _) = resolve_agent_identity(&s, "claude");
        assert_eq!(role, "coder");
        assert_eq!(aid, agent.agent_id);
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
