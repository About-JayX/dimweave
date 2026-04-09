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
        outcome.buffer_reason,
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
        // Telegram outbound hook — queue report_telegram messages
        if crate::telegram::report::should_send_telegram_report(&msg) {
            let s = state.read().await;
            if s.telegram_notifications_enabled {
                if let (Some(ref tx), Some(chat_id)) =
                    (&s.telegram_outbound_tx, s.telegram_paired_chat_id)
                {
                    let task_title = s
                        .active_task_id
                        .as_ref()
                        .and_then(|tid| s.task_graph.get_task(tid))
                        .map(|t| t.title.clone());
                    let tx = tx.clone();
                    drop(s);
                    let report =
                        crate::telegram::report::build_telegram_report(task_title.as_deref(), &msg);
                    for chunk in crate::telegram::report::chunk_report(&report) {
                        let _ = tx
                            .send(crate::telegram::types::TelegramOutbound {
                                chat_id,
                                text: chunk,
                                parse_mode: Some("HTML".into()),
                            })
                            .await;
                    }
                }
            }
        }
    }
}
