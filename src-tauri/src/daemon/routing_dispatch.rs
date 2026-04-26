use super::*;

pub async fn route_message(
    state: &SharedState,
    app: &AppHandle,
    msg: BridgeMessage,
) -> RouteResult {
    route_message_with_display(state, app, msg, true).await
}

pub async fn route_message_silent(
    state: &SharedState,
    app: &AppHandle,
    msg: BridgeMessage,
) -> RouteResult {
    route_message_with_display(state, app, msg, false).await
}

async fn route_message_with_display(
    state: &SharedState,
    app: &AppHandle,
    msg: BridgeMessage,
    display_in_gui: bool,
) -> RouteResult {
    let outcome = route_message_inner_with_meta(state, msg.clone()).await;
    // Persist only messages that reach the GUI timeline, matching the emit
    // condition below. `route_message_silent` is used for per-target
    // duplicates of a single user input (display_msg was already persisted
    // in routing_user_input); gating on display_in_gui prevents N+1
    // duplicates in task_messages for a single user turn.
    if display_in_gui
        && matches!(outcome.result, RouteResult::Delivered | RouteResult::ToGui)
        && routing_display::is_renderable_message(&msg)
    {
        state.read().await.task_graph.persist_task_message(&msg);
    }
    routing_display::emit_route_side_effects(
        app,
        &msg,
        &outcome.result,
        outcome.buffer_reason,
        outcome.emit_claude_thinking,
        display_in_gui,
    );
    if matches!(outcome.result, RouteResult::Delivered | RouteResult::ToGui) {
        let (effects, became_implementing, transition_task_id) = {
            let mut s = state.write().await;
            let before_status = s
                .active_task_id
                .as_ref()
                .and_then(|tid| s.task_graph.get_task(tid))
                .map(|t| t.status);
            let eff = s.observe_task_message_effects(&msg);
            let after_status = s
                .active_task_id
                .as_ref()
                .and_then(|tid| s.task_graph.get_task(tid))
                .map(|t| t.status);
            let transitioned = !matches!(
                before_status,
                Some(crate::daemon::task_graph::types::TaskStatus::Implementing)
            ) && matches!(
                after_status,
                Some(crate::daemon::task_graph::types::TaskStatus::Implementing)
            );
            // Prefer msg.task_id (explicit message context) over global active_task_id
            let tid = msg.task_id.clone().or_else(|| s.active_task_id.clone());
            (eff, transitioned, tid)
        };
        for event in effects.ui_events {
            event.emit(app);
        }
        for released_msg in effects.released {
            Box::pin(route_message_with_display(state, app, released_msg, false)).await;
        }
        // Feishu bug transition hook — best-effort transition to 处理中
        if became_implementing {
            if let Some(task_id) = transition_task_id {
                let state2 = state.clone();
                let app2 = app.clone();
                tokio::spawn(async move {
                    crate::daemon::feishu_project_task_link::try_transition_to_processing(
                        &state2, &app2, &task_id,
                    )
                    .await;
                });
            }
        }
        // Telegram outbound hook — queue lead messages for Telegram
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
    outcome.result
}
