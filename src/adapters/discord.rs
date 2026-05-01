use super::adapter::{ChannelAdapter, PlatformConfig, PlatformMessage};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

pub struct DiscordAdapter {
    config: Option<PlatformConfig>,
    client: Client,
}

impl DiscordAdapter {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        "https://discord.com/api/v10"
    }

    fn auth_header(&self) -> String {
        let token = &self.config.as_ref().map(|c| &c.token).unwrap();
        format!("Bot {}", token)
    }
}

#[async_trait]
impl ChannelAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> Result<()> {
        let url = format!("{}/channels/{}/messages", self.base_url(), channel_id);
        let body = serde_json::json!({
            "content": content
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Discord API error: {}", error_text);
        }

        Ok(())
    }

    async fn get_updates(&self, offset: Option<u64>) -> Result<Vec<PlatformMessage>> {
        Ok(Vec::new())
    }

    async fn set_webhook(&self, webhook_url: &str) -> Result<()> {
        let config = self.config.as_ref().ok_or_else(|| anyhow::anyhow!("Not initialized"))?;
        
        if let Some(interaction_endpoint) = config.extra.get("interaction_endpoint") {
            let url = format!("{}/applications/@me", self.base_url());
            let body = serde_json::json!({
                "interactions_endpoint_url": interaction_endpoint
            });

            let response = self
                .client
                .patch(&url)
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to set interaction endpoint: {}", error_text);
            }
        }

        Ok(())
    }

    async fn delete_webhook(&self) -> Result<()> {
        Ok(())
    }

    async fn get_me(&self) -> Result<Value> {
        let url = format!("{}/users/@me", self.base_url());
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let json: Value = response.json().await?;
        Ok(json)
    }

    fn is_initialized(&self) -> bool {
        self.config.is_some()
    }

    fn is_user_allowed(&self, _user_id: &str) -> bool {
        true
    }
}
