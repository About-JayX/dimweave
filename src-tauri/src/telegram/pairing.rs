use rand::Rng;

/// Generate a 6-digit pair code with 10-minute expiry.
pub fn generate_pair_code(now_ms: u64) -> (String, u64) {
    let code: u32 = rand::rng().random_range(100_000..1_000_000);
    let expires_at = now_ms + 10 * 60 * 1000;
    (code.to_string(), expires_at)
}

/// Extract the code from a `/pair <code>` message.
pub fn match_pair_command(text: &str) -> Option<&str> {
    text.strip_prefix("/pair ").map(str::trim)
}

/// Check if a pairing code is still valid.
pub fn is_code_valid(pending: Option<&str>, expires_at: Option<u64>, now_ms: u64) -> bool {
    pending.is_some() && expires_at.map_or(false, |exp| now_ms < exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_six_digit_code() {
        let (code, _) = generate_pair_code(1000);
        assert_eq!(code.len(), 6);
        assert!(code.parse::<u32>().is_ok());
    }

    #[test]
    fn generate_expiry_is_ten_minutes_ahead() {
        let now = 1_000_000;
        let (_, expires_at) = generate_pair_code(now);
        assert_eq!(expires_at, now + 600_000);
    }

    #[test]
    fn match_pair_command_extracts_code() {
        assert_eq!(match_pair_command("/pair 123456"), Some("123456"));
        assert_eq!(match_pair_command("/pair  654321 "), Some("654321"));
        assert_eq!(match_pair_command("hello"), None);
        assert_eq!(match_pair_command("/start"), None);
    }

    #[test]
    fn expired_code_is_invalid() {
        assert!(!is_code_valid(Some("123456"), Some(500), 1000));
        assert!(is_code_valid(Some("123456"), Some(2000), 1000));
        assert!(!is_code_valid(None, Some(2000), 1000));
    }
}
