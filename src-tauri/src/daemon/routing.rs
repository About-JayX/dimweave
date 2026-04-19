#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets;
#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets_for_task;
pub use crate::daemon::routing_user_input::route_user_input;
use crate::daemon::{
    routing_display,
    types::{BridgeMessage, MessageTarget, ToAgent},
    SharedState,
};
use tauri::AppHandle;

// ── reply-target tracking ────────────────────────────────────
// When A delegates to B (agent-targeted), record B → (A, A_role).
// When B later replies to A_role, redirect to A's specific agent.

fn reply_target_map() -> &'static std::sync::Mutex<std::collections::HashMap<String, (String, String)>> {
    static MAP: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (String, String)>>> = std::sync::OnceLock::new();
    MAP.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn apply_reply_target(sender_agent_id: &str, target: &str) -> Option<String> {
    if matches!(target, "user" | "claude" | "codex") { return None; }
    let guard = reply_target_map().lock().unwrap();
    let (agent_id, role) = guard.get(sender_agent_id)?;
    if role == target { Some(agent_id.clone()) } else { None }
}

/// Returns the specific agent_id that delegated to `sender_agent_id`, if
/// recorded. Used by daemon-fabricated diagnostics (worker silent turn,
/// parse errors, dropped messages) to route back to the exact delegating
/// agent rather than falling to `User` or broadcasting to a `Role` target.
pub fn delegator_agent_id(sender_agent_id: &str) -> Option<String> {
    let guard = reply_target_map().lock().unwrap();
    guard.get(sender_agent_id).map(|(id, _)| id.clone())
}

fn record_reply_target(recipient_id: &str, sender_id: &str, sender_role: &str) {
    reply_target_map().lock().unwrap().insert(
        recipient_id.to_string(),
        (sender_id.to_string(), sender_role.to_string()),
    );
}

#[cfg(test)]
fn clear_reply_targets() {
    reply_target_map().lock().unwrap().clear();
}

#[path = "routing_dispatch.rs"]
mod dispatch;
pub use dispatch::{route_message, route_message_silent};

#[derive(Clone, Copy)]
pub enum RouteResult {
    Delivered,
    Buffered,
    Dropped,
    ToGui,
}

struct RouteOutcome {
    result: RouteResult,
    emit_claude_thinking: bool,
    buffer_reason: Option<&'static str>,
}

async fn route_message_inner_with_meta(state: &SharedState, msg: BridgeMessage) -> RouteOutcome {
    let decision = {
        let mut s = state.write().await;
        s.prepare_task_routing(&msg)
    };
    if !decision.is_allowed {
        return RouteOutcome {
            result: RouteResult::Buffered,
            emit_claude_thinking: false,
            buffer_reason: decision.buffer_reason,
        };
    }

    if msg.is_to_user() {
        // Soft guard: non-lead workers directing to user bypass the lead node
        // (which is normally the summarizer to the user). We observe but do
        // not block — "user explicitly named coder" is a legal edge case, and
        // a hard deny would false-positive on LLM outputs that follow that
        // instruction. Log-only for now; promote to deny after observation.
        if matches!(msg.source_role(), "coder" | "reviewer") {
            eprintln!(
                "[routing][WARN] {} → user (non-lead worker routing direct \
                 to user; lead normally summarizes). Consider prompt review \
                 if this fires often.",
                msg.source_role()
            );
        }
        return RouteOutcome {
            result: RouteResult::ToGui,
            emit_claude_thinking: false,
            buffer_reason: None,
        };
    }

    // Reply-target redirect: if sender has a recorded delegator,
    // redirect role-targeted replies to that specific agent.
    let redirect = msg.source_agent_id()
        .and_then(|sid| apply_reply_target(sid, msg.target_str()));
    let was_redirected = redirect.is_some();
    let msg = if let Some(new_to) = redirect {
        let new_target = if new_to == "user" {
            MessageTarget::User
        } else {
            MessageTarget::Agent { agent_id: new_to }
        };
        BridgeMessage { target: new_target, ..msg }
    } else {
        msg
    };

    // Resolution phase: resolve ALL providers for the target role.
    // Broadcast: target=<role> may resolve to multiple providers (AC2).
    // Task-first: when msg carries a task_id, use task_agents[] as sole truth.
    let resolution = {
        let s = state.read().await;
        resolve_broadcast_targets(&s, &msg)
    };

    match resolution {
        BroadcastResolution::Dropped => RouteOutcome {
            result: RouteResult::Dropped,
            emit_claude_thinking: false,
            buffer_reason: None,
        },
        BroadcastResolution::NeedBuffer => {
            let buffer_reason = {
                let daemon = state.read().await;
                daemon.route_buffer_reason(&msg)
            };
            state.write().await.buffer_message(msg);
            RouteOutcome {
                result: RouteResult::Buffered,
                emit_claude_thinking: false,
                buffer_reason,
            }
        }
        BroadcastResolution::Targets {
            deliveries,
            emit_claude_thinking,
            agent_targeted,
        } => {
            // Suppress reply-target recording for redirected replies:
            // a redirect already made this agent-targeted, but recording
            // would create a reciprocal mapping that turns subsequent
            // non-reply messages into sticky one-to-one redirects.
            let record = agent_targeted && !was_redirected;
            deliver_broadcast(state, msg, deliveries, emit_claude_thinking, record).await
        }
    }
}

