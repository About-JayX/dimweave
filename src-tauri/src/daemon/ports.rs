/// Centralized port configuration for the daemon.
/// Reads from environment variables with compile-time defaults.
/// Set `DIMWEAVE_DAEMON_PORT`, `DIMWEAVE_CODEX_PORT` to override.
#[derive(Clone, Copy)]
pub struct PortConfig {
    pub daemon: u16,
    pub codex: u16,
}

const DEFAULT_DAEMON_PORT: u16 = 4502;
const DEFAULT_CODEX_PORT: u16 = 4500;

impl PortConfig {
    pub fn from_env() -> Self {
        Self {
            daemon: parse_port_env("DIMWEAVE_DAEMON_PORT", DEFAULT_DAEMON_PORT),
            codex: parse_port_env("DIMWEAVE_CODEX_PORT", DEFAULT_CODEX_PORT),
        }
    }
}

fn parse_port_env(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ports() {
        let cfg = PortConfig::from_env();
        assert_eq!(cfg.daemon, DEFAULT_DAEMON_PORT);
        assert_eq!(cfg.codex, DEFAULT_CODEX_PORT);
    }
}
