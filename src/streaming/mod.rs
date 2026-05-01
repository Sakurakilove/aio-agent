use anyhow::Result;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

/// 流式LLM提供商，支持SSE流式响应
pub struct StreamingLlmProvider {
    pub client: Client,
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamingChatRequest {
    pub model: String,
    pub messages: Vec<StreamingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingMessage {
    pub role: String,
    pub content: String,
}

impl StreamingMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamDelta {
    #[serde(default)]
    pub content: Option<String>,
}

impl StreamingLlmProvider {
    pub fn new(api_key: &str, base_url: &str, default_model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            default_model: default_model.to_string(),
        }
    }

    pub fn default_config() -> Self {
        Self::new(
            &std::env::var("AIO_AGENT_API_KEY").unwrap_or_default(),
            &std::env::var("AIO_AGENT_API_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            &std::env::var("AIO_AGENT_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
        )
    }

    /// 发送流式Chat请求，返回SSE事件流
    pub async fn stream_chat(
        &self,
        request: StreamingChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
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
            anyhow::bail!("流式请求失败: {}", error_text);
        }

        let mut stream = response.bytes_stream();

        let output = async_stream::try_stream! {
            let mut buffer = String::new();
            while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
                let bytes = chunk?;
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                if let Some(last_newline) = buffer.rfind('\n') {
                    let complete = buffer[..=last_newline].to_string();
                    buffer = buffer[last_newline + 1..].to_string();

                    for line in complete.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                return;
                            }
                            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                                if let Some(content) = chunk.choices.first().and_then(|c| c.delta.content.clone()) {
                                    yield content;
                                }
                            }
                        }
                    }
                }
            }

            for line in buffer.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        return;
                    }
                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(content) = chunk.choices.first().and_then(|c| c.delta.content.clone()) {
                            yield content;
                        }
                    }
                }
            }
        };

        Ok(Box::pin(output))
    }

    pub async fn stream_chat_simple(
        &self,
        messages: Vec<StreamingMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let request = StreamingChatRequest {
            model: self.default_model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: true,
        };

        self.stream_chat(request).await
    }
}

/// 逐块打印流式响应并返回完整文本
pub async fn print_stream(mut stream: Pin<Box<dyn Stream<Item = Result<String>> + Send>>) -> Result<String> {
    use tokio::io::AsyncWriteExt;
    let mut full_response = String::new();

    let mut stdout = tokio::io::stdout();
    while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
        match chunk {
            Ok(text) => {
                full_response.push_str(&text);
                stdout.write_all(text.as_bytes()).await?;
                stdout.flush().await?;
            }
            Err(e) => eprintln!("\n流式错误: {}", e),
        }
    }
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;

    Ok(full_response)
}
