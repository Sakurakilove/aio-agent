use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct WeComAdapter {
    pub corp_id: String,
    pub agent_id: String,
    pub corp_secret: String,
    pub webhook_key: Option<String>,
    pub allowed_users: Vec<String>,
    initialized: bool,
}

impl WeComAdapter {
    pub fn new(corp_id: &str, agent_id: &str, corp_secret: &str) -> Self {
        Self {
            corp_id: corp_id.to_string(),
            agent_id: agent_id.to_string(),
            corp_secret: corp_secret.to_string(),
            webhook_key: None,
            allowed_users: Vec::new(),
            initialized: false,
        }
    }

    pub fn with_webhook_key(mut self, key: &str) -> Self {
        self.webhook_key = Some(key.to_string());
        self
    }

    pub fn with_allowed_users(mut self, users: Vec<String>) -> Self {
        self.allowed_users = users;
        self
    }

    async fn get_access_token(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let url = "https://qyapi.weixin.qq.com/cgi-bin/gettoken";
        
        let response = client.get(url)
            .query(&[
                ("corpid", &self.corp_id),
                ("corpsecret", &self.corp_secret),
            ])
            .send()
            .await?;

        let json: Value = response.json().await?;
        if let Some(token) = json["access_token"].as_str() {
            Ok(token.to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get WeCom access token"))
        }
    }
}

#[async_trait]
impl ChannelAdapter for WeComAdapter {
    async fn initialize(&mut self, _config: PlatformConfig) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, user_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let url = "https://qyapi.weixin.qq.com/cgi-bin/message/send";
        
        let body = serde_json::json!({
            "touser": user_id,
            "agentid": self.agent_id,
            "msgtype": "text",
            "text": {
                "content": content,
            },
        });

        let response = client.post(url)
            .query(&[("access_token", &token)])
            .json(&body)
            .send()
            .await?;

        let json: Value = response.json().await?;
        if json["errcode"].as_i64().unwrap_or(-1) != 0 {
            let errmsg = json["errmsg"].as_str().unwrap_or("unknown error");
            return Err(anyhow::anyhow!("WeCom send error: {}", errmsg));
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
        Ok(serde_json::json!({
            "corpid": self.corp_id,
            "agentid": self.agent_id,
        }))
    }

    fn name(&self) -> &str {
        "wecom"
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
