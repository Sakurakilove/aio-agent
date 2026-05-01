use super::adapter::{ChannelAdapter, PlatformConfig, PlatformMessage};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct TelegramAdapter {
    config: Option<PlatformConfig>,
    client: reqwest::Client,
    offset: u64,
}

impl TelegramAdapter {
    pub fn new() -> Self {
        Self {
            config: None,
            client: reqwest::Client::new(),
            offset: 0,
        }
    }

    fn base_url(&self) -> String {
        let token = &self.config.as_ref().map(|c| &c.token).unwrap();
        format!("https://api.telegram.org/bot{}", token)
    }
}

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let url = format!("{}/sendMessage", self.base_url());
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": content,
            "parse_mode": "Markdown",
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Telegram API error: {}", error_text);
        }

        Ok(())
    }

    async fn get_updates(&self, offset: Option<u64>) -> Result<Vec<PlatformMessage>> {
        let url = format!("{}/getUpdates", self.base_url());
        let timeout = self
            .config
            .as_ref()
            .and_then(|c| c.extra.get("timeout"))
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(30);

        let limit = self
            .config
            .as_ref()
            .and_then(|c| c.extra.get("limit"))
            .and_then(|l| l.parse::<u32>().ok())
            .unwrap_or(100);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("offset", offset.unwrap_or(self.offset).to_string()),
                ("limit", limit.to_string()),
                ("timeout", timeout.to_string()),
            ])
            .send()
            .await?;

        let json: Value = response.json().await?;

        if json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let results = json["result"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|update| {
                            let message = update.get("message")?;
                            let chat = message.get("chat")?;
                            let from = message.get("from")?;

                            Some(PlatformMessage {
                                id: update["update_id"].to_string(),
                                chat_id: chat["id"].to_string(),
                                sender_id: from["id"].to_string(),
                                content: message["text"].as_str().unwrap_or("").to_string(),
                                timestamp: chrono::Utc::now(),
                                raw: update.clone(),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or(Vec::new());

            Ok(results)
        } else {
            anyhow::bail!("Telegram API returned error")
        }
    }

    async fn set_webhook(&self, webhook_url: &str) -> Result<()> {
        let url = format!("{}/setWebhook", self.base_url());
        let body = serde_json::json!({
            "url": webhook_url,
            "allowed_updates": ["message"]
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to set webhook: {}", error_text);
        }

        Ok(())
    }

    async fn delete_webhook(&self) -> Result<()> {
        let url = format!("{}/deleteWebhook", self.base_url());
        let response = self.client.post(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to delete webhook: {}", error_text);
        }

        Ok(())
    }

    async fn get_me(&self) -> Result<Value> {
        let url = format!("{}/getMe", self.base_url());
        let response = self.client.get(&url).send().await?;
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
