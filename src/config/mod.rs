use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn default_model() -> String {
    "gpt-4".to_string()
}

fn default_max_iterations() -> usize {
    90
}

fn default_timeout_seconds() -> u64 {
    1800
}

fn default_memory_path() -> String {
    "~/.aio-agent/memory.db".to_string()
}

fn default_tools_enabled() -> Vec<String> {
    vec![
        "web_search".to_string(),
        "file_read".to_string(),
        "file_write".to_string(),
        "terminal".to_string(),
    ]
}

fn default_tools_disabled() -> Vec<String> {
    vec![]
}

fn default_gateway_port() -> u16 {
    3000
}

fn default_gateway_host() -> String {
    "127.0.0.1".to_string()
}

fn default_max_sessions() -> usize {
    100
}

fn default_permissions_allow() -> Vec<String> {
    vec![
        "read_file(~/.aio-agent/**)".to_string(),
        "write_to_file(~/.aio-agent/**)".to_string(),
    ]
}

fn default_permissions_deny() -> Vec<String> {
    vec!["execute_code(rm -rf /)".to_string()]
}

fn default_api_key() -> String {
    String::new()
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub channels: HashMap<String, ChannelConfig>,
    #[serde(default)]
    pub permissions: PermissionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_api_key")]
    pub api_key: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default)]
    pub provider: String,
    #[serde(default = "default_memory_path")]
    pub path: String,
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    #[serde(default = "default_tools_enabled")]
    pub enabled: Vec<String>,
    #[serde(default = "default_tools_disabled")]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_gateway_host")]
    pub host: String,
    #[serde(default = "default_gateway_port")]
    pub port: u16,
    #[serde(default)]
    pub auth_token: Option<String>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: default_gateway_port(),
            auth_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    #[serde(default = "default_permissions_allow")]
    pub allow: Vec<String>,
    #[serde(default = "default_permissions_deny")]
    pub deny: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            llm: LlmConfig::default(),
            memory: MemoryConfig::default(),
            tools: ToolsConfig::default(),
            channels: HashMap::new(),
            permissions: PermissionConfig::default(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            max_iterations: default_max_iterations(),
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            provider: "sqlite".to_string(),
            path: default_memory_path(),
            max_sessions: default_max_sessions(),
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            enabled: default_tools_enabled(),
            disabled: default_tools_disabled(),
        }
    }
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            allow: default_permissions_allow(),
            deny: default_permissions_deny(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: default_api_key(),
            base_url: default_base_url(),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        if path.ends_with(".json") {
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            let config: Config = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            anyhow::bail!("Unsupported config format: {}", path)
        }
    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = if path.ends_with(".json") {
            serde_json::to_string_pretty(self)?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::to_string(self)?
        } else {
            anyhow::bail!("Unsupported config format: {}", path)
        };

        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }
}
