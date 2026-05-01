use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ToolBudget {
    pub max_executions: usize,
    pub current_executions: AtomicUsize,
    pub max_time_seconds: u64,
    pub start_time: std::time::Instant,
}

impl ToolBudget {
    pub fn new(max_executions: usize, max_time_seconds: u64) -> Self {
        Self {
            max_executions,
            current_executions: AtomicUsize::new(0),
            max_time_seconds,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn can_execute(&self) -> bool {
        let current = self.current_executions.load(Ordering::SeqCst);
        let elapsed = self.start_time.elapsed().as_secs();
        current < self.max_executions && elapsed < self.max_time_seconds
    }

    pub fn record_execution(&self) {
        self.current_executions.fetch_add(1, Ordering::SeqCst);
    }

    pub fn remaining(&self) -> usize {
        let current = self.current_executions.load(Ordering::SeqCst);
        self.max_executions.saturating_sub(current)
    }

    pub fn time_remaining(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs();
        self.max_time_seconds.saturating_sub(elapsed)
    }
}
