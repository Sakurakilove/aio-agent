use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct DingTalkAdapter {
    pub app_key: String,
    pub app_secret: String,
    pub robot_code: String,
    pub webhook_token: Option<String>,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl DingTalkAdapter {
    pub fn new(app_key: &str, app_secret: &str, robot_code: &str) -> Self {
        Self {
            app_key: app_key.to_string(),
            app_secret: app_secret.to_string(),
            robot_code: robot_code.to_string(),
            webhook_token: None,
            allowed_users: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_webhook_token(mut self, token: &str) -> Self {
        self.webhook_token = Some(token.to_string());
        self
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }

    async fn get_access_token(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let url = "https://api.dingtalk.com/v1.0/oauth2/accessToken";
        
        let body = serde_json::json!({
            "appKey": self.app_key,
            "appSecret": self.app_secret,
        });

        let response = client.post(url)
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if let Some(token) = json["accessToken"].as_str() {
            Ok(token.to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get DingTalk access token"))
        }
    }
}

#[async_trait]
impl ChannelAdapter for DingTalkAdapter {
    async fn initialize(&mut self, _config: PlatformConfig) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, conversation_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let url = "https://api.dingtalk.com/v1.0/robot/oToMessages/batchSend";
        
        let body = serde_json::json!({
            "robotCode": self.robot_code,
            "userIds": [conversation_id],
            "msgKey": "sampleText",
            "msgParam": serde_json::to_string(&serde_json::json!({
                "content": content,
            }))?,
        });

        let response = client.post(url)
            .header("x-acs-dingtalk-access-token", &token)
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if json["success"].as_bool().unwrap_or(false) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("DingTalk send error: {}", json["errorMsg"].as_str().unwrap_or("unknown")))
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
            "robotCode": self.robot_code,
        }))
    }

    fn name(&self) -> &str {
        "dingtalk"
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
