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
