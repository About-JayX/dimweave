pub(crate) const MAX_WS_RECONNECT_ATTEMPTS: u32 = 5;
const WS_RECONNECT_BASE_DELAY_MS: u64 = 500;

pub(crate) fn reconnect_delay_ms(attempt: u32) -> Option<u64> {
    if attempt == 0 || attempt > MAX_WS_RECONNECT_ATTEMPTS {
        return None;
    }
    Some(WS_RECONNECT_BASE_DELAY_MS * 2u64.pow(attempt - 1))
}
