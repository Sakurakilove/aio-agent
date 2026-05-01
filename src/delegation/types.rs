use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentRequest {
    pub task: String,
    pub max_iterations: usize,
    pub allowed_tools: Option<Vec<String>>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub agent_id: String,
    pub task: String,
    pub success: bool,
    pub output: String,
    pub iterations_used: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationPolicy {
    pub max_subagents: usize,
    pub max_iterations_per_subagent: usize,
    pub max_total_iterations: usize,
    pub timeout_seconds: u64,
    pub allow_nested_delegation: bool,
}

impl Default for DelegationPolicy {
    fn default() -> Self {
        Self {
            max_subagents: 5,
            max_iterations_per_subagent: 50,
            max_total_iterations: 90,
            timeout_seconds: 300,
            allow_nested_delegation: false,
        }
    }
}
