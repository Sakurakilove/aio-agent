use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackEvent {
    pub event_type: CallbackEventType,
    pub session_id: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackEventType {
    LlmStart,
    LlmEnd,
    LlmError,
    ToolStart,
    ToolEnd,
    ToolError,
    AgentStart,
    AgentEnd,
    AgentError,
    ContextCompressed,
    ProviderSwitched,
    DelegationCreated,
    DelegationCompleted,
}

pub trait CallbackHandler: Send + Sync {
    fn on_event(&self, event: &CallbackEvent);
}

pub struct LoggingCallback;

impl CallbackHandler for LoggingCallback {
    fn on_event(&self, event: &CallbackEvent) {
        tracing::info!(
            "[Callback] {:?} session={} data={}",
            event.event_type,
            event.session_id,
            event.data
        );
    }
}

pub struct CallbackManager {
    handlers: Vec<Arc<dyn CallbackHandler>>,
    event_log: Arc<Mutex<Vec<CallbackEvent>>>,
}

impl CallbackManager {
    pub fn new() -> Self {
        Self {
            handlers: vec![Arc::new(LoggingCallback)],
            event_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_handler(&mut self, handler: Arc<dyn CallbackHandler>) {
        self.handlers.push(handler);
    }

    pub fn emit(&self, event_type: CallbackEventType, session_id: &str, data: serde_json::Value) {
        let event = CallbackEvent {
            event_type,
            session_id: session_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data,
        };

        for handler in &self.handlers {
            handler.on_event(&event);
        }

        let log = self.event_log.clone();
        let event_clone = event.clone();
        tokio::spawn(async move {
            let mut log = log.lock().await;
            if log.len() >= 1000 {
                log.drain(0..500);
            }
            log.push(event_clone);
        });
    }

    pub async fn get_events(&self) -> Vec<CallbackEvent> {
        self.event_log.lock().await.clone()
    }

    pub async fn get_events_by_type(&self, event_type: &CallbackEventType) -> Vec<CallbackEvent> {
        let log = self.event_log.lock().await;
        log.iter()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(event_type))
            .cloned()
            .collect()
    }
}
