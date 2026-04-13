use super::*;

#[test]
fn codex_port_pool_allocates_first_available() {
    let mut pool = CodexPortPool::new(4500);
    assert_eq!(pool.reserve("task_a", 1), Some(4500));
}

#[test]
fn codex_port_pool_skips_reserved_ports() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    assert_eq!(pool.reserve("task_b", 2), Some(4501));
}

#[test]
fn codex_port_pool_returns_none_when_exhausted() {
    let mut pool = CodexPortPool::new(4500);
    for i in 0..8 {
        pool.reserve(&format!("task_{i}"), i as u64);
    }
    assert_eq!(pool.reserve("task_overflow", 99), None);
}

#[test]
fn codex_port_pool_contains_checks_range() {
    let pool = CodexPortPool::new(4500);
    assert!(pool.contains(4500));
    assert!(pool.contains(4507));
    assert!(!pool.contains(4508));
    assert!(!pool.contains(4499));
}

#[test]
fn codex_port_pool_released_port_becomes_available() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    assert_eq!(pool.reserve("task_b", 2), Some(4501));
    pool.release(4500, "task_a", 1);
    assert_eq!(pool.reserve("task_c", 3), Some(4500));
}

#[test]
fn codex_port_pool_promote_guards_owner() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    assert!(!pool.promote(4500, "task_b", 1), "wrong task rejected");
    assert!(!pool.promote(4500, "task_a", 99), "wrong launch_id rejected");
    assert!(pool.promote(4500, "task_a", 1), "correct owner accepted");
}

#[test]
fn codex_port_pool_release_guards_owner() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    assert!(!pool.release(4500, "task_b", 1), "wrong task rejected");
    assert!(!pool.release(4500, "task_a", 99), "wrong launch_id rejected");
    assert!(pool.release(4500, "task_a", 1), "correct owner accepted");
}

#[test]
fn codex_port_pool_reserved_port_not_reallocated() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    assert_eq!(pool.reserve("task_b", 2), Some(4501));
    assert_ne!(pool.reserve("task_c", 3), Some(4500));
}

#[test]
fn codex_port_pool_stale_release_ignored() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    pool.promote(4500, "task_a", 1);
    pool.release(4500, "task_a", 1);
    pool.reserve("task_b", 2);
    // stale task_a callback — must fail
    assert!(!pool.release(4500, "task_a", 1));
    assert!(pool.leases.contains_key(&4500));
}

#[test]
fn codex_port_pool_release_all_for_task() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    pool.reserve("task_a", 2); // 4501
    pool.reserve("task_b", 3); // 4502
    pool.release_all_for_task("task_a");
    assert_eq!(pool.leased_ports().len(), 1);
    assert!(pool.leased_ports().contains(&4502));
}

/// AC: reserved lease becomes live on successful handshake
#[test]
fn codex_port_pool_reserve_promote_release_lifecycle() {
    let mut pool = CodexPortPool::new(4500);
    let port = pool.reserve("task_a", 1).unwrap();
    assert_eq!(port, 4500);
    assert!(pool.promote(port, "task_a", 1));
    assert!(pool.release(port, "task_a", 1));
    assert!(!pool.leased_ports().contains(&4500));
    assert_eq!(pool.reserve("task_b", 2), Some(4500));
}

/// AC: natural process exit releases the leased port via exit notice
#[test]
fn codex_port_pool_exit_notice_releases_live_port() {
    let mut pool = CodexPortPool::new(4500);
    let port_a = pool.reserve("task_a", 1).unwrap();
    let port_b = pool.reserve("task_b", 2).unwrap();
    pool.promote(port_a, "task_a", 1);
    pool.promote(port_b, "task_b", 2);
    assert_eq!(pool.leased_ports().len(), 2);
    pool.release(port_a, "task_a", 1);
    assert_eq!(pool.leased_ports().len(), 1);
    assert!(pool.leased_ports().contains(&port_b));
}

/// AC: stale exit notice for a port re-assigned to another task is ignored
#[test]
fn codex_port_pool_stale_exit_notice_after_reassignment() {
    let mut pool = CodexPortPool::new(4500);
    let port = pool.reserve("task_a", 1).unwrap();
    pool.promote(port, "task_a", 1);
    pool.release(port, "task_a", 1);
    let port2 = pool.reserve("task_b", 2).unwrap();
    assert_eq!(port, port2);
    pool.promote(port2, "task_b", 2);
    assert!(!pool.release(port, "task_a", 1));
    assert!(pool.leased_ports().contains(&port));
}

/// AC: same-task restart — stale exit notice with old launch_id is rejected
#[test]
fn codex_port_pool_same_task_restart_stale_launch_id_rejected() {
    let mut pool = CodexPortPool::new(4500);
    // Launch 1 for task_a
    let port = pool.reserve("task_a", 1).unwrap();
    pool.promote(port, "task_a", 1);
    // Explicit release during restart (stop_codex_for_task)
    pool.release(port, "task_a", 1);
    // Launch 2 for the same task_a, gets same port
    let port2 = pool.reserve("task_a", 2).unwrap();
    assert_eq!(port, port2);
    pool.promote(port2, "task_a", 2);
    // Stale exit notice from launch 1 — same task_id, same port, OLD launch_id
    assert!(!pool.release(port, "task_a", 1), "old launch_id must be rejected");
    assert!(pool.leased_ports().contains(&port), "new lease must survive");
    // Correct release with current launch_id succeeds
    assert!(pool.release(port, "task_a", 2));
}

/// AC: promote only matches the exact launch_id
#[test]
fn codex_port_pool_promote_rejects_stale_launch_id() {
    let mut pool = CodexPortPool::new(4500);
    pool.reserve("task_a", 1);
    // Release and re-reserve with new launch_id
    pool.release(4500, "task_a", 1);
    pool.reserve("task_a", 2);
    // Stale promote from launch 1 — must fail
    assert!(!pool.promote(4500, "task_a", 1));
    // Correct promote with launch 2
    assert!(pool.promote(4500, "task_a", 2));
}
