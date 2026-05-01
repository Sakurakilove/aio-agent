use std::sync::atomic::{AtomicBool, Ordering};

pub struct InterruptHandler {
    interrupted: AtomicBool,
    cleanup_callbacks: std::sync::Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
}

impl InterruptHandler {
    pub fn new() -> Self {
        Self {
            interrupted: AtomicBool::new(false),
            cleanup_callbacks: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn is_interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }

    pub fn trigger_interrupt(&self) {
        self.interrupted.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.interrupted.store(false, Ordering::SeqCst);
    }

    pub fn add_cleanup_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.cleanup_callbacks.lock() {
            callbacks.push(Box::new(callback));
        }
    }

    pub fn execute_cleanup(&self) {
        if let Ok(mut callbacks) = self.cleanup_callbacks.lock() {
            for callback in callbacks.iter() {
                callback();
            }
            callbacks.clear();
        }
    }

    pub fn check_interrupt(&self) -> Result<(), InterruptError> {
        if self.is_interrupted() {
            Err(InterruptError::UserCancelled)
        } else {
            Ok(())
        }
    }
}

pub enum InterruptError {
    UserCancelled,
    Timeout,
    SignalReceived,
    ResourceLimit,
}

impl std::fmt::Display for InterruptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterruptError::UserCancelled => write!(f, "操作已被用户取消"),
            InterruptError::Timeout => write!(f, "操作超时"),
            InterruptError::SignalReceived => write!(f, "收到系统信号"),
            InterruptError::ResourceLimit => write!(f, "资源限制触发中断"),
        }
    }
}
