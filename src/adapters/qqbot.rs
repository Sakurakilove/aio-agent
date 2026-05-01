use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct QQBotAdapter {
    pub app_id: String,
    pub token: String,
    pub app_secret: String,
    pub sandbox: bool,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl QQBotAdapter {
    pub fn new(app_id: &str, token: &str, app_secret: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            token: token.to_string(),
            app_secret: app_secret.to_string(),
            sandbox: false,
            allowed_users: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_sandbox(mut self, sandbox: bool) -> Self {
        self.sandbox = sandbox;
        self
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }

    fn base_url(&self) -> String {
        if self.sandbox {
            "https://sandbox.api.bot.qzone.qq.com".to_string()
        } else {
            "https://api.sgroup.qq.com".to_string()
        }
    }

    async fn get_access_token(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let url = "https://bots.qq.com/app/getAppAccessToken";
        
        let body = serde_json::json!({
            "appId": self.app_id,
            "clientSecret": self.app_secret,
        });

        let response = client.post(url)
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if let Some(token) = json["access_token"].as_str() {
            Ok(token.to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get QQ access token"))
        }
    }
}

#[async_trait]
impl ChannelAdapter for QQBotAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        self.token = config.token;
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let url = format!("{}/channels/{}/messages", self.base_url(), channel_id);
        
        let body = serde_json::json!({
            "content": content,
            "msg_type": 0,
        });

        let response = client.post(&url)
            .header("Authorization", format!("QQBot {}", token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("QQ send error: {}", error_text));
        }

        Ok(())
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
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let url = format!("{}/users/@me", self.base_url());
        
        let response = client.get(&url)
            .header("Authorization", format!("QQBot {}", token))
            .send()
            .await?;

        let json: Value = response.json().await?;
        Ok(json)
    }

    fn name(&self) -> &str {
        "qqbot"
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
