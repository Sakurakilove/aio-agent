use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct SignalAdapter {
    pub http_url: String,
    pub account: String,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl SignalAdapter {
    pub fn new(http_url: &str, account: &str) -> Self {
        Self {
            http_url: http_url.trim_end_matches('/').to_string(),
            account: account.to_string(),
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
impl ChannelAdapter for SignalAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        if let Some(url) = config.extra.get("http_url") {
            self.http_url = url.trim_end_matches('/').to_string();
        }
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v2/send", self.http_url);
        
        let body = serde_json::json!({
            "message": content,
            "number": self.account,
            "recipients": [chat_id],
        });

        let response = client.put(&url)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("Signal API error: {}", error_text))
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
        Ok(serde_json::json!({
            "account": self.account,
            "http_url": self.http_url,
        }))
    }

    fn name(&self) -> &str {
        "signal"
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
