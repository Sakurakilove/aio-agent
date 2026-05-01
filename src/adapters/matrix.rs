use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct MatrixAdapter {
    pub homeserver: String,
    pub access_token: String,
    pub user_id: String,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl MatrixAdapter {
    pub fn new(homeserver: &str, access_token: &str, user_id: &str) -> Self {
        Self {
            homeserver: homeserver.trim_end_matches('/').to_string(),
            access_token: access_token.to_string(),
            user_id: user_id.to_string(),
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
impl ChannelAdapter for MatrixAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        if let Some(url) = config.extra.get("homeserver") {
            self.homeserver = url.trim_end_matches('/').to_string();
        }
        if let Some(uid) = config.extra.get("user_id") {
            self.user_id = uid.clone();
        }
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, room_id: &str, content: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let encoded_room = urlencoding::encode(room_id);
        let url = format!("{}/_matrix/client/v3/rooms/{}/send/m.room.message", 
            self.homeserver, encoded_room);
        
        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": content,
        });

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("Matrix API error: {}", error_text))
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
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
        
        let response = client.get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        let json: Value = response.json().await?;
        Ok(json)
    }

    fn name(&self) -> &str {
        "matrix"
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
