use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::Config;
use crate::messaging::{Message, Role};
use crate::tools::{ToolRegistry, ToolResult, WebSearchTool, FileReadTool, FileWriteTool, TerminalTool};
use crate::memory::MemoryManager;
use crate::permissions::PermissionChecker;
use crate::budget::{IterationBudget, ToolBudget};
use crate::context::{ContextCompressor, CompressedContext, MessageWithTokens, StreamingContextScrubber};
use crate::errors::{ApiError, ErrorClassifier, RetryPolicy, RetryResult};
use crate::delegation::{DelegationManager, DelegationPolicy, SubAgentRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub final_response: String,
    pub messages: Vec<Message>,
    pub iterations: usize,
    pub context_compressed: bool,
    pub delegation_count: usize,
    pub errors_encountered: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub total_iterations: usize,
    pub total_tool_calls: usize,
    pub total_errors: usize,
    pub context_compressions: usize,
    pub delegations_created: usize,
    pub delegations_succeeded: usize,
}

pub struct AioAgent {
    pub config: Config,
    pub tools: Arc<ToolRegistry>,
    pub permissions: PermissionChecker,
    pub memory: MemoryManager,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub iteration_budget: IterationBudget,
    pub tool_budget: ToolBudget,
    pub context_compressor: ContextCompressor,
    pub context_scrubber: StreamingContextScrubber,
    pub delegation_manager: DelegationManager,
    pub retry_policy: RetryPolicy,
    pub stats: AgentStats,
    pub compressed_context: Option<CompressedContext>,
}

impl AioAgent {
    pub fn new(config: Config) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let db_path = config.memory.path.clone();
        let memory = MemoryManager::new(&db_path)?;

        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(WebSearchTool));
        tools.register(Arc::new(FileReadTool));
        tools.register(Arc::new(FileWriteTool));
        tools.register(Arc::new(TerminalTool));

        let permissions = PermissionChecker::new(
            config.permissions.allow.clone(),
            config.permissions.deny.clone(),
        );

        let max_iterations = config.agent.max_iterations;
        let timeout = config.agent.timeout_seconds;

        Ok(Self {
            config,
            tools: Arc::new(tools),
            permissions,
            memory,
            session_id,
            messages: Vec::new(),
            iteration_budget: IterationBudget::new(max_iterations),
            tool_budget: ToolBudget::new(100, timeout),
            context_compressor: ContextCompressor::new(8000, 4000),
            context_scrubber: StreamingContextScrubber::new(50, 10),
            delegation_manager: DelegationManager::new(DelegationPolicy::default()),
            retry_policy: RetryPolicy::default(),
            stats: AgentStats {
                total_iterations: 0,
                total_tool_calls: 0,
                total_errors: 0,
                context_compressions: 0,
                delegations_created: 0,
                delegations_succeeded: 0,
            },
            compressed_context: None,
        })
    }

    pub fn add_message(&mut self, role: Role, content: String) {
        let message = Message::new(role, content);
        self.messages.push(message);
    }

    pub fn get_stats(&self) -> &AgentStats {
        &self.stats
    }

    pub fn get_context_info(&self) -> String {
        let mut info = String::new();
        info.push_str(&format!("消息数: {}\n", self.messages.len()));
        info.push_str(&format!("迭代预算: {}/{}\n", self.iteration_budget.used(), self.iteration_budget.remaining()));
        info.push_str(&format!("工具预算: {}/{}\n", self.tool_budget.current_executions.load(std::sync::atomic::Ordering::SeqCst), self.tool_budget.max_executions));
        info.push_str(&format!("委派数: {}\n", self.delegation_manager.active_count()));
        if let Some(ctx) = &self.compressed_context {
            info.push_str(&format!("上下文压缩: {} -> {} 字符\n", ctx.original_length, ctx.compressed_length));
        }
        info
    }

    pub async fn delegate_task(&mut self, task: &str, max_iterations: usize) -> Result<String> {
        if !self.delegation_manager.can_delegate() {
            return Err(anyhow::anyhow!("达到最大委派数量"));
        }

        let request = SubAgentRequest {
            task: task.to_string(),
            max_iterations,
            allowed_tools: None,
            context: std::collections::HashMap::new(),
        };

        let delegation_id = self.delegation_manager.create_delegation(request);
        self.stats.delegations_created += 1;

        let mock_output = format!("子Agent完成委派任务: {}", task);
        self.delegation_manager.complete_delegation(&delegation_id, true, mock_output, 3);
        self.stats.delegations_succeeded += 1;

        Ok(delegation_id)
    }

    pub fn compress_context_if_needed(&mut self) {
        let total_length: usize = self.messages.iter().map(|m| m.content.len()).sum();

        if self.context_compressor.should_compress(total_length) {
            let token_messages: Vec<MessageWithTokens> = self.messages
                .iter()
                .map(|m| MessageWithTokens {
                    role: format!("{:?}", m.role),
                    content: m.content.clone(),
                    token_count: m.content.len() / 4,
                    timestamp: 0,
                })
                .collect();

            let compressed = self.context_compressor.compress_messages(&token_messages);
            self.compressed_context = Some(compressed);
            self.stats.context_compressions += 1;
        }
    }

    pub async fn run_conversation(&mut self, user_message: &str) -> Result<AgentResult> {
        self.add_message(Role::User, user_message.to_string());

        let mut iterations = 0;
        let mut errors_encountered = 0;

        while self.iteration_budget.consume() {
            iterations += 1;
            self.stats.total_iterations += 1;

            self.compress_context_if_needed();

            if !self.tool_budget.can_execute() {
                break;
            }

            let tool_name = "web_search";
            let tool_args = serde_json::json!({
                "query": user_message
            });

            self.tool_budget.record_execution();
            self.stats.total_tool_calls += 1;

            if self.permissions.check("execute", tool_name) {
                let result = self.tools.execute(tool_name, tool_args).await;
                match result {
                    Ok(tool_result) => {
                        let response = if tool_result.success {
                            format!("搜索结果: {}", user_message)
                        } else {
                            format!("工具执行失败: {}", tool_result.error.unwrap_or_default())
                        };
                        self.add_message(Role::Assistant, response);
                    }
                    Err(e) => {
                        errors_encountered += 1;
                        self.stats.total_errors += 1;

                        let api_error = ApiError::Unknown { message: e.to_string() };
                        let classified = ErrorClassifier::classify(&api_error);

                        match self.retry_policy.evaluate(iterations, &classified) {
                            RetryResult::Retry { delay } => {
                                tokio::time::sleep(delay).await;
                                continue;
                            }
                            RetryResult::Abort => {
                                self.add_message(Role::Assistant, format!("操作失败: {}", e));
                                break;
                            }
                            RetryResult::Success => unreachable!(),
                        }
                    }
                }
            } else {
                self.add_message(Role::Assistant, format!("权限不足: {}", tool_name));
            }

            if iterations >= 2 {
                break;
            }
        }

        let messages_json: Vec<serde_json::Value> = self.messages.iter()
            .map(|m| serde_json::json!({
                "role": format!("{:?}", m.role),
                "content": m.content,
            }))
            .collect();
        self.memory.save_session(&self.session_id, &messages_json)?;

        let final_response = self.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        Ok(AgentResult {
            final_response,
            messages: self.messages.clone(),
            iterations,
            context_compressed: self.compressed_context.is_some(),
            delegation_count: self.delegation_manager.active_count(),
            errors_encountered,
        })
    }
}