// ── broadcast resolution ──────────────────────────────────────

enum DeliveryChannel {
    ClaudeSdk(tokio::sync::mpsc::Sender<String>),
    ClaudeBridge(tokio::sync::mpsc::Sender<ToAgent>),
    Codex {
        tx: tokio::sync::mpsc::Sender<(Vec<serde_json::Value>, bool)>,
        items: Vec<serde_json::Value>,
        from_user: bool,
    },
}

struct ResolvedDelivery {
    agent_id: String,
    channel: DeliveryChannel,
}

enum BroadcastResolution {
    Dropped,
    NeedBuffer,
    Targets {
        deliveries: Vec<ResolvedDelivery>,
        emit_claude_thinking: bool,
        agent_targeted: bool,
    },
}

fn resolve_broadcast_targets(
    s: &crate::daemon::state::DaemonState,
    msg: &BridgeMessage,
) -> BroadcastResolution {
    use crate::daemon::state::MatchedTaskAgent;
    // Task-scoped claude role check: if this message carries a task_id, only
    // emit Claude thinking when that specific task actually has a Claude agent
    // whose role matches the target. The global `claude_role` singleton races
    // across tasks (Task B's claude@lead leaks into Task A's codex@lead flow).
    let claude_role_for_thinking = msg
        .task_id
        .as_deref()
        .map(|tid| {
            s.task_graph
                .agents_for_task(tid)
                .iter()
                .find(|a| matches!(a.provider, crate::daemon::task_graph::types::Provider::Claude))
                .map(|a| a.role.clone())
                .unwrap_or_default()
        })
        .unwrap_or_else(|| s.claude_role.clone());
    let emit_claude_thinking =
        routing_display::should_emit_claude_thinking_pre(msg, &claude_role_for_thinking);

    // Resolution priority: agent_id → role → user (user handled before this call).
    // Task-first: when msg carries a task_id, use task_agents[] as sole truth.
    let (matched_agents, task_agents_authoritative, agent_targeted) = match msg.task_id.as_deref() {
        Some(tid) => {
            let task_agents = s.task_graph.agents_for_task(tid);
            let has_agents = !task_agents.is_empty();
            // Agent-targeted: check if msg.target_str() matches a concrete agent_id
            if let Some(agent) = task_agents.iter().find(|a| a.agent_id == msg.target_str()) {
                let runtime = match agent.provider {
                    crate::daemon::task_graph::types::Provider::Claude => "claude",
                    crate::daemon::task_graph::types::Provider::Codex => "codex",
                };
                (vec![MatchedTaskAgent {
                    agent_id: agent.agent_id.clone(),
                    runtime,
                }], true, true)
            } else {
                // Role-targeted: broadcast to all matching agents for this role
                let agents = s.resolve_task_role_providers(tid, msg.target_str());
                if agents.is_empty() {
                    if has_agents {
                        return BroadcastResolution::Dropped;
                    }
                    return BroadcastResolution::NeedBuffer;
                }
                (agents, has_agents, false)
            }
        }
        None => {
            let mut agents = Vec::new();
            // Agent-targeted: check if target matches a known concrete agent_id
            let agent_targeted = match msg.target_str() {
                "claude" => {
                    agents.push(MatchedTaskAgent {
                        agent_id: "claude".into(), runtime: "claude",
                    });
                    true
                }
                "codex" => {
                    agents.push(MatchedTaskAgent {
                        agent_id: "codex".into(), runtime: "codex",
                    });
                    true
                }
                _ => false,
            };
            // Role-targeted (only when not agent-targeted)
            if !agent_targeted {
                if s.claude_role == msg.target_str() {
                    agents.push(MatchedTaskAgent {
                        agent_id: "claude".into(), runtime: "claude",
                    });
                }
                if s.codex_role == msg.target_str() {
                    agents.push(MatchedTaskAgent {
                        agent_id: "codex".into(), runtime: "codex",
                    });
                }
            }
            if agents.is_empty() {
                if crate::daemon::is_valid_agent_role(msg.target_str()) {
                    return BroadcastResolution::NeedBuffer;
                }
                return BroadcastResolution::Dropped;
            }
            (agents, false, agent_targeted)
        }
    };

    let task_id = msg.task_id.as_deref();
    let ta_resolved = task_agents_authoritative;
    // Sender gate: a Claude recipient only accepts worker-role messages from
    // user/system or from a Codex agent that is actually registered in the
    // same task. The legacy fallback (singleton `codex_role`) is only used
    // when the message carries no task_id — otherwise the singleton races
    // with multi-task launches and legitimate coder→lead messages get dropped.
    let claude_sender_ok = msg.is_from_user()
        || msg.is_from_system()
        || match task_id {
            Some(tid) => s
                .task_graph
                .agents_for_task(tid)
                .iter()
                .any(|a| {
                    matches!(a.provider, crate::daemon::task_graph::types::Provider::Codex)
                        && a.role == msg.source_role()
                }),
            None => msg.source_role() == s.codex_role,
        };

    // Iterate per-agent and collect deliveries keyed by agent_id (AC1/AC2).
    // Each agent gets its own channel lookup; no provider-level dedup.
    let mut deliveries: Vec<ResolvedDelivery> = Vec::new();
    let mut any_sender_gated = false;

    for agent in &matched_agents {
        if deliveries.iter().any(|d| d.agent_id == agent.agent_id) {
            continue;
        }
        match agent.runtime {
            "claude" => {
                if !claude_sender_ok {
                    any_sender_gated = true;
                    continue;
                }
                if !should_deliver_to_agent(s, "claude", msg, ta_resolved) {
                    continue;
                }
                // Per-agent mode: only per-agent channel; no provider fallback
                // that could misroute to another same-provider agent's slot.
                let sdk_tx = if ta_resolved {
                    task_id.and_then(|tid| s.claude_task_ws_tx_for_agent(tid, &agent.agent_id))
                } else {
                    task_id
                        .and_then(|tid| s.claude_task_ws_tx_for_agent(tid, &agent.agent_id))
                        .or_else(|| task_id.and_then(|tid| s.claude_task_ws_tx(tid)))
                        .or_else(|| s.claude_sdk_ws_tx.clone())
                };
                if let Some(tx) = sdk_tx {
                    deliveries.push(ResolvedDelivery {
                        agent_id: agent.agent_id.clone(),
                        channel: DeliveryChannel::ClaudeSdk(tx),
                    });
                } else if !ta_resolved {
                    if let Some(bridge) =
                        s.attached_agents.get("claude").map(|a| a.tx.clone())
                    {
                        deliveries.push(ResolvedDelivery {
                            agent_id: agent.agent_id.clone(),
                            channel: DeliveryChannel::ClaudeBridge(bridge),
                        });
                    }
                }
            }
            "codex" => {
                if !should_deliver_to_agent(s, "codex", msg, ta_resolved) {
                    continue;
                }
                let tx = if ta_resolved {
                    task_id.and_then(|tid| s.codex_task_inject_tx_for_agent(tid, &agent.agent_id))
                } else {
                    task_id
                        .and_then(|tid| s.codex_task_inject_tx_for_agent(tid, &agent.agent_id))
                        .or_else(|| task_id.and_then(|tid| s.codex_task_inject_tx(tid)))
                        .or_else(|| s.codex_inject_tx.clone())
                };
                if let Some(tx) = tx {
                    deliveries.push(ResolvedDelivery {
                        agent_id: agent.agent_id.clone(),
                        channel: DeliveryChannel::Codex {
                            tx,
                            items: build_codex_input_items(msg),
                            from_user: msg.is_from_user(),
                        },
                    });
                }
            }
            _ => {}
        }
    }

    if deliveries.is_empty() {
        if any_sender_gated
            && !matched_agents.iter().any(|a| a.runtime == "codex")
        {
            return BroadcastResolution::Dropped;
        }
        return BroadcastResolution::NeedBuffer;
    }

    BroadcastResolution::Targets {
        deliveries,
        emit_claude_thinking,
        agent_targeted,
    }
}

