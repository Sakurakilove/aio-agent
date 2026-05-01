use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// 中断信号类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptSignal {
    /// 用户手动中断 (Ctrl+C)
    UserCancel,
    /// 超时中断
    Timeout,
    /// 预算耗尽中断
    BudgetExhausted,
    /// Guardrails拦截中断
    GuardrailBlocked(String),
    /// 自定义中断
    Custom(String),
}

/// 中断处理器trait
pub trait InterruptHandler: Send + Sync {
    /// 处理中断信号，返回true表示继续执行，false表示终止
    fn handle(&self, signal: &InterruptSignal) -> bool;
}

/// 默认中断处理器
pub struct DefaultInterruptHandler {
    /// 是否允许在超时后继续
    pub allow_continue_on_timeout: bool,
}

impl DefaultInterruptHandler {
    pub fn new() -> Self {
        Self {
            allow_continue_on_timeout: false,
        }
    }
}

impl Default for DefaultInterruptHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptHandler for DefaultInterruptHandler {
    fn handle(&self, signal: &InterruptSignal) -> bool {
        match signal {
            InterruptSignal::UserCancel => false,
            InterruptSignal::Timeout => self.allow_continue_on_timeout,
            InterruptSignal::BudgetExhausted => false,
            InterruptSignal::GuardrailBlocked(_) => false,
            InterruptSignal::Custom(reason) => {
                eprintln!("中断信号: {}", reason);
                false
            }
        }
    }
}

/// 中断管理器
pub struct InterruptManager {
    handler: Arc<Mutex<Box<dyn InterruptHandler>>>,
    interrupted: Arc<Mutex<bool>>,
    interrupt_reason: Arc<Mutex<Option<InterruptSignal>>>,
}

impl InterruptManager {
    /// 创建新的中断管理器
    pub fn new(handler: Box<dyn InterruptHandler>) -> Self {
        Self {
            handler: Arc::new(Mutex::new(handler)),
            interrupted: Arc::new(Mutex::new(false)),
            interrupt_reason: Arc::new(Mutex::new(None)),
        }
    }

    /// 使用默认处理器创建
    pub fn default_handler() -> Self {
        Self::new(Box::new(DefaultInterruptHandler::new()))
    }

    /// 触发中断
    pub async fn interrupt(&self, signal: InterruptSignal) -> bool {
        let handler = self.handler.lock().await;
        let should_continue = handler.handle(&signal);

        if !should_continue {
            *self.interrupted.lock().await = true;
            *self.interrupt_reason.lock().await = Some(signal);
        }

        should_continue
    }

    /// 检查是否已被中断
    pub async fn is_interrupted(&self) -> bool {
        *self.interrupted.lock().await
    }

    /// 获取中断原因
    pub async fn get_interrupt_reason(&self) -> Option<InterruptSignal> {
        self.interrupt_reason.lock().await.clone()
    }

    /// 重置中断状态
    pub async fn reset(&self) {
        *self.interrupted.lock().await = false;
        *self.interrupt_reason.lock().await = None;
    }
}
