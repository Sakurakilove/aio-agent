use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
}

impl Task {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            result: None,
        }
    }
}

pub struct TaskLoop {
    pub goal: String,
    pub max_tasks: usize,
    pub tasks: Vec<Task>,
    pub completed_tasks: Vec<Task>,
}

impl TaskLoop {
    pub fn new(goal: &str, max_tasks: usize) -> Self {
        Self {
            goal: goal.to_string(),
            max_tasks,
            tasks: Vec::new(),
            completed_tasks: Vec::new(),
        }
    }

    pub fn decompose(&mut self) -> Vec<&Task> {
        let mut task_id = 1;
        self.tasks.push(Task::new(
            &format!("{}-{}", self.goal.replace(' ', "-").to_lowercase(), task_id),
            &format!("收集与{}相关的信息", self.goal),
        ));
        task_id += 1;

        self.tasks.push(Task::new(
            &format!("{}-{}", self.goal.replace(' ', "-").to_lowercase(), task_id),
            &format!("分析{}的优缺点", self.goal),
        ));
        task_id += 1;

        self.tasks.push(Task::new(
            &format!("{}-{}", self.goal.replace(' ', "-").to_lowercase(), task_id),
            &format!("总结{}的核心特性", self.goal),
        ));

        self.tasks.iter().collect()
    }

    pub async fn run(&mut self) -> Result<Vec<Task>> {
        if self.tasks.is_empty() {
            self.decompose();
        }

        for task in &mut self.tasks {
            task.status = TaskStatus::InProgress;

            task.result = Some(format!("已完成: {}", task.description));
            task.status = TaskStatus::Completed;

            self.completed_tasks.push(task.clone());

            if self.completed_tasks.len() >= self.max_tasks {
                break;
            }
        }

        Ok(self.completed_tasks.clone())
    }
}