/// When task_agents[] resolved the providers, the binding is authoritative
/// and we skip the legacy per-message session check (which assumes singleton
/// lead/coder session pointers). Otherwise we fall back to session matching.
fn should_deliver_to_agent(
    s: &crate::daemon::state::DaemonState,
    agent: &str,
    msg: &BridgeMessage,
    task_agents_resolved: bool,
) -> bool {
    if task_agents_resolved {
        return true;
    }
    s.agent_matches_task_message(agent, msg)
}

// ── broadcast delivery ────────────────────────────────────────

async fn deliver_broadcast(
    state: &SharedState,
    msg: BridgeMessage,
    deliveries: Vec<ResolvedDelivery>,
    emit_claude_thinking: bool,
    agent_targeted: bool,
) -> RouteOutcome {
    let mut any_delivered = false;
    let mut needs_claude_turn = false;
    let mut delivered_agent_ids: Vec<String> = Vec::new();

    for delivery in deliveries {
        let aid = delivery.agent_id;
        match delivery.channel {
            DeliveryChannel::ClaudeSdk(tx) => {
                let ndjson = format_ndjson_user_message(&msg).await;
                if tx.send(ndjson).await.is_ok() {
                    needs_claude_turn = true;
                    any_delivered = true;
                    delivered_agent_ids.push(aid);
                }
            }
            DeliveryChannel::ClaudeBridge(tx) => {
                if tx
                    .send(ToAgent::RoutedMessage {
                        message: msg.clone(),
                    })
                    .await
                    .is_ok()
                {
                    needs_claude_turn = true;
                    any_delivered = true;
                    delivered_agent_ids.push(aid);
                }
            }
            DeliveryChannel::Codex {
                tx,
                items,
                from_user,
            } => {
                if tx.send((items, from_user)).await.is_ok() {
                    any_delivered = true;
                    delivered_agent_ids.push(aid);
                }
            }
        }
    }

    // Record reply-target mappings for agent-targeted delegations
    if agent_targeted {
        if let Some(sender_id) = msg.source_agent_id() {
            for rid in &delivered_agent_ids {
                record_reply_target(rid, sender_id, msg.source_role());
            }
        }
    }

    if needs_claude_turn {
        state.write().await.prepare_claude_response_turn();
    }

    if any_delivered {
        RouteOutcome {
            result: RouteResult::Delivered,
            emit_claude_thinking,
            buffer_reason: None,
        }
    } else {
        state.write().await.buffer_message(msg);
        RouteOutcome {
            result: RouteResult::Buffered,
            emit_claude_thinking: false,
            buffer_reason: Some("target_agent_offline"),
        }
    }
}

#[cfg(test)]
pub async fn route_message_inner(state: &SharedState, msg: BridgeMessage) -> RouteResult {
    route_message_inner_with_meta(state, msg).await.result
}

pub use super::routing_format::{build_codex_input_items, format_codex_input, format_ndjson_user_message};

#[cfg(test)] #[path = "routing_behavior_tests.rs"] mod behavior_tests;
#[cfg(test)] #[path = "routing_shared_role_tests.rs"] mod shared_role_tests;
#[cfg(test)] #[path = "routing_tests.rs"] mod tests;
#[cfg(test)] #[path = "routing_user_target_tests.rs"] mod user_target_tests;
