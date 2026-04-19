use super::*;

const SAMPLE: &str = r#"{
  "account": {
    "uuid": "abc",
    "full_name": "Jayson",
    "display_name": "Jayson",
    "email": "aboutjayx@gmail.com",
    "has_claude_max": true,
    "has_claude_pro": false,
    "created_at": "2025-12-02T03:28:00Z"
  },
  "organization": {
    "uuid": "org-1",
    "name": "Jayson's Org",
    "organization_type": "claude_max",
    "billing_type": "stripe_subscription",
    "rate_limit_tier": "default_claude_max_20x",
    "subscription_status": "active"
  },
  "application": {
    "uuid": "app-1",
    "name": "Claude Code",
    "slug": "claude-code"
  }
}"#;

#[test]
fn parses_profile_with_max_subscription() {
    let p = parse_profile(SAMPLE).unwrap();
    assert_eq!(p.email, "aboutjayx@gmail.com");
    assert_eq!(p.display_name, "Jayson");
    assert_eq!(p.subscription_tier, "max");
    assert_eq!(p.rate_limit_tier, "default_claude_max_20x");
    assert_eq!(p.organization_name, "Jayson's Org");
    assert_eq!(p.subscription_status, "active");
}

#[test]
fn prefers_pro_over_free_when_max_absent() {
    let body = r#"{"account":{"email":"x@x","display_name":"X","has_claude_max":false,"has_claude_pro":true},"organization":{"name":"O","rate_limit_tier":"t","subscription_status":"active"}}"#;
    let p = parse_profile(body).unwrap();
    assert_eq!(p.subscription_tier, "pro");
}

#[test]
fn falls_back_to_free_when_neither_flag_is_set() {
    let body = r#"{"account":{"email":"x@x","display_name":"X"},"organization":{"name":"O","rate_limit_tier":"t","subscription_status":"active"}}"#;
    let p = parse_profile(body).unwrap();
    assert_eq!(p.subscription_tier, "free");
}

#[test]
fn missing_fields_default_to_empty_string() {
    let body = r#"{"account":{},"organization":{}}"#;
    let p = parse_profile(body).unwrap();
    assert_eq!(p.email, "");
    assert_eq!(p.rate_limit_tier, "");
}

#[test]
fn malformed_json_returns_err() {
    let err = parse_profile("{{}").unwrap_err();
    assert!(err.starts_with("parse profile json"));
}
