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
        model: agent.config.agent.model.clone(),
        tools_count: agent.tools.list_tools().len(),
        session_id: agent.session_id.clone(),
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
