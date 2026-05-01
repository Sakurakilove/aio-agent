use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM提供商，封装API密钥、基础URL和HTTP客户端
pub struct LlmProvider {
    pub client: Client,
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub request_timeout: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

fn deserialize_nullable_string<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match opt {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(v) => Ok(Some(v.to_string())),
    }
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: MessageRole::System,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

impl LlmProvider {
    /// 对API密钥进行脱敏处理，仅显示前4位和后4位
    pub fn mask_api_key(key: &str) -> String {
        if key.len() <= 8 {
            return "*".repeat(key.len());
        }
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    }

    /// 获取当前提供商的脱敏API密钥
    pub fn masked_api_key(&self) -> String {
        Self::mask_api_key(&self.api_key)
    }

    /// 创建新的LLM提供商实例，默认120秒请求超时
    pub fn new(api_key: &str, base_url: &str, default_model: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            default_model: default_model.to_string(),
            request_timeout: std::time::Duration::from_secs(120),
        }
    }

    /// 从环境变量创建默认配置的LLM提供商
    pub fn default_config() -> Self {
        Self::new(
            &std::env::var("AIO_AGENT_API_KEY").unwrap_or_default(),
            &std::env::var("AIO_AGENT_API_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            &std::env::var("AIO_AGENT_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
        )
    }

    /// 设置自定义请求超时时间
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.request_timeout = timeout;
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| self.client.clone());
        self
    }

    /// 发送Chat Completion请求到LLM API
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("API请求失败: {}", error_text);
        }

        let result: ChatCompletionResponse = response.json().await?;
        Ok(result)
    }

    /// 简单对话接口，发送消息列表并返回文本响应
    pub async fn simple_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request = ChatCompletionRequest {
            model: self.default_model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let response = self.chat_completion(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone().unwrap_or_default())
        } else {
            anyhow::bail!("API响应中没有找到选择")
        }
    }

    /// 获取可用模型列表
    pub async fn list_models(&self) -> Result<ModelListResponse> {
        let url = format!("{}/models", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("获取模型列表失败: {}", error_text);
        }

        let result: ModelListResponse = response.json().await?;
        Ok(result)
    }

    /// 测试API连接是否正常
    pub async fn test_connection(&self) -> Result<String> {
        let messages = vec![
            ChatMessage::system("你是一个测试助手。请简短回复以确认API连接正常。"),
            ChatMessage::user("测试连接，请回复'连接成功'"),
        ];

        let response = self.simple_chat(messages).await?;
        Ok(response)
    }
}
