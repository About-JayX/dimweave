use std::collections::{HashMap, HashSet};

const POOL_SIZE: u16 = 8;

/// Port lease states.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LeaseState {
    /// Reserved for a task launch but not yet online.
    Reserved { task_id: String, launch_id: u64 },
    /// Actively used by an online Codex session.
    Live { task_id: String, launch_id: u64 },
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

    /// Reserve the first free port for `task_id` + `launch_id`.
    /// Returns `None` if the pool is exhausted.
    pub fn reserve(&mut self, task_id: &str, launch_id: u64) -> Option<u16> {
        let port = (self.base_port..self.base_port + self.pool_size)
            .find(|p| !self.leases.contains_key(p))?;
        self.leases.insert(
            port,
            LeaseState::Reserved {
                task_id: task_id.to_string(),
                launch_id,
            },
        );
        Some(port)
    }

    /// Promote a reserved port to live. No-op if the port is not
    /// reserved by the exact `(task_id, launch_id)` pair.
    pub fn promote(&mut self, port: u16, task_id: &str, launch_id: u64) -> bool {
        match self.leases.get(&port) {
            Some(LeaseState::Reserved {
                task_id: owner,
                launch_id: lid,
            }) if owner == task_id && *lid == launch_id => {
                self.leases.insert(
                    port,
                    LeaseState::Live {
                        task_id: task_id.to_string(),
                        launch_id,
                    },
                );
                true
            }
            _ => false,
        }
    }

    /// Release a port only if it is owned by the exact `(task_id, launch_id)`.
    /// Stale callbacks from an older launch of the same task are silently ignored.
    pub fn release(&mut self, port: u16, task_id: &str, launch_id: u64) -> bool {
        match self.leases.get(&port) {
            Some(LeaseState::Reserved {
                task_id: owner,
                launch_id: lid,
            })
            | Some(LeaseState::Live {
                task_id: owner,
                launch_id: lid,
            }) if owner == task_id && *lid == launch_id => {
                self.leases.remove(&port);
                true
            }
            _ => false,
        }
    }

    /// Release all ports owned by `task_id` (any launch_id).
    pub fn release_all_for_task(&mut self, task_id: &str) {
        self.leases
            .retain(|_, lease| match lease {
                LeaseState::Reserved { task_id: owner, .. }
                | LeaseState::Live { task_id: owner, .. } => owner != task_id,
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
#[path = "port_pool_tests.rs"]
mod tests;
