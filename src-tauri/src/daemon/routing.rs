#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets;
pub use crate::daemon::routing_user_input::route_user_input;
use crate::daemon::{
    routing_display,
    types::{BridgeMessage, ToAgent},
    SharedState,
};
use tauri::AppHandle;

pub enum RouteResult {
    Delivered,
    Buffered,
    Dropped,
    ToGui,
}

struct RouteOutcome {
    result: RouteResult,
    emit_claude_thinking: bool,
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
        };
    }

    if msg.to == "user" {
        return RouteOutcome {
            result: RouteResult::ToGui,
            emit_claude_thinking: false,
        };
    }

    enum Target {
        Claude(tokio::sync::mpsc::Sender<ToAgent>),
        ClaudeSdk(tokio::sync::mpsc::Sender<String>),
        Codex(tokio::sync::mpsc::Sender<(Vec<serde_json::Value>, bool)>, Vec<serde_json::Value>, bool),
        NeedBuffer,
    }

    // Collect online candidates for the target role, then pick the first.
    // This avoids the sequential if-else bug where an offline Claude with a
    // cached role would shadow an online Codex holding the same role.
    let (target, emit_claude_thinking) = {
        let s = state.read().await;
        let emit_claude_thinking =
            routing_display::should_emit_claude_thinking_pre(&msg, &s.claude_role);
        let claude_matches = s.claude_role == msg.to;
        let codex_matches = s.codex_role == msg.to;

        if !claude_matches && !codex_matches {
            if crate::daemon::is_valid_agent_role(&msg.to) {
                (Target::NeedBuffer, false)
            } else {
                return RouteOutcome {
                    result: RouteResult::Dropped,
                    emit_claude_thinking: false,
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
                };
            }
            // Collect online candidates for the target role.
            // Prefer Claude SDK WS over bridge for Claude delivery.
            let claude_sdk_tx = if claude_matches {
                s.claude_sdk_ws_tx.clone()
            } else {
                None
            };
            let claude_tx = if claude_matches && claude_sdk_tx.is_none() {
                s.attached_agents.get("claude").map(|a| a.tx.clone())
            } else {
                None
            };
            let codex_tx = if codex_matches {
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
                }
            } else {
                state.write().await.buffer_message(msg);
                RouteOutcome {
                    result: RouteResult::Buffered,
                    emit_claude_thinking: false,
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
                }
            } else {
                state.write().await.buffer_message(msg);
                RouteOutcome {
                    result: RouteResult::Buffered,
                    emit_claude_thinking: false,
                }
            }
        }
        Target::Codex(tx, items, from_user) => {
            if tx.send((items, from_user)).await.is_ok() {
                RouteOutcome {
                    result: RouteResult::Delivered,
                    emit_claude_thinking: false,
                }
            } else {
                state.write().await.buffer_message(msg);
                RouteOutcome {
                    result: RouteResult::Buffered,
                    emit_claude_thinking: false,
                }
            }
        }
        Target::NeedBuffer => {
            state.write().await.buffer_message(msg);
            RouteOutcome {
                result: RouteResult::Buffered,
                emit_claude_thinking: false,
            }
        }
    }
}

#[cfg(test)]
pub async fn route_message_inner(state: &SharedState, msg: BridgeMessage) -> RouteResult {
    route_message_inner_with_meta(state, msg).await.result
}

pub async fn route_message(state: &SharedState, app: &AppHandle, msg: BridgeMessage) {
    route_message_with_display(state, app, msg, true).await;
}

pub async fn route_message_silent(state: &SharedState, app: &AppHandle, msg: BridgeMessage) {
    route_message_with_display(state, app, msg, false).await;
}

async fn route_message_with_display(
    state: &SharedState,
    app: &AppHandle,
    msg: BridgeMessage,
    display_in_gui: bool,
) {
    let outcome = route_message_inner_with_meta(state, msg.clone()).await;
    routing_display::emit_route_side_effects(
        app,
        &msg,
        &outcome.result,
        outcome.emit_claude_thinking,
        display_in_gui,
    );
    if matches!(outcome.result, RouteResult::Delivered | RouteResult::ToGui) {
        let effects = {
            let mut s = state.write().await;
            s.observe_task_message_effects(&msg)
        };
        for event in effects.ui_events {
            event.emit(app);
        }
        for released_msg in effects.released {
            Box::pin(route_message_with_display(state, app, released_msg, false)).await;
        }
    }
}

pub use super::routing_format::{build_codex_input_items, format_codex_input, format_ndjson_user_message};

#[cfg(test)]
#[path = "routing_behavior_tests.rs"]
mod behavior_tests;
#[cfg(test)]
#[path = "routing_shared_role_tests.rs"]
mod shared_role_tests;
#[cfg(test)]
#[path = "routing_tests.rs"]
mod tests;
