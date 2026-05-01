//! 平台适配器trait定义

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// 平台消息
#[derive(Debug, Clone)]
pub struct PlatformMessage {
    pub id: String,
    pub chat_id: String,
    pub sender_id: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub raw: Value,
}

impl PlatformMessage {
    pub fn new(chat_id: &str, sender_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            chat_id: chat_id.to_string(),
            sender_id: sender_id.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            raw: Value::Null,
        }
    }
}

/// 平台配置
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub token: String,
    pub webhook_url: Option<String>,
    pub extra: HashMap<String, String>,
}

impl PlatformConfig {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            webhook_url: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: &str, value: &str) -> Self {
        self.extra.insert(key.to_string(), value.to_string());
        self
    }
}

/// 聊天平台适配器trait
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()>;
    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()>;
    async fn get_updates(&self, offset: Option<u64>) -> Result<Vec<PlatformMessage>>;
    async fn set_webhook(&self, webhook_url: &str) -> Result<()>;
    async fn delete_webhook(&self) -> Result<()>;
    async fn get_me(&self) -> Result<Value>;
    fn is_initialized(&self) -> bool;
    fn is_user_allowed(&self, user_id: &str) -> bool;
}

/// 适配器工厂
pub struct AdapterFactory;

impl AdapterFactory {
    pub fn list_adapters() -> Vec<&'static str> {
        vec!["telegram", "discord", "slack", "whatsapp", "signal", "matrix", "teams", "webhook", "qqbot", "wecom", "feishu", "dingtalk"]
    }
}
