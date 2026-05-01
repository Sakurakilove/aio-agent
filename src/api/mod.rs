use anyhow::Result;
use axum::{
    extract::State,
    http::Method,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::agent_engine::AioAgent;
use crate::config::Config;

pub struct ServerState {
    pub agent: Mutex<AioAgent>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub iterations: usize,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub model: String,
    pub tools_count: usize,
    pub session_id: String,
    pub active_provider: String,
    pub providers_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct SwitchProviderRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct SwitchProviderResponse {
    pub success: bool,
    pub message: String,
    pub provider_name: String,
}

#[derive(Debug, Serialize)]
pub struct ListProvidersResponse {
    pub providers: Vec<ProviderInfoResponse>,
    pub active: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfoResponse {
    pub name: String,
    pub base_url: String,
    pub default_model: String,
    pub models: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct MemorySessionsResponse {
    pub sessions: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatsResponse {
    pub current_provider: String,
    pub current_model: String,
    pub current_base_url: String,
    pub available_providers: Vec<String>,
    pub tools_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct HandoffRequest {
    pub target_agent: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HandoffResponse {
    pub success: bool,
    pub accepted: bool,
    pub agent_name: String,
    pub response: String,
}

#[derive(Debug, Serialize)]
pub struct CheckpointsResponse {
    pub checkpoints: Vec<CheckpointInfo>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CheckpointInfo {
    pub id: String,
    pub session_id: String,
    pub step: usize,
    pub messages_summary: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct SkillsResponse {
    pub skills: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct CrewRequest {
    pub agents: Vec<CrewAgentRequest>,
    pub tasks: Vec<CrewTaskRequest>,
    #[serde(default = "default_process")]
    pub process: String,
}

fn default_process() -> String {
    "sequential".to_string()
}

#[derive(Debug, Deserialize)]
pub struct CrewAgentRequest {
    pub id: String,
    pub role: String,
    pub goal: String,
    pub backstory: String,
}

#[derive(Debug, Deserialize)]
pub struct CrewTaskRequest {
    pub id: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct CrewResponse {
    pub success: bool,
    pub results: std::collections::HashMap<String, String>,
    pub tasks_completed: usize,
}

pub async fn start_server(config: Config, host: &str, port: u16) -> Result<()> {
    let agent = AioAgent::new(config)?;
    let state = Arc::new(ServerState {
        agent: Mutex::new(agent),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/status", get(get_status))
        .route("/chat", post(chat))
        .route("/tools", get(list_tools))
        .route("/providers", get(list_providers))
        .route("/providers/switch", post(switch_provider))
        .route("/providers/stats", get(get_provider_stats))
        .route("/memory/sessions", get(list_memory_sessions))
        .route("/handoff", post(execute_handoff))
        .route("/handoff/agents", get(list_handoff_agents))
        .route("/checkpoints", get(list_checkpoints))
        .route("/skills", get(list_skills_api))
        .route("/crew", post(execute_crew))
        .with_state(state)
        .layer(cors);

    let addr = format!("{}:{}", host, port);
    println!("HTTP API 服务器启动: http://{}", addr);
    println!("健康检查: http://{}/health", addr);
    println!("API文档: http://{}/docs (待实现)", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "aio-agent"
    }))
}

async fn get_status(State(state): State<Arc<ServerState>>) -> Json<StatusResponse> {
    let agent = state.agent.lock().await;
    Json(StatusResponse {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: agent.llm_provider.default_model.clone(),
        tools_count: agent.tools.list_tools().len(),
        session_id: agent.session_id.clone(),
        active_provider: agent.config.providers.active.clone(),
        providers_count: agent.config.providers.providers.len(),
    })
}

async fn chat(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let mut agent = state.agent.lock().await;
    match agent.run_conversation(&request.message).await {
        Ok(result) => Json(ChatResponse {
            response: result.final_response,
            iterations: result.iterations,
            session_id: agent.session_id.clone(),
        }),
        Err(e) => Json(ChatResponse {
            response: format!("错误: {}", e),
            iterations: 0,
            session_id: agent.session_id.clone(),
        }),
    }
}

async fn list_tools(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let agent = state.agent.lock().await;
    let tools = agent.tools.list_tools();
    Json(serde_json::json!({
        "tools": tools,
        "count": tools.len()
    }))
}

async fn list_providers(State(state): State<Arc<ServerState>>) -> Json<ListProvidersResponse> {
    let agent = state.agent.lock().await;
    let providers: Vec<ProviderInfoResponse> = agent.config.providers.providers.iter().map(|p| {
        ProviderInfoResponse {
            name: p.name.clone(),
            base_url: p.base_url.clone(),
            default_model: p.default_model.clone(),
            models: p.models.clone(),
            enabled: p.enabled,
        }
    }).collect();

    Json(ListProvidersResponse {
        providers,
        active: agent.config.providers.active.clone(),
    })
}

async fn switch_provider(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<SwitchProviderRequest>,
) -> Json<SwitchProviderResponse> {
    let mut agent = state.agent.lock().await;
    match agent.switch_provider(&request.name) {
        Ok(()) => Json(SwitchProviderResponse {
            success: true,
            message: format!("已切换到提供商: {}", request.name),
            provider_name: request.name,
        }),
        Err(e) => Json(SwitchProviderResponse {
            success: false,
            message: format!("切换失败: {}", e),
            provider_name: request.name,
        }),
    }
}

async fn get_provider_stats(State(state): State<Arc<ServerState>>) -> Json<ProviderStatsResponse> {
    let agent = state.agent.lock().await;
    let available_providers: Vec<String> = agent.config.providers.providers.iter()
        .filter(|p| p.enabled)
        .map(|p| p.name.clone())
        .collect();

    Json(ProviderStatsResponse {
        current_provider: agent.config.providers.active.clone(),
        current_model: agent.llm_provider.default_model.clone(),
        current_base_url: agent.llm_provider.base_url.clone(),
        available_providers,
        tools_count: agent.tools.list_tools().len(),
    })
}

async fn list_memory_sessions(State(state): State<Arc<ServerState>>) -> Json<MemorySessionsResponse> {
    let agent = state.agent.lock().await;
    let sessions = agent.memory.list_sessions().unwrap_or_default();
    Json(MemorySessionsResponse {
        count: sessions.len(),
        sessions,
    })
}

async fn execute_handoff(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<HandoffRequest>,
) -> Json<HandoffResponse> {
    let mut agent = state.agent.lock().await;
    if agent.handoff_manager.is_none() {
        agent.enable_handoff();
    }
    let reason = request.reason.unwrap_or_else(|| "API handoff request".to_string());
    match agent.handoff_to(&request.target_agent, &reason).await {
        Ok(result) => Json(HandoffResponse {
            success: true,
            accepted: result.accepted,
            agent_name: result.agent_name,
            response: result.response,
        }),
        Err(e) => Json(HandoffResponse {
            success: false,
            accepted: false,
            agent_name: String::new(),
            response: format!("Handoff error: {}", e),
        }),
    }
}

async fn list_handoff_agents(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let mut agent = state.agent.lock().await;
    if agent.handoff_manager.is_none() {
        agent.enable_handoff();
    }
    let agents: Vec<serde_json::Value> = agent.list_handoff_agents().iter().map(|a| {
            serde_json::json!({
                "name": a.name,
                "tools": a.tools,
                "handoff_targets": a.handoff_targets,
            })
        }).collect();
    Json(serde_json::json!({"agents": agents, "count": agents.len()}))
}

async fn list_checkpoints(State(state): State<Arc<ServerState>>) -> Json<CheckpointsResponse> {
    let agent = state.agent.lock().await;
    if let Some(ref checkpoint_mgr) = agent.checkpoint_manager {
        match checkpoint_mgr.list_checkpoints(&agent.session_id) {
            Ok(checkpoints) => {
                let infos: Vec<CheckpointInfo> = checkpoints.iter().map(|c| CheckpointInfo {
                    id: c.id.clone(),
                    session_id: c.session_id.clone(),
                    step: c.step,
                    messages_summary: c.messages_summary.clone(),
                    created_at: c.created_at,
                }).collect();
                let count = infos.len();
                Json(CheckpointsResponse { checkpoints: infos, count })
            }
            Err(_) => Json(CheckpointsResponse { checkpoints: vec![], count: 0 }),
        }
    } else {
        Json(CheckpointsResponse { checkpoints: vec![], count: 0 })
    }
}

async fn list_skills_api(State(state): State<Arc<ServerState>>) -> Json<SkillsResponse> {
    let agent = state.agent.lock().await;
    let skills = agent.list_skills();
    let count = skills.len();
    Json(SkillsResponse { skills, count })
}

async fn execute_crew(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<CrewRequest>,
) -> Json<CrewResponse> {
    let mut agent = state.agent.lock().await;
    let crew_agents: Vec<crate::agents::Agent> = request.agents.iter().map(|a| {
        crate::agents::Agent::new(&a.id, &a.role, &a.goal, &a.backstory)
    }).collect();

    let tasks: Vec<crate::tasks::Task> = request.tasks.iter().map(|t| {
        crate::tasks::Task::new(&t.id, &t.description)
    }).collect();

    let process = match request.process.as_str() {
        "hierarchical" => crate::agents::Process::Hierarchical,
        _ => crate::agents::Process::Sequential,
    };

    match agent.run_crew(crew_agents, tasks, process).await {
        Ok(results) => Json(CrewResponse {
            success: true,
            tasks_completed: results.len(),
            results,
        }),
        Err(e) => Json(CrewResponse {
            success: false,
            results: std::collections::HashMap::new(),
            tasks_completed: 0,
        }),
    }
}
