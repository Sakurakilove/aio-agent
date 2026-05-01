use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::tools::{ToolResult, ToolRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopStep {
    pub name: String,
    pub tool_name: String,
    pub assigned_to: String,
    pub status: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopStepResult {
    pub step_name: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub struct Sop {
    pub steps: Vec<SopStep>,
}

impl Sop {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, name: &str, tool_name: &str, assigned_to: &str, dependencies: Option<Vec<String>>) {
        let step = SopStep {
            name: name.to_string(),
            tool_name: tool_name.to_string(),
            assigned_to: assigned_to.to_string(),
            status: "pending".to_string(),
            dependencies: dependencies.unwrap_or_default(),
        };
        self.steps.push(step);
    }

    pub async fn execute(&self, context: &HashMap<String, String>) -> Result<HashMap<String, SopStepResult>> {
        let mut results = HashMap::new();

        for step in &self.steps {
            let start = std::time::Instant::now();

            let result = SopStepResult {
                step_name: step.name.clone(),
                success: true,
                output: Some(serde_json::json!({
                    "step": step.name,
                    "tool": step.tool_name,
                    "agent": step.assigned_to,
                    "context": context
                })),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            };

            results.insert(step.name.clone(), result);
        }

        Ok(results)
    }
}
