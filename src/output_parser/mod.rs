use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 解析后的输出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedOutput {
    pub raw: String,
    pub parsed: Option<Value>,
    pub format: OutputFormat,
    pub parse_success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Markdown,
    Text,
    KeyValue,
    List,
}

/// 结构化输出解析器，支持JSON/Markdown/KeyValue/List格式
pub struct OutputParser;

impl OutputParser {
    pub fn parse_json(output: &str) -> ParsedOutput {
        let trimmed = Self::extract_json_block(output);
        match serde_json::from_str::<Value>(&trimmed) {
            Ok(value) => ParsedOutput {
                raw: output.to_string(),
                parsed: Some(value),
                format: OutputFormat::Json,
                parse_success: true,
                error: None,
            },
            Err(e) => ParsedOutput {
                raw: output.to_string(),
                parsed: None,
                format: OutputFormat::Json,
                parse_success: false,
                error: Some(format!("JSON解析失败: {}", e)),
            },
        }
    }

    pub fn parse_list(output: &str) -> ParsedOutput {
        let items: Vec<String> = output
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim().trim_start_matches("- ").trim_start_matches("* ").trim_start_matches(|c: char| c.is_numeric()).trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        ParsedOutput {
            raw: output.to_string(),
            parsed: Some(Value::Array(items.iter().map(|s| Value::String(s.clone())).collect())),
            format: OutputFormat::List,
            parse_success: true,
            error: None,
        }
    }

    pub fn parse_key_value(output: &str) -> ParsedOutput {
        let mut map = serde_json::Map::new();
        for line in output.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                map.insert(key, Value::String(value));
            } else if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                map.insert(key, Value::String(value));
            }
        }

        ParsedOutput {
            raw: output.to_string(),
            parsed: Some(Value::Object(map.clone())),
            format: OutputFormat::KeyValue,
            parse_success: !map.is_empty(),
            error: if map.is_empty() { Some("未找到键值对".to_string()) } else { None },
        }
    }

    pub fn parse_markdown_sections(output: &str) -> ParsedOutput {
        let mut sections = serde_json::Map::new();
        let mut current_header = "headerless".to_string();
        let mut current_content = String::new();

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                if !current_content.is_empty() {
                    sections.insert(current_header.clone(), Value::String(current_content.trim().to_string()));
                    current_content = String::new();
                }
                current_header = trimmed.trim_start_matches('#').trim().to_string();
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        if !current_content.is_empty() {
            sections.insert(current_header, Value::String(current_content.trim().to_string()));
        }

        ParsedOutput {
            raw: output.to_string(),
            parsed: Some(Value::Object(sections.clone())),
            format: OutputFormat::Markdown,
            parse_success: !sections.is_empty(),
            error: None,
        }
    }

    pub fn extract_field(parsed: &ParsedOutput, field_path: &str) -> Option<Value> {
        let value = parsed.parsed.as_ref()?;
        let parts: Vec<&str> = field_path.split('.').collect();
        let mut current = value;

        for part in parts {
            if let Some(index_str) = part.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if let Ok(index) = index_str.parse::<usize>() {
                    if let Some(arr) = current.as_array() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                }
            } else if let Some(obj) = current.as_object() {
                current = obj.get(part)?;
            } else if let Some(arr) = current.as_array() {
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index)?;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        Some(current.clone())
    }

    fn extract_json_block(text: &str) -> String {
        if let Some(start) = text.find("```json") {
            if let Some(end) = text[start + 7..].find("```") {
                return text[start + 7..start + 7 + end].trim().to_string();
            }
        }

        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    return text[start..=end].to_string();
                }
            }
        }

        if let Some(start) = text.find('[') {
            if let Some(end) = text.rfind(']') {
                if end > start {
                    return text[start..=end].to_string();
                }
            }
        }

        text.trim().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutputSchema {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

impl StructuredOutputSchema {
    pub fn json_schema(name: &str, description: &str, schema: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            schema,
        }
    }

    pub fn to_tool_definition(&self) -> crate::providers::ToolDefinition {
        crate::providers::ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::providers::FunctionDefinition {
                name: self.name.clone(),
                description: self.description.clone(),
                parameters: self.schema.clone(),
            },
        }
    }
}
