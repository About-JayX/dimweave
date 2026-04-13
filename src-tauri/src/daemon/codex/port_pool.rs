use std::collections::HashSet;

const POOL_SIZE: u16 = 8;

/// Stateless Codex port allocator.
/// Given a base port and the set of currently used ports,
/// returns the next available port from [base, base + POOL_SIZE).
pub struct CodexPortPool {
    base_port: u16,
    pool_size: u16,
}

impl CodexPortPool {
    pub fn new(base_port: u16) -> Self {
        Self {
            base_port,
            pool_size: POOL_SIZE,
        }
    }

    /// Find the first available port not in `used`.
    pub fn allocate(&self, used: &HashSet<u16>) -> Option<u16> {
        (self.base_port..self.base_port + self.pool_size).find(|p| !used.contains(p))
    }

    /// True if `port` falls within this pool's range.
    pub fn contains(&self, port: u16) -> bool {
        port >= self.base_port && port < self.base_port + self.pool_size
    }

    pub fn pool_size(&self) -> u16 {
        self.pool_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_port_pool_allocates_first_available() {
        let pool = CodexPortPool::new(4500);
        let used = HashSet::new();
        assert_eq!(pool.allocate(&used), Some(4500));
    }

    #[test]
    fn codex_port_pool_skips_used_ports() {
        let pool = CodexPortPool::new(4500);
        let used: HashSet<u16> = [4500, 4501].into();
        assert_eq!(pool.allocate(&used), Some(4502));
    }

    #[test]
    fn codex_port_pool_returns_none_when_exhausted() {
        let pool = CodexPortPool::new(4500);
        let used: HashSet<u16> = (4500..4508).collect();
        assert_eq!(pool.allocate(&used), None);
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
        let pool = CodexPortPool::new(4500);
        let mut used: HashSet<u16> = [4500].into();
        assert_eq!(pool.allocate(&used), Some(4501));
        // Simulate release
        used.remove(&4500);
        assert_eq!(pool.allocate(&used), Some(4500));
    }
}
