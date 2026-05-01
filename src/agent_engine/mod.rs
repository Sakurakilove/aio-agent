use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::Config;
use crate::messaging::{Message, Role};
use crate::tools::{ToolRegistry, ToolResult, WebSearchTool, FileReadTool, FileWriteTool, TerminalTool,
    WebFetchTool, SearchFilesTool, PatchFileTool, ListDirTool, JsonTool, UrlTool,
    TextTool, DateTimeTool, FileInfoTool, MkdirTool, RemoveTool, CopyTool, MoveTool,
    EnvTool, SystemInfoTool, CalculatorTool, Base64Tool, HashTool, RegexTool,
    BrowserNavigateTool, BrowserScreenshotTool, BrowserClickTool, BrowserFillFormTool,
    BrowserGetContentTool, BrowserEvaluateJsTool,
};
use crate::memory::MemoryManager;
use crate::permissions::PermissionChecker;
use crate::budget::{IterationBudget, ToolBudget};
use crate::context::{ContextCompressor, CompressedContext, MessageWithTokens, StreamingContextScrubber};
use crate::errors::{ApiError, ErrorClassifier, RetryPolicy, RetryResult};
use crate::delegation::{DelegationManager, DelegationPolicy, SubAgentRequest};
use crate::providers::{LlmProvider, ChatMessage as LlmChatMessage, MessageRole as LlmMessageRole, ToolDefinition, FunctionDefinition, ToolCall};
use crate::callbacks::{CallbackManager, CallbackEventType};
use crate::guardrails::GuardrailManager;
use crate::human_in_loop::HumanInTheLoop;
use crate::checkpoint::{CheckpointManager, Checkpoint, StateSnapshot};
use crate::handoff::HandoffManager;
use crate::output_parser::OutputParser;
use crate::agents::{Crew, Agent as CrewAgent, Process};
use crate::tasks::Task;
use crate::skills::SkillManager;
use crate::streaming::StreamingLlmProvider;

/// Agent执行结果，包含最终响应、消息历史和统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub final_response: String,
    pub messages: Vec<Message>,
    pub iterations: usize,
    pub context_compressed: bool,
    pub delegation_count: usize,
    pub errors_encountered: usize,
}

/// Agent运行统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub total_iterations: usize,
    pub total_tool_calls: usize,
    pub total_errors: usize,
    pub context_compressions: usize,
    pub delegations_created: usize,
    pub delegations_succeeded: usize,
}

/// AIO Agent核心结构体，集成LLM提供商、工具注册、权限控制、记忆管理、
/// 预算控制、上下文压缩、委派管理、回调系统、Guardrails、人机协作和检查点等功能
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
    pub guardrails: GuardrailManager,
    pub human_in_loop: HumanInTheLoop,
    pub checkpoint_manager: Option<CheckpointManager>,
    pub handoff_manager: Option<HandoffManager>,
    pub output_parser: OutputParser,
    pub skill_manager: Option<SkillManager>,
}

impl AioAgent {
    /// 创建新的AioAgent实例，初始化所有子系统
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
        
        let enabled_tools: std::collections::HashSet<String> = config.tools.enabled.iter().cloned().collect();
        let disabled_tools: std::collections::HashSet<String> = config.tools.disabled.iter().cloned().collect();

        let all_tools: Vec<Arc<dyn crate::tools::Tool>> = vec![
            Arc::new(WebSearchTool), Arc::new(FileReadTool), Arc::new(FileWriteTool),
            Arc::new(TerminalTool), Arc::new(WebFetchTool), Arc::new(SearchFilesTool),
            Arc::new(PatchFileTool), Arc::new(ListDirTool), Arc::new(JsonTool),
            Arc::new(UrlTool), Arc::new(TextTool), Arc::new(DateTimeTool),
            Arc::new(FileInfoTool), Arc::new(MkdirTool), Arc::new(RemoveTool),
            Arc::new(CopyTool), Arc::new(MoveTool), Arc::new(EnvTool),
            Arc::new(SystemInfoTool), Arc::new(CalculatorTool), Arc::new(Base64Tool),
            Arc::new(HashTool), Arc::new(RegexTool),
            Arc::new(BrowserNavigateTool), Arc::new(BrowserScreenshotTool),
            Arc::new(BrowserClickTool), Arc::new(BrowserFillFormTool),
            Arc::new(BrowserGetContentTool), Arc::new(BrowserEvaluateJsTool),
        ];

