use super::{
    enqueue_events, nonce::current_launch_nonce, nonce::LaunchNonceError, nonce::LaunchNonceQuery,
    processing::summarize_event_shape, processing::summarize_events_batch,
    reconnect_delay_ms, EventEnqueueError,
};
use crate::daemon::claude_sdk::protocol::PostEventsBody as EventsBody;
use crate::daemon::state::DaemonState;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[test]
fn summarize_assistant_event_reports_shape_and_lengths() {
    let event = json!({
        "type": "assistant",
        "session_id": "sess-1",
        "message": {
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "tool_use", "name": "edit"}
            ]
        }
    });

    let summary = summarize_event_shape(&event);

    assert!(summary.contains("assistant"));
    assert!(summary.contains("shape={type,session_id,message{content[]}}"));
    assert!(summary.contains("content_items=2"));
    assert!(summary.contains("text_len=5"));
}

#[test]
fn summarize_events_batch_reports_count_and_event_kinds() {
    let body = EventsBody {
        events: vec![
            json!({"type": "system", "session_id": "sess-1"}),
            json!({"type": "result", "session_id": "sess-1", "result": "done"}),
        ],
    };

    let summary = summarize_events_batch(&body);

    assert!(summary.contains("count=2"));
    assert!(summary.contains("system"));
    assert!(summary.contains("result"));
    assert!(summary.contains("shape={type,session_id,result}"));
}

#[tokio::test]
async fn enqueue_events_errors_when_queue_missing() {
    let state = Arc::new(RwLock::new(DaemonState::new()));

    let result = enqueue_events(&state, vec![json!({"type": "system"})]).await;

    assert_eq!(result, Err(EventEnqueueError::QueueUnavailable));
}

#[tokio::test]
async fn enqueue_events_errors_when_queue_closed() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);
    state.write().await.claude_sdk_event_tx = Some(tx);

    let result = enqueue_events(&state, vec![json!({"type": "system"})]).await;

    assert_eq!(result, Err(EventEnqueueError::QueueClosed));
}

#[tokio::test]
async fn enqueue_events_preserves_fifo_batch_order() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    state.write().await.claude_sdk_event_tx = Some(tx);
    let first = vec![json!({"type": "system", "session_id": "1"})];
    let second = vec![json!({"type": "result", "session_id": "2"})];

    enqueue_events(&state, first.clone()).await.unwrap();
    enqueue_events(&state, second.clone()).await.unwrap();

    assert_eq!(rx.recv().await, Some(first));
    assert_eq!(rx.recv().await, Some(second));
}

#[tokio::test]
async fn current_launch_nonce_requires_query_param() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let query = LaunchNonceQuery::default();

    let result = current_launch_nonce(&state, &query).await;

    assert_eq!(result, Err(LaunchNonceError::Missing));
}

#[tokio::test]
async fn current_launch_nonce_accepts_pending_and_active_launches() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    {
        let mut s = state.write().await;
        let epoch = s.begin_claude_sdk_launch("nonce-a".into());
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", tx).is_some());
    }

    let pending_state = Arc::new(RwLock::new(DaemonState::new()));
    pending_state
        .write()
        .await
        .begin_claude_sdk_launch("nonce-pending".into());

    let active = current_launch_nonce(
        &state,
        &LaunchNonceQuery {
            launch_nonce: Some("nonce-a".into()),
        },
    )
    .await;
    let pending = current_launch_nonce(
        &pending_state,
        &LaunchNonceQuery {
            launch_nonce: Some("nonce-pending".into()),
        },
    )
    .await;

    assert_eq!(active.unwrap(), "nonce-a");
    assert_eq!(pending.unwrap(), "nonce-pending");
}

#[tokio::test]
async fn current_launch_nonce_rejects_stale_nonce() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    state
        .write()
        .await
        .begin_claude_sdk_launch("nonce-a".into());

    let result = current_launch_nonce(
        &state,
        &LaunchNonceQuery {
            launch_nonce: Some("nonce-b".into()),
        },
    )
    .await;

    assert_eq!(result, Err(LaunchNonceError::Stale));
}

#[test]
fn reconnect_delay_uses_bounded_exponential_backoff() {
    assert_eq!(reconnect_delay_ms(1), Some(500));
    assert_eq!(reconnect_delay_ms(2), Some(1_000));
    assert_eq!(reconnect_delay_ms(3), Some(2_000));
    assert_eq!(reconnect_delay_ms(5), Some(8_000));
    assert_eq!(reconnect_delay_ms(6), None);
}
