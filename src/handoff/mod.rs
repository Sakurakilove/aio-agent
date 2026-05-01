use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::providers::{LlmProvider, ChatMessage, ChatCompletionRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub instructions: String,
    pub tools: Vec<String>,
    pub handoff_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRequest {
    pub from_agent: String,
    pub to_agent: String,
    pub reason: String,
    pub context: HashMap<String, serde_json::Value>,
    pub conversation_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    pub accepted: bool,
    pub agent_name: String,
    pub response: String,
    pub should_handoff_to: Option<String>,
}

pub struct HandoffManager {
    pub agents: HashMap<String, AgentDefinition>,
    pub llm_provider: LlmProvider,
    pub current_agent: String,
    pub handoff_history: Vec<HandoffRequest>,
}

impl HandoffManager {
    pub fn new(llm_provider: LlmProvider, default_agent: &str) -> Self {
        let mut agents = HashMap::new();
        agents.insert(default_agent.to_string(), AgentDefinition {
            name: default_agent.to_string(),
            instructions: "你是一个通用的AI助手，可以帮助用户完成各种任务。如果需要专业帮助，请将对话转交给合适的专家Agent。".to_string(),
            tools: vec!["web_search".to_string(), "file_read".to_string()],
            handoff_targets: vec![],
        });

        Self {
            agents,
            llm_provider,
            current_agent: default_agent.to_string(),
            handoff_history: Vec::new(),
        }
    }

    pub fn register_agent(&mut self, definition: AgentDefinition) {
        self.agents.insert(definition.name.clone(), definition);
    }

    pub fn register_default_agents(&mut self) {
        self.register_agent(AgentDefinition {
            name: "researcher".to_string(),
            instructions: "你是一个专业研究员，擅长搜索和分析信息。你的任务是深入研究用户的问题，提供详尽的分析报告。".to_string(),
            tools: vec!["web_search".to_string(), "web_fetch".to_string(), "search_files".to_string()],
            handoff_targets: vec!["writer".to_string(), "analyst".to_string()],
        });

        self.register_agent(AgentDefinition {
            name: "writer".to_string(),
            instructions: "你是一个专业写作者，擅长撰写清晰、有条理的文档和报告。你的任务是将研究结果整理成高质量的文档。".to_string(),
            tools: vec!["file_write".to_string(), "patch_file".to_string(), "text_tool".to_string()],
            handoff_targets: vec!["reviewer".to_string()],
        });

        self.register_agent(AgentDefinition {
            name: "analyst".to_string(),
            instructions: "你是一个数据分析师，擅长处理和分析数据。你的任务是从数据中提取洞察和结论。".to_string(),
            tools: vec!["json_tool".to_string(), "calculator".to_string(), "regex".to_string()],
            handoff_targets: vec!["writer".to_string()],
        });

        self.register_agent(AgentDefinition {
            name: "coder".to_string(),
            instructions: "你是一个编程专家，擅长编写和调试代码。你的任务是帮助用户解决编程问题。".to_string(),
            tools: vec!["file_read".to_string(), "file_write".to_string(), "patch_file".to_string(), "terminal".to_string()],
            handoff_targets: vec!["reviewer".to_string()],
        });

        self.register_agent(AgentDefinition {
            name: "reviewer".to_string(),
            instructions: "你是一个质量审核员，负责检查工作成果的质量。你的任务是审核并给出改进建议。".to_string(),
            tools: vec!["file_read".to_string(), "search_files".to_string()],
            handoff_targets: vec!["writer".to_string(), "coder".to_string()],
        });

        if let Some(default) = self.agents.get_mut("default") {
            default.handoff_targets = self.agents.keys().cloned().filter(|k| k != "default").collect();
        }
    }

    pub fn get_current_agent(&self) -> &AgentDefinition {
        self.agents.get(&self.current_agent).unwrap_or_else(|| {
            self.agents.values().next().expect("至少有一个Agent")
        })
    }

    pub fn can_handoff_to(&self, agent_name: &str) -> bool {
        let current = self.get_current_agent();
        current.handoff_targets.contains(&agent_name.to_string()) || current.name == "default"
    }

    pub async fn execute_handoff(&mut self, request: HandoffRequest) -> Result<HandoffResult> {
        if !self.can_handoff_to(&request.to_agent) {
            return Ok(HandoffResult {
                accepted: false,
                agent_name: self.current_agent.clone(),
                response: format!("当前Agent '{}' 无法转交给 '{}'", self.current_agent, request.to_agent),
                should_handoff_to: None,
            });
        }

        let target_agent = self.agents.get(&request.to_agent)
            .ok_or_else(|| anyhow::anyhow!("目标Agent '{}' 不存在", request.to_agent))?;

        self.handoff_history.push(request.clone());
        self.current_agent = request.to_agent.clone();

        let system_prompt = format!(
            "你是Agent '{}'。\n\n指令: {}\n\n可用工具: {}\n\n上下文: {}\n\n前一个Agent的总结: {}",
            target_agent.name,
            target_agent.instructions,
            target_agent.tools.join(", "),
            serde_json::to_string(&request.context).unwrap_or_default(),
            request.conversation_summary,
        );

        let messages = vec![
            ChatMessage::system(&system_prompt),
            ChatMessage::user("请确认你已接手此任务，并简要说明你的计划。"),
        ];

        let llm_request = ChatCompletionRequest {
            model: self.llm_provider.default_model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(1024),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let response = self.llm_provider.chat_completion(llm_request).await?;
        let response_text = response.choices.first()
            .map(|c| c.message.content.clone().unwrap_or_default())
            .unwrap_or_default();

        Ok(HandoffResult {
            accepted: true,
            agent_name: self.current_agent.clone(),
            response: response_text,
            should_handoff_to: None,
        })
    }

    pub fn list_agents(&self) -> Vec<&AgentDefinition> {
        self.agents.values().collect()
    }

    pub fn get_handoff_history(&self) -> &[HandoffRequest] {
        &self.handoff_history
    }
}
