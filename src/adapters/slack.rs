use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct SlackAdapter {
    pub bot_token: String,
    pub app_token: String,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl SlackAdapter {
    pub fn new(bot_token: &str, app_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            app_token: app_token.to_string(),
            allowed_users: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }
}

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        self.bot_token = config.token;
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = "https://slack.com/api/chat.postMessage";
        
        let body = serde_json::json!({
            "channel": chat_id,
            "text": content,
        });

        let response = client.post(url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if json["ok"].as_bool().unwrap_or(false) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Slack API error: {}", json["error"].as_str().unwrap_or("unknown")))
        }
    }

    async fn get_updates(&self, _offset: Option<u64>) -> Result<Vec<PlatformMessage>> {
        Ok(Vec::new())
    }

    async fn set_webhook(&self, _webhook_url: &str) -> Result<()> {
        Ok(())
    }

    async fn delete_webhook(&self) -> Result<()> {
        Ok(())
    }

    async fn get_me(&self) -> Result<Value> {
        let client = reqwest::Client::new();
        let url = "https://slack.com/api/auth.test";
        
        let response = client.post(url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .send()
            .await?;

        let json: Value = response.json().await?;
        Ok(json)
    }

    fn name(&self) -> &str {
        "slack"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.allowed_users.is_empty() {
            return true;
        }
        self.allowed_users.iter().any(|u| u == user_id)
    }
}