        for tool in all_tools {
            let name = tool.name().to_string();
            if disabled_tools.contains(&name) {
                continue;
            }
            if !enabled_tools.is_empty() && !enabled_tools.contains(&name) {
                continue;
            }
            tools.register(tool);
        }

        let permissions = PermissionChecker::new(
            config.permissions.allow.clone(),
            config.permissions.deny.clone(),
        );

        let max_iterations = config.agent.max_iterations;
        let timeout = config.agent.timeout_seconds;

        let checkpoint_manager = CheckpointManager::new(&db_path).ok();

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
            guardrails: GuardrailManager::with_defaults(),
            human_in_loop: HumanInTheLoop::console(true),
            checkpoint_manager,
            handoff_manager: None,
            output_parser: OutputParser,
            skill_manager: SkillManager::new().ok(),
        })
    }

    /// 添加一条消息到消息历史
    pub fn add_message(&mut self, role: Role, content: String) {
        let message = Message::new(role, content);
        self.messages.push(message);
    }

    /// 获取Agent运行统计信息
    pub fn get_stats(&self) -> &AgentStats {
        &self.stats
    }

    /// 获取上下文信息摘要，包括消息数、预算使用和委派状态
    pub fn get_context_info(&self) -> String {
        let mut info = String::new();
        info.push_str(&format!("消息数: {}\n", self.messages.len()));
        info.push_str(&format!("迭代预算: {}/{}\n", self.iteration_budget.used(), self.iteration_budget.used() + self.iteration_budget.remaining()));
        info.push_str(&format!("工具预算: {}/{}\n", self.tool_budget.current_executions.load(std::sync::atomic::Ordering::SeqCst), self.tool_budget.max_executions));
        info.push_str(&format!("委派数: {}\n", self.delegation_manager.active_count()));
        if let Some(ctx) = &self.compressed_context {
            info.push_str(&format!("上下文压缩: {} -> {} 字符\n", ctx.original_length, ctx.compressed_length));
        }
        info
    }

    /// 将任务委派给子Agent执行，子Agent通过LLM独立完成任务
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
        }).collect();

        let tools = self.build_tool_definitions();

        let request = crate::providers::ChatCompletionRequest {
            model: self.llm_provider.default_model.clone(),
            messages: llm_messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(false),
            tools: if tools.is_empty() { None } else { Some(tools) },
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

    /// 当消息总长度超过阈值时压缩上下文，减少token消耗
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

    /// 切换LLM提供商，更新活跃provider配置并触发回调事件
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

    /// 初始化HandoffManager并注册默认Agent
    pub fn enable_handoff(&mut self) {
        let mut handoff_mgr = HandoffManager::new(
            LlmProvider::new(
                &self.llm_provider.api_key,
                &self.llm_provider.base_url,
                &self.llm_provider.default_model,
            ),
            "default",
        );
        handoff_mgr.register_default_agents();
        self.handoff_manager = Some(handoff_mgr);
    }

    /// 将当前对话转交给指定Agent
    pub async fn handoff_to(&mut self, target_agent: &str, reason: &str) -> Result<crate::handoff::HandoffResult> {
        if let Some(ref mut handoff_mgr) = self.handoff_manager {
            let conversation_summary: String = self.messages.iter()
                .filter(|m| m.role == Role::User || m.role == Role::Assistant)
                .map(|m| m.content.chars().take(200).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n");

            let mut context = std::collections::HashMap::new();
            context.insert("session_id".to_string(), serde_json::json!(self.session_id));
            context.insert("messages_count".to_string(), serde_json::json!(self.messages.len()));

            let request = crate::handoff::HandoffRequest {
                from_agent: handoff_mgr.current_agent.clone(),
                to_agent: target_agent.to_string(),
                reason: reason.to_string(),
                context,
                conversation_summary,
            };

            let result = handoff_mgr.execute_handoff(request).await?;

            self.callbacks.emit(
                CallbackEventType::AgentEnd,
                &self.session_id,
                serde_json::json!({"handoff_to": target_agent, "accepted": result.accepted}),
            );

            Ok(result)
        } else {
            Err(anyhow::anyhow!("Handoff manager not enabled. Call enable_handoff() first."))
        }
    }

    /// 列出可用的Handoff Agent
    pub fn list_handoff_agents(&self) -> Vec<&crate::handoff::AgentDefinition> {
        self.handoff_manager.as_ref()
            .map(|m| m.list_agents())
            .unwrap_or_default()
    }

    /// 使用OutputParser解析LLM输出为结构化格式
    pub fn parse_output(&self, content: &str) -> crate::output_parser::ParsedOutput {
        self.output_parser.parse(content)
    }

    /// 创建并执行Crew多Agent协作任务
    pub async fn run_crew(
        &mut self,
        agents: Vec<CrewAgent>,
        tasks: Vec<Task>,
        process: Process,
    ) -> Result<std::collections::HashMap<String, String>> {
        let crew = Crew::new(agents, tasks, process)
            .with_llm(LlmProvider::new(
                &self.llm_provider.api_key,
                &self.llm_provider.base_url,
                &self.llm_provider.default_model,
            ));

        self.callbacks.emit(
            CallbackEventType::AgentStart,
            &self.session_id,
            serde_json::json!({"action": "crew_kickoff"}),
        );

        let results = crew.kickoff().await?;

        self.callbacks.emit(
            CallbackEventType::AgentEnd,
            &self.session_id,
            serde_json::json!({"action": "crew_completed", "tasks_completed": results.len()}),
        );

        Ok(results)
    }

    /// 列出已注册的Skills
    pub fn list_skills(&self) -> Vec<String> {
        self.skill_manager.as_ref()
            .map(|sm| sm.list_skills().iter().map(|s| s.metadata.name.clone()).collect())
            .unwrap_or_default()
    }

    /// 搜索匹配的Skills
    pub fn search_skills(&self, query: &str) -> Vec<String> {
        self.skill_manager.as_ref()
            .map(|sm| sm.search_skills(query).iter().map(|s| s.metadata.name.clone()).collect())
            .unwrap_or_default()
    }

    /// 使用流式模式与LLM对话，实时输出token
    pub async fn stream_conversation(&self, user_message: &str) -> Result<String> {
        let provider = StreamingLlmProvider::new(
            &self.llm_provider.api_key,
            &self.llm_provider.base_url,
            &self.llm_provider.default_model,
        );

        let mut messages = vec![
            crate::streaming::StreamingMessage::system(&self.config.agent.system_prompt),
        ];

        for msg in &self.messages {
            match msg.role {
                Role::User => messages.push(crate::streaming::StreamingMessage::user(&msg.content)),
                Role::Assistant => messages.push(crate::streaming::StreamingMessage::assistant(&msg.content)),
                _ => {}
            }
        }

        messages.push(crate::streaming::StreamingMessage::user(user_message));

        let stream = provider.stream_chat_simple(messages).await?;
        let response = crate::streaming::print_stream(stream).await?;

        Ok(response)
    }

    /// 从持久化存储加载指定会话，恢复消息历史（含tool_calls和tool_call_id）
    pub fn load_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.memory.load_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", session_id))?;

        self.messages.clear();
        self.session_id = session_id.to_string();

        for msg_value in &session.messages {
            if let Some(role_str) = msg_value.get("role").and_then(|r| r.as_str()) {
                let role = match role_str {
                    "System" => Role::System,
                    "User" => Role::User,
                    "Assistant" => Role::Assistant,
                    "Tool" => Role::Tool,
                    _ => Role::User,
                };

                let content = msg_value.get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                let tool_calls = msg_value.get("tool_calls").and_then(|tc| {
                    serde_json::from_value::<Vec<crate::messaging::ToolCall>>(tc.clone()).ok()
                });

                let tool_call_id = msg_value.get("tool_call_id")
                    .and_then(|id| id.as_str())
                    .map(|s| s.to_string());

                let tool_result = msg_value.get("tool_result").cloned();

                let mut msg = Message::new(role, content);
                msg.tool_calls = tool_calls;
                msg.tool_call_id = tool_call_id;
                msg.tool_result = tool_result;
                self.messages.push(msg);
            }
        }

        Ok(())
    }

    /// 将内部消息格式转换为LLM API消息格式，保留tool_calls和tool_call_id
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

    /// 从工具注册表构建OpenAI格式的工具定义列表
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

    /// 核心对话循环：接收用户消息，通过ReAct模式（LLM→工具调用→执行→反馈→继续）
    /// 与LLM交互，支持Guardrails验证、HITL审批、检查点保存和回调事件
    pub async fn run_conversation(&mut self, user_message: &str) -> Result<AgentResult> {
        if self.messages.is_empty() {
            self.add_message(Role::System, self.config.agent.system_prompt.clone());
        }

        self.add_message(Role::User, user_message.to_string());

        let guardrail_result = self.guardrails.validate_input(user_message);
        if !guardrail_result.passed {
            return Ok(AgentResult {
                final_response: format!("输入被Guardrails拦截: {}", guardrail_result.message),
                messages: self.messages.clone(),
                iterations: 0,
                context_compressed: false,
                delegation_count: 0,
                errors_encountered: 0,
            });
        }

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

                                let mut effective_args = tool_args.clone();

                                if self.permissions.check("execute", tool_name) {
                                    if self.human_in_loop.needs_approval(tool_name) {
                                        let approval_request = crate::human_in_loop::ApprovalRequest {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            action_type: tool_name.to_string(),
                                            description: format!("执行工具 '{}' 并传递参数", tool_name),
                                            details: tool_args.clone(),
                                            risk_level: match tool_name.as_str() {
                                                "terminal" => crate::human_in_loop::RiskLevel::High,
                                                "file_write" | "remove" | "move" => crate::human_in_loop::RiskLevel::Medium,
                                                _ => crate::human_in_loop::RiskLevel::Low,
                                            },
                                        };

                                        let approval = self.human_in_loop.request_approval(approval_request);
                                        match approval {
                                            crate::human_in_loop::HumanApproval::Rejected(reason) => {
                                                let error_data = serde_json::json!({"error": format!("用户拒绝执行: {}", reason)});
                                                let tool_msg = Message::tool_result(
                                                    tool_call_id,
                                                    format!("用户拒绝执行工具 '{}': {}", tool_name, reason),
                                                    error_data,
                                                );
                                                self.messages.push(tool_msg);
                                                continue;
                                            }
                                            crate::human_in_loop::HumanApproval::Modified(modified) => {
                                                if let Ok(modified_args) = serde_json::from_str(&modified) {
                                                    effective_args = modified_args;
                                                }
                                            }
                                            crate::human_in_loop::HumanApproval::Approved => {}
                                        }
                                    }

                                    match self.tools.execute(tool_name, effective_args).await {
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
            .map(|m| {
                let mut obj = serde_json::json!({
                    "role": format!("{:?}", m.role),
                    "content": m.content,
                });
                if let Some(ref tc) = m.tool_calls {
                    obj["tool_calls"] = serde_json::to_value(tc).unwrap_or_default();
                }
                if let Some(ref id) = m.tool_call_id {
                    obj["tool_call_id"] = serde_json::to_value(id).unwrap_or_default();
                }
                if let Some(ref result) = m.tool_result {
                    obj["tool_result"] = result.clone();
                }
                obj
            })
            .collect();
        self.memory.save_session(&self.session_id, &messages_json)?;

        self.callbacks.emit(
            CallbackEventType::AgentEnd,
            &self.session_id,
            serde_json::json!({"iterations": iterations, "errors": errors_encountered}),
        );

        let output_guardrail = self.guardrails.validate_output(&final_response);
        if !output_guardrail.passed {
            final_response = format!("⚠️ 输出被Guardrails拦截: {}\n\n原始输出已被过滤。", output_guardrail.message);
        }

        if let Some(ref checkpoint_mgr) = self.checkpoint_manager {
            let checkpoint = Checkpoint {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: self.session_id.clone(),
                agent_name: "default".to_string(),
                step: iterations,
                state: serde_json::json!({
                    "messages_count": self.messages.len(),
                    "final_response_length": final_response.len(),
                }),
                messages_summary: final_response.chars().take(200).collect(),
                created_at: chrono::Utc::now().timestamp_millis(),
            };
            let _ = checkpoint_mgr.save_checkpoint(&checkpoint);
        }

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
