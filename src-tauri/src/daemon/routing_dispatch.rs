use super::*;

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
