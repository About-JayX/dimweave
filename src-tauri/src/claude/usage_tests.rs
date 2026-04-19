use super::*;
use reqwest::header::{HeaderMap, HeaderValue};

fn headers_from(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
    let mut h = HeaderMap::new();
    for (k, v) in pairs {
        h.insert(*k, HeaderValue::from_static(v));
    }
    h
}

#[test]
fn parses_both_windows_with_full_metadata() {
    let h = headers_from(&[
        ("anthropic-ratelimit-unified-status", "allowed"),
        ("anthropic-ratelimit-unified-5h-status", "allowed"),
        ("anthropic-ratelimit-unified-5h-reset", "1776574800"),
        ("anthropic-ratelimit-unified-5h-utilization", "0.23"),
        ("anthropic-ratelimit-unified-7d-status", "allowed"),
        ("anthropic-ratelimit-unified-7d-reset", "1776981600"),
        ("anthropic-ratelimit-unified-7d-utilization", "0.49"),
    ]);
    let u = parse_usage_headers(&h).unwrap();
    assert_eq!(u.overall_status, "allowed");
    let five = u.five_hour.unwrap();
    assert!((five.utilization - 0.23).abs() < 1e-9);
    assert_eq!(five.resets_at, Some(1776574800));
    assert_eq!(five.status, "allowed");
    let seven = u.seven_day.unwrap();
    assert!((seven.utilization - 0.49).abs() < 1e-9);
    assert_eq!(seven.resets_at, Some(1776981600));
}

#[test]
fn window_absent_when_utilization_header_missing() {
    let h = headers_from(&[
        ("anthropic-ratelimit-unified-status", "allowed"),
        ("anthropic-ratelimit-unified-5h-utilization", "0.1"),
        ("anthropic-ratelimit-unified-5h-reset", "100"),
    ]);
    let u = parse_usage_headers(&h).unwrap();
    assert!(u.five_hour.is_some());
    assert!(u.seven_day.is_none());
}

#[test]
fn overall_status_defaults_to_unknown_when_absent() {
    let h = headers_from(&[]);
    let u = parse_usage_headers(&h).unwrap();
    assert_eq!(u.overall_status, "unknown");
    assert!(u.five_hour.is_none());
    assert!(u.seven_day.is_none());
}

#[test]
fn non_numeric_utilization_drops_the_window() {
    let h = headers_from(&[
        ("anthropic-ratelimit-unified-5h-utilization", "notanumber"),
    ]);
    let u = parse_usage_headers(&h).unwrap();
    assert!(u.five_hour.is_none());
}
