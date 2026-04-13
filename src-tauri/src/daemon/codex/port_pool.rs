use std::collections::{HashMap, HashSet};

const POOL_SIZE: u16 = 8;

/// Port lease states.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LeaseState {
    /// Reserved for a task launch but not yet online.
    Reserved { task_id: String },
    /// Actively used by an online Codex session.
    Live { task_id: String },
}

/// Codex port allocator with reservation tracking.
/// Ports transition: Free → Reserved → Live → Free.
/// A reserved port is invisible to `codex_used_ports()` (which only
/// sees online slots), but the pool itself knows it is taken.
pub struct CodexPortPool {
    base_port: u16,
    pool_size: u16,
    leases: HashMap<u16, LeaseState>,
}

impl CodexPortPool {
    pub fn new(base_port: u16) -> Self {
        Self {
            base_port,
            pool_size: POOL_SIZE,
            leases: HashMap::new(),
        }
    }

    /// Reserve the first free port for `task_id`.
    /// Returns `None` if the pool is exhausted.
    pub fn reserve(&mut self, task_id: &str) -> Option<u16> {
        let port = (self.base_port..self.base_port + self.pool_size)
            .find(|p| !self.leases.contains_key(p))?;
        self.leases.insert(
            port,
            LeaseState::Reserved {
                task_id: task_id.to_string(),
            },
        );
        Some(port)
    }

    /// Promote a reserved port to live. No-op if the port is not
    /// reserved by `task_id` (stale callback guard).
    pub fn promote(&mut self, port: u16, task_id: &str) -> bool {
        match self.leases.get(&port) {
            Some(LeaseState::Reserved { task_id: owner }) if owner == task_id => {
                self.leases.insert(
                    port,
                    LeaseState::Live {
                        task_id: task_id.to_string(),
                    },
                );
                true
            }
            _ => false,
        }
    }

    /// Release a port only if it is owned by `task_id`.
    /// Stale callbacks for a different task are silently ignored.
    pub fn release(&mut self, port: u16, task_id: &str) -> bool {
        match self.leases.get(&port) {
            Some(LeaseState::Reserved { task_id: owner })
            | Some(LeaseState::Live { task_id: owner })
                if owner == task_id =>
            {
                self.leases.remove(&port);
                true
            }
            _ => false,
        }
    }

    /// Release all ports owned by `task_id`.
    pub fn release_all_for_task(&mut self, task_id: &str) {
        self.leases
            .retain(|_, lease| match lease {
                LeaseState::Reserved { task_id: owner }
                | LeaseState::Live { task_id: owner } => owner != task_id,
            });
    }

    /// True if `port` falls within this pool's range.
    pub fn contains(&self, port: u16) -> bool {
        port >= self.base_port && port < self.base_port + self.pool_size
    }

    pub fn pool_size(&self) -> u16 {
        self.pool_size
    }

