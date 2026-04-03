use crate::daemon::orchestrator::review_gate::ReviewGate;
use crate::daemon::orchestrator::task_flow;
use crate::daemon::task_graph::types::*;
use crate::daemon::task_graph::TaskGraphStore;
use crate::daemon::types::{BridgeMessage, MessageStatus};

fn msg(from: &str, to: &str, status: MessageStatus) -> BridgeMessage {
    BridgeMessage {
        id: format!("{from}_to_{to}"),
        from: from.into(),
        display_source: None,
        to: to.into(),
        content: "test".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,
    }
}

// ── Review gate: should_block ──────────────────────────────

#[test]
fn gate_blocks_lead_to_coder_during_any_review_status() {
    let gate = ReviewGate::new();
    for rs in [
        ReviewStatus::PendingLeadReview,
        ReviewStatus::InReview,
        ReviewStatus::PendingLeadApproval,
    ] {
        assert!(
            gate.should_block(Some(rs), &msg("lead", "coder", MessageStatus::Done)),
            "should block during {rs:?}"
        );
    }
}

#[test]
fn gate_allows_non_lead_to_coder_during_review() {
    let gate = ReviewGate::new();
    assert!(!gate.should_block(
        Some(ReviewStatus::InReview),
        &msg("reviewer", "coder", MessageStatus::Done),
    ));
}

#[test]
fn gate_allows_lead_to_coder_when_no_review() {
    let gate = ReviewGate::new();
    assert!(!gate.should_block(None, &msg("lead", "coder", MessageStatus::Done)));
}

// ── Review gate: buffer + approve ──────────────────────────

#[test]
fn gate_approve_releases_buffered_messages() {
    let mut gate = ReviewGate::new();
    let tid = "task-1";
    gate.buffer_message(tid, msg("lead", "coder", MessageStatus::Done));
    gate.buffer_message(tid, msg("lead", "coder", MessageStatus::Done));
    let released = gate.approve(tid);
    assert_eq!(released.len(), 2);
    assert!(gate.approve(tid).is_empty());
}

#[test]
fn gate_approve_with_no_buffered_returns_empty() {
    let mut gate = ReviewGate::new();
    assert!(gate.approve("no-such-task").is_empty());
}

// ── Task flow: reviewer done sets PendingLeadApproval ──────

#[test]
fn reviewer_done_sets_pending_lead_approval_not_clear() {
    let mut store = TaskGraphStore::new();
    let mut gate = ReviewGate::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Reviewing);
    store.update_task_review_status(&tid, Some(ReviewStatus::InReview));

    let m = msg("reviewer", "lead", MessageStatus::Done);
    let released = task_flow::process_message(&mut store, &mut gate, &tid, &m);

    assert!(released.is_empty(), "should NOT release messages");
    let t = store.get_task(&tid).unwrap();
    assert_eq!(t.review_status, Some(ReviewStatus::PendingLeadApproval));
    assert_eq!(t.status, TaskStatus::Reviewing);
}

// ── Full cycle: coder → review → lead approval ────────────

#[test]
fn full_review_cycle_requires_lead_approval() {
    let mut store = TaskGraphStore::new();
    let mut gate = ReviewGate::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Implementing);

    // 1. coder → lead (done) → PendingLeadReview
    let m1 = msg("coder", "lead", MessageStatus::Done);
    task_flow::process_message(&mut store, &mut gate, &tid, &m1);
    let t = store.get_task(&tid).unwrap();
    assert_eq!(t.review_status, Some(ReviewStatus::PendingLeadReview));
    assert_eq!(t.status, TaskStatus::Reviewing);

    // 2. lead → coder blocked by gate
    let blocked = msg("lead", "coder", MessageStatus::Done);
    assert!(gate.should_block(t.review_status, &blocked));
    gate.buffer_message(&tid, blocked);

    // 3. lead → reviewer → InReview
    let m3 = msg("lead", "reviewer", MessageStatus::Done);
    task_flow::process_message(&mut store, &mut gate, &tid, &m3);
    let t = store.get_task(&tid).unwrap();
    assert_eq!(t.review_status, Some(ReviewStatus::InReview));

    // 4. reviewer → lead (done) → PendingLeadApproval (NOT released)
    let m4 = msg("reviewer", "lead", MessageStatus::Done);
    let released = task_flow::process_message(&mut store, &mut gate, &tid, &m4);
    assert!(released.is_empty());
    let t = store.get_task(&tid).unwrap();
    assert_eq!(t.review_status, Some(ReviewStatus::PendingLeadApproval));

    // 5. lead approves → releases blocked, clears gate
    let released = task_flow::lead_approve(&mut store, &mut gate, &tid);
    assert_eq!(released.len(), 1);
    let t = store.get_task(&tid).unwrap();
    assert_eq!(t.review_status, None);
    assert_eq!(t.status, TaskStatus::Implementing);
}

// ── lead_approve when not in PendingLeadApproval is no-op ──

#[test]
fn lead_approve_noop_when_not_pending_approval() {
    let mut store = TaskGraphStore::new();
    let mut gate = ReviewGate::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Implementing);

    let released = task_flow::lead_approve(&mut store, &mut gate, &tid);
    assert!(released.is_empty());
    // Status unchanged
    assert_eq!(
        store.get_task(&tid).unwrap().status,
        TaskStatus::Implementing
    );
}

// ── preferred_auto_target ──────────────────────────────────

#[test]
fn auto_target_returns_lead_during_review() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    store.update_task_status(&task.task_id, TaskStatus::Reviewing);
    store.update_task_review_status(&task.task_id, Some(ReviewStatus::PendingLeadApproval));
    let target = task_flow::preferred_auto_target(store.get_task(&task.task_id).unwrap());
    assert_eq!(target.as_deref(), Some("lead"));
}

#[test]
fn auto_target_returns_coder_during_implementing() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    store.update_task_status(&task.task_id, TaskStatus::Implementing);
    let target = task_flow::preferred_auto_target(store.get_task(&task.task_id).unwrap());
    assert_eq!(target.as_deref(), Some("coder"));
}
