use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct FeishuAdapter {
    pub app_id: String,
    pub app_secret: String,
    pub domain: String,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl FeishuAdapter {
    pub fn new(app_id: &str, app_secret: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            app_secret: app_secret.to_string(),
            domain: "open.feishu.cn".to_string(),
            allowed_users: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }

    async fn get_tenant_access_token(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!("https://{}/open-apis/auth/v3/tenant_access_token/internal", self.domain);
        
        let body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret,
        });

        let response = client.post(&url)
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if json["code"].as_i64().unwrap_or(-1) == 0 {
            if let Some(token) = json["tenant_access_token"].as_str() {
                return Ok(token.to_string());
            }
        }
        Err(anyhow::anyhow!("Failed to get Feishu tenant access token"))
    }
}

#[async_trait]
impl ChannelAdapter for FeishuAdapter {
    async fn initialize(&mut self, _config: PlatformConfig) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let token = self.get_tenant_access_token().await?;
        let client = reqwest::Client::new();
        let url = format!("https://{}/open-apis/im/v1/messages?receive_id_type=chat_id", self.domain);
        
        let body = serde_json::json!({
            "receive_id": chat_id,
            "msg_type": "post",
            "content": serde_json::to_string(&serde_json::json!({
                "zh_cn": {
                    "content": [[
                        {"tag": "md", "text": content}
                    ]]
                }
            }))?,
        });

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if json["code"].as_i64().unwrap_or(-1) != 0 {
            let msg = json["msg"].as_str().unwrap_or("unknown error");
            return Err(anyhow::anyhow!("Feishu send error: {}", msg));
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
        let token = self.get_tenant_access_token().await?;
        let client = reqwest::Client::new();
        let url = format!("https://{}/open-apis/bot/v3/info", self.domain);
        
        let response = client.get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let json: Value = response.json().await?;
        Ok(json)
    }

    fn name(&self) -> &str {
        "feishu"
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
