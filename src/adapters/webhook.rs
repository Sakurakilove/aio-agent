use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use crate::adapters::adapter::{ChannelAdapter, PlatformMessage, PlatformConfig};

pub struct WebhookAdapter {
    pub port: u16,
    pub global_secret: String,
    initialized: bool,
}

impl WebhookAdapter {
    pub fn new(port: u16, global_secret: &str) -> Self {
        Self {
            port,
            global_secret: global_secret.to_string(),
            initialized: false,
        }
    }

    pub fn validate_signature(&self, payload: &[u8], signature: &str, route_secret: Option<&str>) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = route_secret.unwrap_or(&self.global_secret);
        if secret == "INSECURE_NO_AUTH" {
            return true;
        }

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(payload);
        let result = mac.finalize();
        let expected = format!("sha256={:x}", result.into_bytes());
        
        expected == signature
    }
}

#[async_trait]
impl ChannelAdapter for WebhookAdapter {
    async fn initialize(&mut self, config: PlatformConfig) -> Result<()> {
        if let Some(port) = config.extra.get("port") {
            self.port = port.parse().unwrap_or(self.port);
        }
        self.initialized = true;
        Ok(())
    }

    async fn send_message(&self, _chat_id: &str, content: &str) -> Result<()> {
        tracing::info!("[Webhook] Event logged: {}", content);
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
            "port": self.port,
            "secret_configured": !self.global_secret.is_empty(),
        }))
    }

    fn name(&self) -> &str {
        "webhook"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn is_user_allowed(&self, _user_id: &str) -> bool {
        true
    }
}
