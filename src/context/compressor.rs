use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedContext {
    pub summary: String,
    pub key_points: Vec<String>,
    pub original_length: usize,
    pub compressed_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageWithTokens {
    pub role: String,
    pub content: String,
    pub token_count: usize,
    pub timestamp: u64,
}

pub struct ContextCompressor {
    pub max_context_length: usize,
    pub summary_threshold: usize,
}

impl ContextCompressor {
    pub fn new(max_context_length: usize, summary_threshold: usize) -> Self {
        Self {
            max_context_length,
            summary_threshold,
        }
    }

    pub fn compress_messages(&self, messages: &[MessageWithTokens]) -> CompressedContext {
        let total_tokens: usize = messages.iter().map(|m| m.token_count).sum();

        if total_tokens <= self.max_context_length {
            return CompressedContext {
                summary: String::new(),
                key_points: Vec::new(),
                original_length: total_tokens,
                compressed_length: total_tokens,
            };
        }

        let recent_messages: Vec<&MessageWithTokens> = messages
            .iter()
            .rev()
            .take(5)
            .collect();

        let old_messages: Vec<&MessageWithTokens> = messages
            .iter()
            .take(messages.len().saturating_sub(5))
            .collect();

        let key_points = self.extract_key_points(&old_messages);
        let summary = self.generate_summary(&old_messages);

        let recent_tokens: usize = recent_messages.iter().map(|m| m.token_count).sum();
        let compressed_length = summary.len() + recent_tokens;

        CompressedContext {
            summary,
            key_points,
            original_length: total_tokens,
            compressed_length,
        }
    }

    fn extract_key_points(&self, messages: &[&MessageWithTokens]) -> Vec<String> {
        messages
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| {
                let content = &m.content;
                if content.len() > 100 {
                    content[..100].to_string()
                } else {
                    content.clone()
                }
            })
            .collect()
    }

    fn generate_summary(&self, messages: &[&MessageWithTokens]) -> String {
        let user_count = messages.iter().filter(|m| m.role == "user").count();
        let assistant_count = messages.iter().filter(|m| m.role == "assistant").count();
        let tool_count = messages.iter().filter(|m| m.role == "tool").count();

        format!(
            "Historical context: {} user messages, {} assistant responses, {} tool calls. ",
            user_count, assistant_count, tool_count
        )
    }

    pub fn should_compress(&self, current_length: usize) -> bool {
        current_length > self.summary_threshold
    }

    pub fn calculate_compression_ratio(&self, original: usize, compressed: usize) -> f64 {
        if original == 0 {
            1.0
        } else {
            compressed as f64 / original as f64
        }
    }
}
