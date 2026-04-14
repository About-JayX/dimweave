#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets;
#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets_for_task;
pub use crate::daemon::routing_user_input::route_user_input;
use crate::daemon::{
    routing_display,
    types::{BridgeMessage, ToAgent},
    SharedState,
};
use tauri::AppHandle;

#[path = "routing_dispatch.rs"]
mod dispatch;
pub use dispatch::{route_message, route_message_silent};

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

    if msg.to == "user" {
        return RouteOutcome {
            result: RouteResult::ToGui,
            emit_claude_thinking: false,
            buffer_reason: None,
        };
    }

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
            claude_sdk,
            claude_bridge,
            codex,
            emit_claude_thinking,
        } => {
            deliver_broadcast(state, msg, claude_sdk, claude_bridge, codex, emit_claude_thinking)
                .await
        }
    }
}

// ── broadcast resolution ──────────────────────────────────────

enum BroadcastResolution {
    Dropped,
    NeedBuffer,
    Targets {
        claude_sdk: Option<tokio::sync::mpsc::Sender<String>>,
        claude_bridge: Option<tokio::sync::mpsc::Sender<ToAgent>>,
        codex: Option<(
            tokio::sync::mpsc::Sender<(Vec<serde_json::Value>, bool)>,
            Vec<serde_json::Value>,
            bool,
        )>,
        emit_claude_thinking: bool,
    },
}

fn resolve_broadcast_targets(
    s: &crate::daemon::state::DaemonState,
    msg: &BridgeMessage,
) -> BroadcastResolution {
    let emit_claude_thinking =
        routing_display::should_emit_claude_thinking_pre(msg, &s.claude_role);

    // Task-first: resolve providers from task_agents[].
    // Track whether task_agents (not singleton fallback) provided the resolution,
    // so session matching is only skipped for the authoritative new model.
    let (task_providers, task_agents_authoritative) = match msg.task_id.as_deref() {
        Some(tid) => {
            let has_agents = !s.task_graph.agents_for_task(tid).is_empty();
            let providers = s.resolve_task_role_providers(tid, &msg.to);
            (Some(providers), has_agents)
        }
        None => (None, false),
    };

    let (claude_matches, codex_matches) = match &task_providers {
        Some(p) if !p.is_empty() => (p.contains(&"claude"), p.contains(&"codex")),
        Some(_) if task_agents_authoritative => {
            // task_agents exist but none match the role → buffer (AC4)
            return BroadcastResolution::NeedBuffer;
        }
        Some(_) => {
            // Legacy fallback returned empty → no agent for this role
            return BroadcastResolution::NeedBuffer;
        }
        None => (s.claude_role == msg.to, s.codex_role == msg.to),
    };

    if !claude_matches && !codex_matches {
        if crate::daemon::is_valid_agent_role(&msg.to) {
            return BroadcastResolution::NeedBuffer;
        }
        return BroadcastResolution::Dropped;
    }

    // Sender gating: Claude only accepts user/system/current codex_role
    let claude_sender_gated = claude_matches
        && msg.from != "user"
        && msg.from != "system"
        && msg.from != s.codex_role;

    if claude_sender_gated && !codex_matches {
        return BroadcastResolution::Dropped;
    }

    let task_id = msg.task_id.as_deref();
    // Only skip legacy session check when task_agents[] genuinely resolved the providers
    let ta_resolved = task_agents_authoritative;

    // Collect Claude channel (SDK preferred over bridge)
    let (claude_sdk, claude_bridge) = if claude_matches
        && !claude_sender_gated
        && should_deliver_to_agent(s, "claude", msg, ta_resolved)
    {
        let sdk_tx = task_id
            .and_then(|tid| s.claude_task_ws_tx(tid))
            .or_else(|| s.claude_sdk_ws_tx.clone());
        if sdk_tx.is_some() {
            (sdk_tx, None)
        } else {
            (None, s.attached_agents.get("claude").map(|a| a.tx.clone()))
        }
    } else {
        (None, None)
    };

    // Collect Codex channel
    let codex = if codex_matches && should_deliver_to_agent(s, "codex", msg, ta_resolved) {
        let tx = task_id
            .and_then(|tid| s.codex_task_inject_tx(tid))
            .or_else(|| s.codex_inject_tx.clone());
        tx.map(|t| (t, build_codex_input_items(msg), msg.from == "user"))
    } else {
        None
    };

    if claude_sdk.is_none() && claude_bridge.is_none() && codex.is_none() {
        return BroadcastResolution::NeedBuffer;
    }

    BroadcastResolution::Targets {
        claude_sdk,
        claude_bridge,
        codex,
        emit_claude_thinking,
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
    claude_sdk: Option<tokio::sync::mpsc::Sender<String>>,
    claude_bridge: Option<tokio::sync::mpsc::Sender<ToAgent>>,
    codex: Option<(
        tokio::sync::mpsc::Sender<(Vec<serde_json::Value>, bool)>,
        Vec<serde_json::Value>,
        bool,
    )>,
    emit_claude_thinking: bool,
) -> RouteOutcome {
    let mut any_delivered = false;

    // Deliver to Claude (SDK preferred, then bridge)
    if let Some(tx) = claude_sdk {
        let ndjson = format_ndjson_user_message(&msg).await;
        if tx.send(ndjson).await.is_ok() {
            state.write().await.prepare_claude_response_turn();
            any_delivered = true;
        }
    } else if let Some(tx) = claude_bridge {
        if tx
            .send(ToAgent::RoutedMessage {
                message: msg.clone(),
            })
            .await
            .is_ok()
        {
            state.write().await.prepare_claude_response_turn();
            any_delivered = true;
        }
    }

    // Deliver to Codex (independent of Claude — broadcast)
    if let Some((tx, items, from_user)) = codex {
        if tx.send((items, from_user)).await.is_ok() {
            any_delivered = true;
        }
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
