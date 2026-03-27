use crate::daemon::{
    routing_display,
    types::{BridgeMessage, ToAgent},
    SharedState,
};
pub use crate::daemon::routing_user_input::route_user_input;
#[cfg(test)]
pub use crate::daemon::routing_user_input::resolve_user_targets;
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
    if msg.to == "user" {
        return RouteOutcome {
            result: RouteResult::ToGui,
            emit_claude_thinking: false,
        };
    }

    enum Target {
        Claude(tokio::sync::mpsc::Sender<ToAgent>),
        Codex(tokio::sync::mpsc::Sender<(String, bool)>, String, bool),
        NeedBuffer,
    }

    // First try with read lock — avoids write contention on the hot path
    let (target, emit_claude_thinking) = {
        let s = state.read().await;
        let emit_claude_thinking = routing_display::should_emit_claude_thinking_pre(
            &msg, &s.claude_role,
        );
        if s.claude_role == msg.to {
            if msg.from != "user" && msg.from != "system" && msg.from != s.codex_role {
                return RouteOutcome {
                    result: RouteResult::Dropped,
                    emit_claude_thinking: false,
                };
            }
            if let Some(agent) = s.attached_agents.get("claude") {
                (Target::Claude(agent.tx.clone()), emit_claude_thinking)
            } else {
                (Target::NeedBuffer, false)
            }
        } else if s.codex_role == msg.to {
            if let Some(tx) = s.codex_inject_tx.clone() {
                let from_user = msg.from == "user";
                (
                    Target::Codex(tx, format_codex_input(&msg), from_user),
                    false,
                )
            } else {
                (Target::NeedBuffer, false)
            }
        } else if crate::daemon::is_valid_agent_role(&msg.to) {
            (Target::NeedBuffer, false)
        } else {
            return RouteOutcome {
                result: RouteResult::Dropped,
                emit_claude_thinking: false,
            };
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
        Target::Codex(tx, input, from_user) => {
            if tx.send((input, from_user)).await.is_ok() {
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
}

pub fn format_codex_input(msg: &BridgeMessage) -> String {
    if msg.from == "user" {
        msg.content.clone()
    } else {
        match msg.status {
            Some(status) => format!(
                "Message from {} (status: {}):\n{}",
                msg.from,
                status.as_str(),
                msg.content
            ),
            None => format!("Message from {}:\n{}", msg.from, msg.content),
        }
    }
}

#[cfg(test)] #[path = "routing_tests.rs"] mod tests;
#[cfg(test)] #[path = "routing_behavior_tests.rs"] mod behavior_tests;
