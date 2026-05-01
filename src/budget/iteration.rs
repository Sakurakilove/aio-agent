use std::sync::atomic::{AtomicUsize, Ordering};

pub struct IterationBudget {
    max_total: usize,
    used: AtomicUsize,
}

impl IterationBudget {
    pub fn new(max_total: usize) -> Self {
        Self {
            max_total,
            used: AtomicUsize::new(0),
        }
    }

    pub fn consume(&self) -> bool {
        let current = self.used.load(Ordering::SeqCst);
        if current >= self.max_total {
            false
        } else {
            self.used.fetch_add(1, Ordering::SeqCst);
            true
        }
    }

    pub fn refund(&self) {
        let current = self.used.load(Ordering::SeqCst);
        if current > 0 {
            self.used.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn remaining(&self) -> usize {
        let current = self.used.load(Ordering::SeqCst);
        self.max_total.saturating_sub(current)
    }

    pub fn used(&self) -> usize {
        self.used.load(Ordering::SeqCst)
    }
}
