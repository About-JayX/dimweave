use super::validate_bot_token;
use crate::telegram::{config, types::TelegramConfig};
use std::path::PathBuf;

fn temp_config_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "dimweave_tg_test_{label}_{}.json",
        std::process::id(),
    ))
}

#[test]
fn valid_token_accepted() {
    assert!(validate_bot_token("123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11").is_ok());
}

#[test]
fn valid_token_minimal() {
    assert!(validate_bot_token("1:x").is_ok());
}

#[test]
fn rejects_missing_colon() {
    assert!(validate_bot_token("no-colon-here").is_err());
}

#[test]
fn rejects_empty_hash() {
    assert!(validate_bot_token("123456:").is_err());
}

#[test]
fn rejects_empty_bot_id() {
    assert!(validate_bot_token(":some-hash").is_err());
}

#[test]
fn rejects_non_numeric_bot_id() {
    assert!(validate_bot_token("abc:some-hash").is_err());
}

// --- commit_update_cursor tests ---

#[test]
fn commit_update_cursor_persists_immediately() {
    let path = temp_config_path("cursor_persist");
    let mut cfg = TelegramConfig::default();
    crate::telegram::runtime::commit_update_cursor(&mut cfg, 42, &path);
    let loaded = config::load_config(&path).unwrap();
    assert_eq!(loaded.last_update_id, Some(42));
    assert_eq!(cfg.last_update_id, Some(42));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn commit_update_cursor_advances_per_update() {
    let path = temp_config_path("cursor_advance");
    let mut cfg = TelegramConfig { last_update_id: Some(100), ..Default::default() };
    crate::telegram::runtime::commit_update_cursor(&mut cfg, 101, &path);
    assert_eq!(config::load_config(&path).unwrap().last_update_id, Some(101));
    crate::telegram::runtime::commit_update_cursor(&mut cfg, 102, &path);
    assert_eq!(config::load_config(&path).unwrap().last_update_id, Some(102));
    let _ = std::fs::remove_file(&path);
}

/// Simulates a mid-batch crash: only the first update of [101, 102, 103] is
/// committed before the process dies. After restart, the offset should resume
/// from 102, not re-replay 101 from the prior saved position of 100.
#[test]
fn mid_batch_crash_replays_only_uncommitted_updates() {
    let path = temp_config_path("cursor_crash_sim");
    // Initial disk state: cursor at 100
    let initial = TelegramConfig { last_update_id: Some(100), ..Default::default() };
    config::save_config(&path, &initial).unwrap();

    // Process only update 101, then "crash" before 102 and 103
    let mut cfg = initial.clone();
    crate::telegram::runtime::commit_update_cursor(&mut cfg, 101, &path);

    // Restart: load from disk
    let reloaded = config::load_config(&path).unwrap();
    // Restart offset = last saved id + 1 = 102; update 101 is NOT replayed
    let restart_offset = reloaded.last_update_id.map(|id| id + 1);
    assert_eq!(restart_offset, Some(102));
    let _ = std::fs::remove_file(&path);
}

// --- is_bot_own_message tests ---

#[test]
fn bot_own_message_matches_by_user_id() {
    // from_id matches bot_user_id → filter it out
    assert!(crate::telegram::runtime::is_bot_own_message(Some(42), Some(42)));
}

#[test]
fn different_user_is_not_filtered() {
    assert!(!crate::telegram::runtime::is_bot_own_message(Some(99), Some(42)));
}

#[test]
fn unknown_bot_id_passes_through() {
    // If we don't know the bot id yet, don't drop messages
    assert!(!crate::telegram::runtime::is_bot_own_message(Some(42), None));
}

#[test]
fn missing_sender_passes_through() {
    // No from field (e.g. channel posts) — pass through
    assert!(!crate::telegram::runtime::is_bot_own_message(None, Some(42)));
}

#[test]
fn bot_user_id_persisted_in_config() {
    // Verify TelegramConfig carries bot_user_id and round-trips through serde
    let path = temp_config_path("bot_user_id_persist");
    let cfg = TelegramConfig { bot_user_id: Some(123456789), ..Default::default() };
    config::save_config(&path, &cfg).unwrap();
    let loaded = config::load_config(&path).unwrap();
    assert_eq!(loaded.bot_user_id, Some(123456789));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn old_config_without_bot_user_id_loads_as_none() {
    // Backward compat: JSON without bot_user_id deserializes to None
    let path = temp_config_path("bot_user_id_compat");
    std::fs::write(
        &path,
        r#"{"enabled":false,"bot_token":"","notifications_enabled":false}"#,
    ).unwrap();
    let loaded = config::load_config(&path).unwrap();
    assert_eq!(loaded.bot_user_id, None);
    let _ = std::fs::remove_file(&path);
}

// --- RecentUpdateGuard dedup tests ---

#[test]
fn dedup_guard_new_id_is_not_duplicate() {
    let mut guard = crate::telegram::runtime::RecentUpdateGuard::new(64);
    assert!(!guard.check_and_insert(1001), "first occurrence must not be a duplicate");
}

#[test]
fn dedup_guard_repeated_id_is_duplicate() {
    let mut guard = crate::telegram::runtime::RecentUpdateGuard::new(64);
    guard.check_and_insert(1001);
    assert!(guard.check_and_insert(1001), "second occurrence must be detected as duplicate");
}

#[test]
fn dedup_guard_different_ids_are_independent() {
    let mut guard = crate::telegram::runtime::RecentUpdateGuard::new(64);
    assert!(!guard.check_and_insert(1001));
    assert!(!guard.check_and_insert(1002));
    // both still blocked on repeat
    assert!(guard.check_and_insert(1001));
    assert!(guard.check_and_insert(1002));
}

#[test]
fn dedup_guard_bounded_evicts_oldest_allowing_readmission() {
    // capacity=3: insert 100, 101, 102 → full.
    // All three are blocked. Inserting 103 evicts 100 (oldest).
    // After eviction, 100 is readmitted; 102 and 103 remain blocked.
    let mut guard = crate::telegram::runtime::RecentUpdateGuard::new(3);
    guard.check_and_insert(100);
    guard.check_and_insert(101);
    guard.check_and_insert(102);
    // at capacity — all three blocked
    assert!(guard.check_and_insert(100));
    assert!(guard.check_and_insert(101));
    assert!(guard.check_and_insert(102));
    // insert 103 evicts 100 (oldest); window is now [101, 102, 103]
    guard.check_and_insert(103);
    // 100 is no longer in the window — readmitted
    // (re-inserting 100 evicts 101; window becomes [102, 103, 100])
    assert!(!guard.check_and_insert(100), "evicted id must be readmittable");
    // 102 and 103 still within the current window — blocked
    assert!(guard.check_and_insert(102));
    assert!(guard.check_and_insert(103));
}

#[test]
fn dedup_guard_within_capacity_ids_remain_blocked() {
    let mut guard = crate::telegram::runtime::RecentUpdateGuard::new(64);
    for id in 1..=50 {
        guard.check_and_insert(id);
    }
    // All 50 IDs within capacity window are still blocked
    for id in 1..=50 {
        assert!(guard.check_and_insert(id), "id {id} within capacity must stay blocked");
    }
}
