//! Ephemeral game-port allocation for remote stem nodes (plangem.md §3.3).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Thread-safe allocator over a configured inclusive port range.
#[derive(Clone)]
pub struct PortPool {
    inner: Arc<PortPoolInner>,
}

struct PortPoolInner {
    min: u16,
    max: u16,
    used: Mutex<HashSet<u16>>,
}

impl PortPool {
    pub fn new(min: u16, max: u16) -> Self {
        Self {
            inner: Arc::new(PortPoolInner {
                min,
                max,
                used: Mutex::new(HashSet::new()),
            }),
        }
    }

    pub fn allocate(&self) -> Option<u16> {
        let mut used = self.inner.used.lock().expect("port pool lock");
        for port in self.inner.min..=self.inner.max {
            if used.insert(port) {
                return Some(port);
            }
        }
        None
    }

    pub fn release(&self, port: u16) {
        let mut used = self.inner.used.lock().expect("port pool lock");
        used.remove(&port);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_sequential_ports() {
        let pool = PortPool::new(25_600, 25_602);
        assert_eq!(pool.allocate(), Some(25_600));
        assert_eq!(pool.allocate(), Some(25_601));
        pool.release(25_600);
        assert_eq!(pool.allocate(), Some(25_600));
    }
}
