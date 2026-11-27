//! Monotonic wall-clock and node accounting for a synchronous search.

use std::time::{Duration, Instant};

/// Limits checker calls; an already running check cannot be interrupted here.
#[derive(Debug)]
pub(crate) struct Budget {
    started: Instant,
    time_limit: Duration,
    max_nodes: usize,
    visited: usize,
}

impl Budget {
    pub(crate) fn new(max_nodes: usize, time_budget_ms: u64) -> Self {
        Self {
            started: Instant::now(),
            time_limit: Duration::from_millis(time_budget_ms),
            max_nodes,
            visited: 0,
        }
    }

    /// Reserve one checker call only if both limits still permit it.
    pub(crate) fn try_visit(&mut self) -> bool {
        if self.is_exhausted() {
            return false;
        }
        self.visited += 1;
        true
    }

    pub(crate) fn is_exhausted(&self) -> bool {
        self.visited >= self.max_nodes || self.started.elapsed() >= self.time_limit
    }

    pub(crate) fn nodes_visited(&self) -> usize {
        self.visited
    }

    pub(crate) fn millis(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
