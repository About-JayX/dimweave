#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets;
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

    enum Target {
        Claude(tokio::sync::mpsc::Sender<ToAgent>),
        ClaudeSdk(tokio::sync::mpsc::Sender<String>),
        Codex(tokio::sync::mpsc::Sender<(Vec<serde_json::Value>, bool)>, Vec<serde_json::Value>, bool),
        NeedBuffer,
    }

    // Resolve the target agent for the message.
    //
    // Task-first routing (AC1/AC2): when a message carries a task_id, resolve
    // the target provider through the task's persisted lead_provider/coder_provider
    // instead of the global claude_role/codex_role singletons. This ensures that
    // messages stamped against a specific task reach the correct provider even
    // when the active task or global role assignment has changed.
    let (target, emit_claude_thinking) = {
        let s = state.read().await;
        let emit_claude_thinking =
            routing_display::should_emit_claude_thinking_pre(&msg, &s.claude_role);

        // Task-first: resolve agent from task's provider binding
        let task_resolved_agent = msg
            .task_id
            .as_deref()
            .and_then(|tid| s.resolve_task_provider_agent(tid, &msg.to));

        let claude_matches = match task_resolved_agent {
            Some("claude") => true,
            Some(_) => false,
            None => s.claude_role == msg.to,
        };
        let codex_matches = match task_resolved_agent {
            Some("codex") => true,
            Some(_) => false,
            None => s.codex_role == msg.to,
        };

        if !claude_matches && !codex_matches {
            if crate::daemon::is_valid_agent_role(&msg.to) {
                (Target::NeedBuffer, false)
            } else {
                return RouteOutcome {
                    result: RouteResult::Dropped,
                    emit_claude_thinking: false,
                    buffer_reason: None,
                };
            }
        } else {
            // Sender gating: Claude only accepts user/system/current codex_role
            if claude_matches
                && msg.from != "user"
                && msg.from != "system"
                && msg.from != s.codex_role
            {
                return RouteOutcome {
                    result: RouteResult::Dropped,
                    emit_claude_thinking: false,
                    buffer_reason: None,
                };
            }
            // Collect online candidates for the target role.
            // Prefer Claude SDK WS over bridge for Claude delivery.
            let claude_sdk_tx = if claude_matches && s.agent_matches_task_message("claude", &msg) {
                s.claude_sdk_ws_tx.clone()
            } else {
                None
            };
            let claude_tx = if claude_matches
                && claude_sdk_tx.is_none()
                && s.agent_matches_task_message("claude", &msg)
            {
                s.attached_agents.get("claude").map(|a| a.tx.clone())
            } else {
                None
            };
            let codex_tx = if codex_matches && s.agent_matches_task_message("codex", &msg) {
                s.codex_inject_tx.clone()
            } else {
                None
            };

            if let Some(tx) = claude_sdk_tx {
                (Target::ClaudeSdk(tx), emit_claude_thinking)
            } else if let Some(tx) = claude_tx {
                (Target::Claude(tx), emit_claude_thinking)
            } else if let Some(tx) = codex_tx {
                let from_user = msg.from == "user";
                (
                    Target::Codex(tx, build_codex_input_items(&msg), from_user),
                    false,
                )
            } else {
                (Target::NeedBuffer, false)
            }
        }
    };

    match target {
        Target::Claude(tx) => {
            if tx
                .send(ToAgent::RoutedMessage {
                    message: msg.clone(),
                })
                .await
                .is_ok()
            {
                state.write().await.prepare_claude_response_turn();
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
        Target::ClaudeSdk(tx) => {
            let ndjson = format_ndjson_user_message(&msg).await;
            if tx.send(ndjson).await.is_ok() {
                state.write().await.prepare_claude_response_turn();
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
        Target::Codex(tx, items, from_user) => {
            if tx.send((items, from_user)).await.is_ok() {
                RouteOutcome {
                    result: RouteResult::Delivered,
                    emit_claude_thinking: false,
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
        Target::NeedBuffer => {
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
