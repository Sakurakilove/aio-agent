//! 聊天平台适配器类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 平台类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Signal,
    Matrix,
    Teams,
    Webhook,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Telegram => write!(f, "telegram"),
            ChannelType::Discord => write!(f, "discord"),
            ChannelType::Slack => write!(f, "slack"),
            ChannelType::WhatsApp => write!(f, "whatsapp"),
            ChannelType::Signal => write!(f, "signal"),
            ChannelType::Matrix => write!(f, "matrix"),
            ChannelType::Teams => write!(f, "teams"),
            ChannelType::Webhook => write!(f, "webhook"),
        }
    }
}

impl ChannelType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "telegram" => Some(ChannelType::Telegram),
            "discord" => Some(ChannelType::Discord),
            "slack" => Some(ChannelType::Slack),
            "whatsapp" => Some(ChannelType::WhatsApp),
            "signal" => Some(ChannelType::Signal),
            "matrix" => Some(ChannelType::Matrix),
            "teams" => Some(ChannelType::Teams),
            "webhook" => Some(ChannelType::Webhook),
            _ => None,
        }
    }
}

/// 通道账户配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccount {
    pub id: String,
    pub channel_type: ChannelType,
    pub enabled: bool,
    pub token: Option<String>,
    pub app_token: Option<String>,
    pub allowed_users: Vec<String>,
    pub home_channel: Option<String>,
    pub extra: HashMap<String, String>,
}

impl ChannelAccount {
    pub fn new(id: &str, channel_type: ChannelType) -> Self {
        Self {
            id: id.to_string(),
            channel_type,
            enabled: false,
            token: None,
            app_token: None,
            allowed_users: Vec::new(),
            home_channel: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub fn with_app_token(mut self, app_token: String) -> Self {
        self.app_token = Some(app_token);
        self
    }

    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }

    pub fn with_home_channel(mut self, channel: &str) -> Self {
        self.home_channel = Some(channel.to_string());
        self
    }

    pub fn with_extra(mut self, key: &str, value: &str) -> Self {
        self.extra.insert(key.to_string(), value.to_string());
        self
    }
}

/// 平台消息
#[derive(Debug, Clone)]
pub struct PlatformMessage {
    pub chat_id: String,
    pub user_id: String,
    pub text: String,
    pub timestamp: i64,
}