    /// Ports currently leased (reserved or live).
    pub fn leased_ports(&self) -> HashSet<u16> {
        self.leases.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_port_pool_allocates_first_available() {
        let mut pool = CodexPortPool::new(4500);
        assert_eq!(pool.reserve("task_a"), Some(4500));
    }

    #[test]
    fn codex_port_pool_skips_reserved_ports() {
        let mut pool = CodexPortPool::new(4500);
        pool.reserve("task_a");
        assert_eq!(pool.reserve("task_b"), Some(4501));
    }

    #[test]
    fn codex_port_pool_returns_none_when_exhausted() {
        let mut pool = CodexPortPool::new(4500);
        for i in 0..8 {
            pool.reserve(&format!("task_{i}"));
        }
        assert_eq!(pool.reserve("task_overflow"), None);
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
        pool.reserve("task_a");
        assert_eq!(pool.reserve("task_b"), Some(4501));
        pool.release(4500, "task_a");
        assert_eq!(pool.reserve("task_c"), Some(4500));
    }

    #[test]
    fn codex_port_pool_promote_guards_owner() {
        let mut pool = CodexPortPool::new(4500);
        pool.reserve("task_a");
        assert!(!pool.promote(4500, "task_b"), "wrong owner rejected");
        assert!(pool.promote(4500, "task_a"), "correct owner accepted");
    }

    #[test]
    fn codex_port_pool_release_guards_owner() {
        let mut pool = CodexPortPool::new(4500);
        pool.reserve("task_a");
        assert!(!pool.release(4500, "task_b"), "wrong owner rejected");
        assert!(pool.release(4500, "task_a"), "correct owner accepted");
    }

    #[test]
    fn codex_port_pool_reserved_port_not_reallocated() {
        let mut pool = CodexPortPool::new(4500);
        pool.reserve("task_a");
        // task_a is reserved but not yet online — must not be reallocated
        assert_eq!(pool.reserve("task_b"), Some(4501));
        assert_ne!(pool.reserve("task_c"), Some(4500));
    }

    #[test]
    fn codex_port_pool_stale_release_ignored() {
        let mut pool = CodexPortPool::new(4500);
        pool.reserve("task_a");
        pool.promote(4500, "task_a");
        // task_a releases
        pool.release(4500, "task_a");
        // task_b reserves same port
        pool.reserve("task_b");
        // stale task_a callback tries to release — must fail
        assert!(!pool.release(4500, "task_a"));
        assert!(pool.leases.contains_key(&4500));
    }

    #[test]
    fn codex_port_pool_release_all_for_task() {
        let mut pool = CodexPortPool::new(4500);
        pool.reserve("task_a");
        pool.reserve("task_a"); // 4501
        pool.reserve("task_b"); // 4502
        pool.release_all_for_task("task_a");
        assert_eq!(pool.leased_ports().len(), 1);
        assert!(pool.leased_ports().contains(&4502));
    }

    /// AC: reserved lease becomes live on successful handshake
    #[test]
    fn codex_port_pool_reserve_promote_release_lifecycle() {
        let mut pool = CodexPortPool::new(4500);
        // Phase 1: reserve
        let port = pool.reserve("task_a").unwrap();
        assert_eq!(port, 4500);
        assert!(pool.leased_ports().contains(&4500));
        // Phase 2: promote (handshake success)
        assert!(pool.promote(port, "task_a"));
        assert!(pool.leased_ports().contains(&4500));
        // Phase 3: release (session end / exit notice)
        assert!(pool.release(port, "task_a"));
        assert!(!pool.leased_ports().contains(&4500));
        // Port is now available for re-reservation
        assert_eq!(pool.reserve("task_b"), Some(4500));
    }

    /// AC: natural process exit releases the leased port via exit notice
    #[test]
    fn codex_port_pool_exit_notice_releases_live_port() {
        let mut pool = CodexPortPool::new(4500);
        let port_a = pool.reserve("task_a").unwrap();
        let port_b = pool.reserve("task_b").unwrap();
        pool.promote(port_a, "task_a");
        pool.promote(port_b, "task_b");
        assert_eq!(pool.leased_ports().len(), 2);
        // Simulate exit notice for task_a only
        pool.release(port_a, "task_a");
        assert_eq!(pool.leased_ports().len(), 1);
        assert!(!pool.leased_ports().contains(&port_a));
        assert!(pool.leased_ports().contains(&port_b));
    }

    /// AC: stale exit notice for a port re-assigned to another task is ignored
    #[test]
    fn codex_port_pool_stale_exit_notice_after_reassignment() {
        let mut pool = CodexPortPool::new(4500);
        let port = pool.reserve("task_a").unwrap();
        pool.promote(port, "task_a");
        pool.release(port, "task_a");
        // Port reassigned to task_b
        let port2 = pool.reserve("task_b").unwrap();
        assert_eq!(port, port2);
        pool.promote(port2, "task_b");
        // Stale exit notice from task_a arrives — must not release task_b's port
        assert!(!pool.release(port, "task_a"));
        assert!(pool.leased_ports().contains(&port));
    }
}
