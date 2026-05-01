use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Webhook,
    Custom(String),
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Telegram => write!(f, "telegram"),
            ChannelType::Discord => write!(f, "discord"),
            ChannelType::Slack => write!(f, "slack"),
            ChannelType::WhatsApp => write!(f, "whatsapp"),
            ChannelType::Webhook => write!(f, "webhook"),
            ChannelType::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccount {
    pub id: String,
    pub channel_type: ChannelType,
    pub enabled: bool,
    pub configured: bool,
    pub token: Option<String>,
    pub webhook_url: Option<String>,
    pub extra_config: Option<HashMap<String, String>>,
}

impl ChannelAccount {
    pub fn new(id: &str, channel_type: ChannelType) -> Self {
        Self {
            id: id.to_string(),
            channel_type,
            enabled: false,
            configured: false,
            token: None,
            webhook_url: None,
            extra_config: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self.configured = true;
        self
    }

    pub fn with_webhook(mut self, webhook_url: String) -> Self {
        self.webhook_url = Some(webhook_url);
        self.configured = true;
        self
    }

    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn is_ready(&self) -> bool {
        self.enabled && self.configured
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMessage {
    pub id: String,
    pub channel_type: ChannelType,
    pub channel_id: String,
    pub sender_id: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl GatewayMessage {
    pub fn new(channel_type: ChannelType, channel_id: String, sender_id: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel_type,
            channel_id,
            sender_id,
            content,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub auth_token: Option<String>,
    pub max_connections: usize,
    pub channels: Vec<ChannelAccount>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            tls_enabled: false,
            auth_token: None,
            max_connections: 1000,
            channels: Vec::new(),
        }
    }
}

pub struct GatewayBuilder {
    config: GatewayConfig,
}

impl GatewayBuilder {
    pub fn new() -> Self {
        Self {
            config: GatewayConfig::default(),
        }
    }

    pub fn host(mut self, host: &str) -> Self {
        self.config.host = host.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn auth_token(mut self, token: &str) -> Self {
        self.config.auth_token = Some(token.to_string());
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }

    pub fn add_channel(mut self, channel: ChannelAccount) -> Self {
        self.config.channels.push(channel);
        self
    }

    pub fn build(self) -> GatewayServer {
        GatewayServer {
            config: self.config,
        }
    }
}

enum ChannelStatus {
    Stopped,
    Running,
    Error(String),
}

struct ChannelRuntime {
    account: ChannelAccount,
    status: ChannelStatus,
    abort_handle: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct GatewayServer {
    pub config: GatewayConfig,
}

impl GatewayServer {
    pub async fn start_all(&self) -> Result<()> {
        info!(
            "Starting gateway server on {}:{}",
            self.config.host, self.config.port
        );

        for channel in &self.config.channels {
            if !channel.enabled {
                info!("Skipping disabled channel: {}", channel.id);
                continue;
            }
            if !channel.configured {
                warn!("Channel {} is enabled but not configured", channel.id);
                continue;
            }

            info!("Starting channel: {} ({})", channel.id, channel.channel_type);
        }

        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        info!("Stopping gateway server");
        Ok(())
    }

    pub async fn start_channel(&self, channel_id: &str) -> Result<()> {
        let channel = self.config.channels.iter().find(|c| c.id == channel_id)
            .ok_or_else(|| anyhow!("Channel not found: {}", channel_id))?;

        if !channel.configured {
            return Err(anyhow!("Channel {} is not configured", channel_id));
        }

        info!("Starting channel: {}", channel_id);
        let (_tx, _rx) = tokio::sync::oneshot::channel::<()>();

        info!("Channel {} started", channel_id);
        Ok(())
    }

    async fn stop_channel(&self, channel_id: &str) -> Result<()> {
        info!("Stopping channel: {}", channel_id);
        Ok(())
    }

    pub async fn send_message(
        &self,
        channel_type: &ChannelType,
        channel_id: &str,
        recipient_id: &str,
        content: &str,
    ) -> Result<()> {
        let msg = GatewayMessage::new(
            channel_type.clone(),
            channel_id.to_string(),
            "system".to_string(),
            content.to_string(),
        );

        info!(
            "Sending message to {} via {} ({})",
            recipient_id, channel_type, channel_id
        );

        Ok(())
    }

    pub async fn receive_message(&self, channel_type: &ChannelType, channel_id: &str) -> Result<GatewayMessage> {
        let msg = GatewayMessage::new(
            channel_type.clone(),
            channel_id.to_string(),
            "user".to_string(),
            "Test message".to_string(),
        );

        Ok(msg)
    }
}
