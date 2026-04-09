use super::validate_bot_token;

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
