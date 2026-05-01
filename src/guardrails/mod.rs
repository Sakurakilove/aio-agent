use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Guardrail验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailResult {
    pub passed: bool,
    pub message: String,
    pub action: GuardrailAction,
}

/// Guardrail动作类型：允许、警告、阻止、重写
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardrailAction {
    Allow,
    Warn,
    Block,
    Rewrite(String),
}

pub trait Guardrail: Send + Sync {
    fn name(&self) -> &str;
    fn validate_input(&self, input: &str) -> GuardrailResult;
    fn validate_output(&self, output: &str) -> GuardrailResult;
}

pub struct LengthGuardrail {
    pub max_input_length: usize,
    pub max_output_length: usize,
}

impl LengthGuardrail {
    pub fn new(max_input: usize, max_output: usize) -> Self {
        Self {
            max_input_length: max_input,
            max_output_length: max_output,
        }
    }
}

impl Guardrail for LengthGuardrail {
    fn name(&self) -> &str {
        "length_guardrail"
    }

    fn validate_input(&self, input: &str) -> GuardrailResult {
        if input.len() > self.max_input_length {
            GuardrailResult {
                passed: false,
                message: format!("输入过长: {} > {} 字符", input.len(), self.max_input_length),
                action: GuardrailAction::Block,
            }
        } else {
            GuardrailResult {
                passed: true,
                message: String::new(),
                action: GuardrailAction::Allow,
            }
        }
    }

    fn validate_output(&self, output: &str) -> GuardrailResult {
        if output.len() > self.max_output_length {
            GuardrailResult {
                passed: false,
                message: format!("输出过长: {} > {} 字符", output.len(), self.max_output_length),
                action: GuardrailAction::Warn,
            }
        } else {
            GuardrailResult {
                passed: true,
                message: String::new(),
                action: GuardrailAction::Allow,
            }
        }
    }
}

pub struct KeywordGuardrail {
    pub blocked_keywords: Vec<String>,
    pub warn_keywords: Vec<String>,
}

impl KeywordGuardrail {
    pub fn new(blocked: Vec<&str>, warn: Vec<&str>) -> Self {
        Self {
            blocked_keywords: blocked.iter().map(|s| s.to_lowercase()).collect(),
            warn_keywords: warn.iter().map(|s| s.to_lowercase()).collect(),
        }
    }
}

impl Guardrail for KeywordGuardrail {
    fn name(&self) -> &str {
        "keyword_guardrail"
    }

    fn validate_input(&self, input: &str) -> GuardrailResult {
        let input_lower = input.to_lowercase();

        for keyword in &self.blocked_keywords {
            if input_lower.contains(keyword) {
                return GuardrailResult {
                    passed: false,
                    message: format!("输入包含禁止关键词: {}", keyword),
                    action: GuardrailAction::Block,
                };
            }
        }

        for keyword in &self.warn_keywords {
            if input_lower.contains(keyword) {
                return GuardrailResult {
                    passed: true,
                    message: format!("输入包含警告关键词: {}", keyword),
                    action: GuardrailAction::Warn,
                };
            }
        }

        GuardrailResult {
            passed: true,
            message: String::new(),
            action: GuardrailAction::Allow,
        }
    }

    fn validate_output(&self, output: &str) -> GuardrailResult {
        let output_lower = output.to_lowercase();

        for keyword in &self.blocked_keywords {
            if output_lower.contains(keyword) {
                return GuardrailResult {
                    passed: false,
                    message: format!("输出包含禁止关键词: {}", keyword),
                    action: GuardrailAction::Block,
                };
            }
        }

        GuardrailResult {
            passed: true,
            message: String::new(),
            action: GuardrailAction::Allow,
        }
    }
}

pub struct RegexGuardrail {
    pub name: String,
    pub pattern: String,
    pub block_on_match: bool,
    compiled: regex::Regex,
}

impl RegexGuardrail {
    pub fn new(name: &str, pattern: &str, block_on_match: bool) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            pattern: pattern.to_string(),
            block_on_match,
            compiled: regex::Regex::new(pattern)?,
        })
    }
}

impl Guardrail for RegexGuardrail {
    fn name(&self) -> &str {
        &self.name
    }

    fn validate_input(&self, input: &str) -> GuardrailResult {
        let matches = self.compiled.is_match(input);
        if matches == self.block_on_match {
            GuardrailResult {
                passed: !self.block_on_match,
                message: if self.block_on_match {
                    format!("输入匹配禁止模式: {}", self.pattern)
                } else {
                    format!("输入不匹配要求模式: {}", self.pattern)
                },
                action: GuardrailAction::Block,
            }
        } else {
            GuardrailResult {
                passed: true,
                message: String::new(),
                action: GuardrailAction::Allow,
            }
        }
    }

    fn validate_output(&self, output: &str) -> GuardrailResult {
        let matches = self.compiled.is_match(output);
        if matches == self.block_on_match {
            GuardrailResult {
                passed: !self.block_on_match,
                message: if self.block_on_match {
                    format!("输出匹配禁止模式: {}", self.pattern)
                } else {
                    String::new()
                },
                action: if self.block_on_match { GuardrailAction::Block } else { GuardrailAction::Allow },
            }
        } else {
            GuardrailResult {
                passed: true,
                message: String::new(),
                action: GuardrailAction::Allow,
            }
        }
    }
}

/// Guardrail管理器，管理输入/输出验证规则链
pub struct GuardrailManager {
    pub input_guardrails: Vec<Arc<dyn Guardrail>>,
    pub output_guardrails: Vec<Arc<dyn Guardrail>>,
}

impl GuardrailManager {
    pub fn new() -> Self {
        Self {
            input_guardrails: Vec::new(),
            output_guardrails: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut manager = Self::new();
        manager.add_input_guardrail(Arc::new(LengthGuardrail::new(100000, 50000)));
        manager.add_input_guardrail(Arc::new(KeywordGuardrail::new(
            vec!["rm -rf /", "sudo rm -rf", "format c:", "drop table"],
            vec!["password", "secret", "token", "api_key"],
        )));
        manager
    }

    pub fn add_input_guardrail(&mut self, guardrail: Arc<dyn Guardrail>) {
        self.input_guardrails.push(guardrail);
    }

    pub fn add_output_guardrail(&mut self, guardrail: Arc<dyn Guardrail>) {
        self.output_guardrails.push(guardrail);
    }

    pub fn validate_input(&self, input: &str) -> GuardrailResult {
        let mut final_result = GuardrailResult {
            passed: true,
            message: String::new(),
            action: GuardrailAction::Allow,
        };

        for guardrail in &self.input_guardrails {
            let result = guardrail.validate_input(input);
            match result.action {
                GuardrailAction::Block => return result,
                GuardrailAction::Warn => {
                    if final_result.passed {
                        final_result = result;
                    }
                }
                GuardrailAction::Rewrite(_) => return result,
                GuardrailAction::Allow => {}
            }
        }

        final_result
    }

    pub fn validate_output(&self, output: &str) -> GuardrailResult {
        let mut final_result = GuardrailResult {
            passed: true,
            message: String::new(),
            action: GuardrailAction::Allow,
        };

        for guardrail in &self.output_guardrails {
            let result = guardrail.validate_output(output);
            match result.action {
                GuardrailAction::Block => return result,
                GuardrailAction::Warn => {
                    if final_result.passed {
                        final_result = result;
                    }
                }
                GuardrailAction::Rewrite(_) => return result,
                GuardrailAction::Allow => {}
            }
        }

        final_result
    }
}
