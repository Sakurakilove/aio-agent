use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct TeamsAdapter {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl TeamsAdapter {
    pub fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            tenant_id: tenant_id.to_string(),
            allowed_users: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }

    async fn get_access_token(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!("https://login.microsoftonline.com/{}/oauth2/v2.0/token", self.tenant_id);
        
        let response = client.post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("scope", "https://api.botframework.com/.default"),
            ])
            .send()
            .await?;

        let json: Value = response.json().await?;
        if let Some(token) = json["access_token"].as_str() {
            Ok(token.to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get Teams access token"))
        }
    }
}

#[async_trait]
impl ChannelAdapter for TeamsAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        self.client_id = config.extra.get("client_id").cloned().unwrap_or(String::new());
        self.client_secret = config.extra.get("client_secret").cloned().unwrap_or(String::new());
        self.tenant_id = config.extra.get("tenant_id").cloned().unwrap_or(String::new());
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let url = format!("https://smba.trafficmanager.net/teams/v3/conversations/{}/activities", chat_id);
        
        let body = serde_json::json!({
            "type": "message",
            "text": content,
            "textFormat": "markdown",
        });

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("Teams API error: {}", error_text))
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
            "client_id": self.client_id,
            "tenant_id": self.tenant_id,
        }))
    }

    fn name(&self) -> &str {
        "teams"
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
