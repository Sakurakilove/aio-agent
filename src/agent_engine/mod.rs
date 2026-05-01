use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::Config;
use crate::messaging::{Message, Role};
use crate::tools::{ToolRegistry, ToolResult, WebSearchTool, FileReadTool, FileWriteTool, TerminalTool,
    WebFetchTool, SearchFilesTool, PatchFileTool, ListDirTool, JsonTool, UrlTool,
    TextTool, DateTimeTool, FileInfoTool, MkdirTool, RemoveTool, CopyTool, MoveTool,
    EnvTool, SystemInfoTool, CalculatorTool, Base64Tool, HashTool, RegexTool,
};
use crate::memory::MemoryManager;
use crate::permissions::PermissionChecker;
use crate::budget::{IterationBudget, ToolBudget};
use crate::context::{ContextCompressor, CompressedContext, MessageWithTokens, StreamingContextScrubber};
use crate::errors::{ApiError, ErrorClassifier, RetryPolicy, RetryResult};
use crate::delegation::{DelegationManager, DelegationPolicy, SubAgentRequest};
use crate::providers::{LlmProvider, ChatMessage as LlmChatMessage, MessageRole as LlmMessageRole, ToolDefinition, FunctionDefinition, ToolCall};
use crate::callbacks::{CallbackManager, CallbackEventType};

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
    pub llm_provider: LlmProvider,
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
    pub callbacks: CallbackManager,
}

impl AioAgent {
    pub fn new(config: Config) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let db_path = config.memory.path.clone();
        let memory = MemoryManager::new(&db_path)?;

        let active_provider = config.providers.providers.iter()
            .find(|p| p.name == config.providers.active && p.enabled)
            .or_else(|| config.providers.providers.iter().find(|p| p.enabled));

        let llm_provider = if let Some(provider_config) = active_provider {
            LlmProvider::new(
                &provider_config.api_key,
                &provider_config.base_url,
                &provider_config.default_model,
            )
        } else {
            LlmProvider::default_config()
        };

        let mut tools = ToolRegistry::new();
        
        tools.register(Arc::new(WebSearchTool));
        tools.register(Arc::new(FileReadTool));
        tools.register(Arc::new(FileWriteTool));
        tools.register(Arc::new(TerminalTool));
        tools.register(Arc::new(WebFetchTool));
        tools.register(Arc::new(SearchFilesTool));
        tools.register(Arc::new(PatchFileTool));
        tools.register(Arc::new(ListDirTool));
        tools.register(Arc::new(JsonTool));
        tools.register(Arc::new(UrlTool));
        tools.register(Arc::new(TextTool));
        tools.register(Arc::new(DateTimeTool));
        tools.register(Arc::new(FileInfoTool));
        tools.register(Arc::new(MkdirTool));
        tools.register(Arc::new(RemoveTool));
        tools.register(Arc::new(CopyTool));
        tools.register(Arc::new(MoveTool));
        tools.register(Arc::new(EnvTool));
        tools.register(Arc::new(SystemInfoTool));
        tools.register(Arc::new(CalculatorTool));
        tools.register(Arc::new(Base64Tool));
        tools.register(Arc::new(HashTool));
        tools.register(Arc::new(RegexTool));

        let permissions = PermissionChecker::new(
            config.permissions.allow.clone(),
            config.permissions.deny.clone(),
        );

        let max_iterations = config.agent.max_iterations;
        let timeout = config.agent.timeout_seconds;

