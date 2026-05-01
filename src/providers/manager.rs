use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub rate_limit: Option<u32>,
}

impl ProviderInfo {
    pub fn openai(api_key: &str) -> Self {
        Self {
            name: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
            supports_streaming: true,
            supports_tools: true,
            rate_limit: None,
        }
    }

    pub fn anthropic(api_key: &str) -> Self {
        Self {
            name: "anthropic".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            models: vec!["claude-3-opus".to_string(), "claude-3-sonnet".to_string()],
            supports_streaming: true,
            supports_tools: true,
            rate_limit: None,
        }
    }

    pub fn ollama(host: &str) -> Self {
        Self {
            name: "ollama".to_string(),
            base_url: format!("{}/v1", host),
            models: vec!["llama3".to_string(), "mistral".to_string()],
            supports_streaming: true,
            supports_tools: false,
            rate_limit: None,
        }
    }

    pub fn custom(name: &str, base_url: &str, models: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            base_url: base_url.to_string(),
            models,
            supports_streaming: false,
            supports_tools: false,
            rate_limit: None,
        }
    }
}

pub struct ProviderManager {
    providers: Vec<ProviderInfo>,
    active_provider: Option<usize>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            active_provider: None,
        }
    }

    pub fn add_provider(&mut self, provider: ProviderInfo) {
        self.providers.push(provider);
        if self.active_provider.is_none() {
            self.active_provider = Some(self.providers.len() - 1);
        }
    }

    pub fn get_active(&self) -> Option<&ProviderInfo> {
        self.active_provider
            .and_then(|idx| self.providers.get(idx))
    }

    pub fn set_active(&mut self, name: &str) -> bool {
        if let Some(idx) = self.providers.iter().position(|p| p.name == name) {
            self.active_provider = Some(idx);
            true
        } else {
            false
        }
    }

    pub fn list_providers(&self) -> Vec<&ProviderInfo> {
        self.providers.iter().collect()
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ProviderInfo> {
        self.providers.iter().find(|p| p.name == name)
    }

    pub fn failover(&mut self) -> bool {
        if let Some(current) = self.active_provider {
            let next = (current + 1) % self.providers.len();
            self.active_provider = Some(next);
            true
        } else {
            false
        }
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}
