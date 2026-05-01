use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::tasks::{Task, TaskStatus};
use crate::providers::{LlmProvider, ChatMessage, ChatCompletionRequest};

#[derive(Debug, Clone, PartialEq)]
pub enum Process {
    Sequential,
    Hierarchical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub role: String,
    pub goal: String,
    pub backstory: String,
}

impl Agent {
    pub fn new(id: &str, role: &str, goal: &str, backstory: &str) -> Self {
        Self {
            id: id.to_string(),
            role: role.to_string(),
            goal: goal.to_string(),
            backstory: backstory.to_string(),
        }
    }

    fn build_system_prompt(&self) -> String {
        format!(
            "你是一个AI Agent，具有以下角色定义：\n\
            角色: {}\n\
            目标: {}\n\
            背景: {}\n\n\
            请始终以这个角色的视角来思考和行动。给出专业、有针对性的回答。",
            self.role, self.goal, self.backstory
        )
    }
}

pub struct Crew {
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub process: Process,
    pub llm_provider: Option<LlmProvider>,
}

impl Crew {
    pub fn new(agents: Vec<Agent>, tasks: Vec<Task>, process: Process) -> Self {
        Self {
            agents,
            tasks,
            process,
            llm_provider: None,
        }
    }

    pub fn with_llm(mut self, provider: LlmProvider) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    pub async fn kickoff(&self) -> Result<HashMap<String, String>> {
        match self.process {
            Process::Sequential => self.execute_sequential().await,
            Process::Hierarchical => self.execute_hierarchical().await,
        }
    }

    async fn execute_sequential(&self) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();
        let mut context = String::new();

        for (i, task) in self.tasks.iter().enumerate() {
            let agent = self.agents.get(i % self.agents.len())
                .or_else(|| self.agents.first())
                .ok_or_else(|| anyhow::anyhow!("没有可用的Agent"))?;

            let output = self.execute_agent_task(agent, &task.description, &context).await?;

            context = format!("{}\n\n--- {} 的输出 ---\n{}", context, agent.role, output);
            results.insert(task.id.clone(), output);
        }

        Ok(results)
    }

    async fn execute_hierarchical(&self) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();
        let manager = self.agents.first()
            .ok_or_else(|| anyhow::anyhow!("没有Manager Agent"))?;

        let mut context = String::new();

        for task in &self.tasks {
            let assignment = self.execute_agent_task(
                manager,
                &format!("请分析以下任务并决定如何分配：\n任务: {}\n\n可用Agent: {}",
                    task.description,
                    self.agents.iter().skip(1).map(|a| format!("{}({})", a.role, a.goal)).collect::<Vec<_>>().join(", ")
                ),
                &context,
            ).await?;

            let worker = self.agents.get(1).or_else(|| self.agents.first()).unwrap();
            let output = self.execute_agent_task(
                worker,
                &format!("Manager分配给你的任务：\n{}\n\n原始任务: {}", assignment, task.description),
                &context,
            ).await?;

            context = format!("{}\n\n--- {} 的输出 ---\n{}", context, worker.role, output);
            results.insert(task.id.clone(), output);
        }

        Ok(results)
    }

    async fn execute_agent_task(&self, agent: &Agent, task: &str, context: &str) -> Result<String> {
        if let Some(provider) = &self.llm_provider {
            let mut messages = vec![ChatMessage::system(&agent.build_system_prompt())];

            if !context.is_empty() {
                messages.push(ChatMessage::user(&format!("之前的上下文:\n{}", context)));
            }

            messages.push(ChatMessage::user(task));

            let request = ChatCompletionRequest {
                model: provider.default_model.clone(),
                messages,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                stream: Some(false),
                tools: None,
                tool_choice: None,
            };

            let response = provider.chat_completion(request).await?;
            Ok(response.choices.first()
                .map(|c| c.message.content.clone().unwrap_or_default())
                .unwrap_or_default())
        } else {
            Ok(format!("Agent '{}' 完成任务: {} (目标: {})", agent.role, task, agent.goal))
        }
    }
}