        Ok(Self {
            config,
            llm_provider,
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
            callbacks: CallbackManager::new(),
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

        let sub_agent_prompt = format!(
            "你是一个子Agent，负责完成以下委派任务。请直接执行任务并给出结果：\n\n任务: {}\n\n请完成这个任务。",
            task
        );

        let mut sub_messages = self.messages.clone();
        sub_messages.push(Message::user(sub_agent_prompt));

        let llm_messages: Vec<LlmChatMessage> = sub_messages.iter().map(|m| {
            let role = match m.role {
                Role::User => LlmMessageRole::User,
                Role::Assistant => LlmMessageRole::Assistant,
                Role::System => LlmMessageRole::System,
                Role::Tool => LlmMessageRole::Tool,
            };
            LlmChatMessage {
                role,
                content: Some(m.content.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }
        }).collect();

        let request = crate::providers::ChatCompletionRequest {
            model: self.llm_provider.default_model.clone(),
            messages: llm_messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let result = match self.llm_provider.chat_completion(request).await {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    choice.message.content.clone().unwrap_or_default()
                } else {
                    "子Agent未返回结果".to_string()
                }
            }
            Err(e) => format!("子Agent执行失败: {}", e),
        };

        let success = !result.contains("失败");
        self.delegation_manager.complete_delegation(&delegation_id, success, result.clone(), 1);
        if success {
            self.stats.delegations_succeeded += 1;
        }

        Ok(result)
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

    pub fn switch_provider(&mut self, name: &str) -> Result<()> {
        let provider_config = self.config.providers.providers.iter()
            .find(|p| p.name == name && p.enabled)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found or disabled", name))?;

        self.llm_provider = LlmProvider::new(
            &provider_config.api_key,
            &provider_config.base_url,
            &provider_config.default_model,
        );

        self.config.providers.active = name.to_string();

        self.callbacks.emit(
            CallbackEventType::ProviderSwitched,
            &self.session_id,
            serde_json::json!({"provider": name}),
        );

        Ok(())
    }

    pub fn load_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.memory.load_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", session_id))?;

        self.messages.clear();
        self.session_id = session_id.to_string();

        for msg_value in &session.messages {
            if let Some(role_str) = msg_value.get("role").and_then(|r| r.as_str()) {
                if let Some(content) = msg_value.get("content").and_then(|c| c.as_str()) {
                    let role = match role_str {
                        "System" => Role::System,
                        "User" => Role::User,
                        "Assistant" => Role::Assistant,
                        "Tool" => Role::Tool,
                        _ => Role::User,
                    };
                    self.messages.push(Message::new(role, content.to_string()));
                }
            }
        }

        Ok(())
    }

    fn build_llm_messages(&self) -> Vec<LlmChatMessage> {
        self.messages.iter().map(|m| {
            let role = match m.role {
                Role::User => LlmMessageRole::User,
                Role::Assistant => LlmMessageRole::Assistant,
                Role::System => LlmMessageRole::System,
                Role::Tool => LlmMessageRole::Tool,
            };

            let tool_calls = m.tool_calls.as_ref().map(|calls| {
                calls.iter().map(|tc| ToolCall {
                    id: tc.id.clone(),
                    call_type: "function".to_string(),
                    function: crate::providers::FunctionCall {
                        name: tc.name.clone(),
                        arguments: serde_json::to_string(&tc.arguments).unwrap_or_default(),
                    },
                }).collect::<Vec<_>>()
            });

            LlmChatMessage {
                role,
                content: Some(m.content.clone()),
                name: None,
                tool_calls,
                tool_call_id: m.tool_call_id.clone(),
            }
        }).collect()
    }

    fn build_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.get_all_schemas().iter().filter_map(|schema| {
            let name = schema.get("name")?.as_str()?.to_string();
            let description = schema.get("description")?.as_str()?.to_string();
            let parameters = schema.get("parameters")?.clone();

            Some(ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name,
                    description,
                    parameters,
                },
            })
        }).collect()
    }

    pub async fn run_conversation(&mut self, user_message: &str) -> Result<AgentResult> {
        if self.messages.is_empty() {
            self.add_message(Role::System, "你是一个有用的AI助手，可以使用工具来帮助用户完成任务。请根据用户的需求选择合适的工具，并在获得结果后给出清晰的回答。".to_string());
        }

        self.add_message(Role::User, user_message.to_string());

        self.callbacks.emit(
            CallbackEventType::AgentStart,
            &self.session_id,
            serde_json::json!({"message": user_message}),
        );

        let mut iterations = 0;
        let mut errors_encountered = 0;
        let mut final_response = String::new();

        while self.iteration_budget.consume() {
            iterations += 1;
            self.stats.total_iterations += 1;

            self.compress_context_if_needed();

            let llm_messages = self.build_llm_messages();
            let tools = self.build_tool_definitions();

            self.callbacks.emit(
                CallbackEventType::LlmStart,
                &self.session_id,
                serde_json::json!({"model": self.llm_provider.default_model, "iteration": iterations}),
            );

            let request = crate::providers::ChatCompletionRequest {
                model: self.llm_provider.default_model.clone(),
                messages: llm_messages,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                stream: Some(false),
                tools: if tools.is_empty() { None } else { Some(tools) },
                tool_choice: None,
            };

            match self.llm_provider.chat_completion(request).await {
                Ok(response) => {
                    self.callbacks.emit(
                        CallbackEventType::LlmEnd,
                        &self.session_id,
                        serde_json::json!({"model": response.model, "usage": response.usage}),
                    );

                    if let Some(choice) = response.choices.first() {
                        let assistant_content = choice.message.content.clone().unwrap_or_default();

                        if let Some(tool_calls) = &choice.message.tool_calls {
                            let messaging_tool_calls: Vec<crate::messaging::ToolCall> = tool_calls.iter().map(|tc| {
                                crate::messaging::ToolCall {
                                    id: tc.id.clone(),
                                    name: tc.function.name.clone(),
                                    arguments: serde_json::from_str(&tc.function.arguments)
                                        .unwrap_or(serde_json::json!({})),
                                }
                            }).collect();

                            let assistant_msg = Message::with_tool_calls(
                                assistant_content.clone(),
                                messaging_tool_calls,
                            );
                            self.messages.push(assistant_msg);

                            for tool_call in tool_calls {
                                let tool_name = &tool_call.function.name;
                                let tool_args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                                    .unwrap_or(serde_json::json!({}));
                                let tool_call_id = tool_call.id.clone();

                                self.tool_budget.record_execution();
                                self.stats.total_tool_calls += 1;

                                self.callbacks.emit(
                                    CallbackEventType::ToolStart,
                                    &self.session_id,
                                    serde_json::json!({"tool": tool_name, "call_id": tool_call_id}),
                                );

                                if self.permissions.check("execute", tool_name) {
                                    match self.tools.execute(tool_name, tool_args).await {
                                        Ok(tool_result) => {
                                            let tool_output = if tool_result.success {
                                                tool_result.data.clone().unwrap_or(serde_json::json!(null))
                                            } else {
                                                serde_json::json!({"error": tool_result.error.unwrap_or_default()})
                                            };

                                            self.callbacks.emit(
                                                CallbackEventType::ToolEnd,
                                                &self.session_id,
                                                serde_json::json!({"tool": tool_name, "success": tool_result.success}),
                                            );

                                            let tool_msg = Message::tool_result(
                                                tool_call_id,
                                                serde_json::to_string(&tool_output).unwrap_or_default(),
                                                tool_output,
                                            );
                                            self.messages.push(tool_msg);
                                        }
                                        Err(e) => {
                                            errors_encountered += 1;
                                            self.stats.total_errors += 1;

                                            self.callbacks.emit(
                                                CallbackEventType::ToolError,
                                                &self.session_id,
                                                serde_json::json!({"tool": tool_name, "error": e.to_string()}),
                                            );

                                            let error_data = serde_json::json!({"error": e.to_string()});
                                            let tool_msg = Message::tool_result(
                                                tool_call_id,
                                                format!("Tool '{}' execution error: {}", tool_name, e),
                                                error_data,
                                            );
                                            self.messages.push(tool_msg);
                                        }
                                    }
                                } else {
                                    let error_data = serde_json::json!({"error": format!("Permission denied for tool: {}", tool_name)});
                                    let tool_msg = Message::tool_result(
                                        tool_call_id,
                                        format!("Permission denied for tool: {}", tool_name),
                                        error_data,
                                    );
                                    self.messages.push(tool_msg);
                                }
                            }

                            continue;
                        }

                        if !assistant_content.is_empty() {
                            self.add_message(Role::Assistant, assistant_content.clone());
                            final_response = assistant_content;
                            break;
                        }
                    }
                }
                Err(e) => {
                    errors_encountered += 1;
                    self.stats.total_errors += 1;

                    self.callbacks.emit(
                        CallbackEventType::LlmError,
                        &self.session_id,
                        serde_json::json!({"error": e.to_string(), "iteration": iterations}),
                    );

                    let api_error = ApiError::Unknown { message: e.to_string() };
                    let classified = ErrorClassifier::classify(&api_error);

                    match self.retry_policy.evaluate(iterations, &classified) {
                        RetryResult::Retry { delay } => {
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                        RetryResult::Abort => {
                            self.add_message(Role::Assistant, format!("LLM API error: {}", e));
                            final_response = format!("LLM API error: {}", e);
                            break;
                        }
                        RetryResult::Success => unreachable!(),
                    }
                }
            }
        }

        if final_response.is_empty() {
            final_response = self.messages.last()
                .map(|m| m.content.clone())
                .unwrap_or_default();
        }

        let messages_json: Vec<serde_json::Value> = self.messages.iter()
            .map(|m| serde_json::json!({
                "role": format!("{:?}", m.role),
                "content": m.content,
            }))
            .collect();
        self.memory.save_session(&self.session_id, &messages_json)?;

        self.callbacks.emit(
            CallbackEventType::AgentEnd,
            &self.session_id,
            serde_json::json!({"iterations": iterations, "errors": errors_encountered}),
        );

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
