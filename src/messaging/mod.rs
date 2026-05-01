use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
}

impl Message {
    pub fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            tool_calls: None,
            tool_result: None,
            timestamp: Some(Utc::now()),
            metadata: HashMap::new(),
        }
    }

    pub fn system(content: String) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: String) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: String) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn with_tool_calls(content: String, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content,
            tool_calls: Some(tool_calls),
            tool_result: None,
            timestamp: Some(Utc::now()),
            metadata: HashMap::new(),
        }
    }

    pub fn with_tool_result(content: String, result: serde_json::Value) -> Self {
        Self {
            role: Role::Tool,
            content,
            tool_calls: None,
            tool_result: Some(result),
            timestamp: Some(Utc::now()),
            metadata: HashMap::new(),
        }
    }
}
