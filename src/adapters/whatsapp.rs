use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct WhatsAppAdapter {
    pub bridge_port: u16,
    initialized: bool,
}

impl WhatsAppAdapter {
    pub fn new(bridge_port: u16) -> Self {
        Self {
            bridge_port,
            initialized: false,
        }
    }
}

#[async_trait]
impl ChannelAdapter for WhatsAppAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        if let Some(port) = config.extra.get("bridge_port") {
            self.bridge_port = port.parse().unwrap_or(self.bridge_port);
        }
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/send", self.bridge_port);
        
        let body = serde_json::json!({
            "to": chat_id,
            "text": content,
        });

        let response = client.post(&url)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(anyhow::anyhow!("WhatsApp bridge error: {}", error_text))
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
            "bridge_port": self.bridge_port,
        }))
    }

    fn name(&self) -> &str {
        "whatsapp"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn is_user_allowed(&self, _user_id: &str) -> bool {
        true
    }
}
