use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::tasks::{Task, TaskStatus};

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
}

pub struct Crew {
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub process: Process,
}

impl Crew {
    pub fn new(agents: Vec<Agent>, tasks: Vec<Task>, process: Process) -> Self {
        Self {
            agents,
            tasks,
            process,
        }
    }

    pub async fn kickoff(&self) -> Result<HashMap<String, String>> {
        match self.process {
            Process::Sequential => self.execute_sequential().await,
            Process::Hierarchical => self.execute_hierarchical().await,
        }
    }

    async fn execute_sequential(&self) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();

        for task in &self.tasks {
            let assigned_agent = if let Some(agent) = self.agents.first() {
                agent
            } else {
                continue;
            };

            let output = format!(
                "Agent '{}' 完成任务: {} (目标: {})",
                assigned_agent.role, task.description, assigned_agent.goal
            );

            results.insert(task.id.clone(), output);
        }

        Ok(results)
    }

    async fn execute_hierarchical(&self) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();
        let manager_id = "manager";

        for task in &self.tasks {
            let output = format!(
                "Manager将任务 '{}' 分配给Agent执行: {}",
                task.id, task.description
            );

            results.insert(task.id.clone(), output);
        }

        Ok(results)
    }
}
