use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPermission {
    pub skill_name: String,
    pub allowed_tools: Vec<String>,
    pub denied_tools: Vec<String>,
    pub max_executions: usize,
    pub requires_confirmation: bool,
}

impl SkillPermission {
    pub fn new(skill_name: &str) -> Self {
        Self {
            skill_name: skill_name.to_string(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            max_executions: 100,
            requires_confirmation: false,
        }
    }

    pub fn allow_tool(mut self, tool: &str) -> Self {
        self.allowed_tools.push(tool.to_string());
        self
    }

    pub fn deny_tool(mut self, tool: &str) -> Self {
        self.denied_tools.push(tool.to_string());
        self
    }

    pub fn require_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    pub fn with_max_executions(mut self, max: usize) -> Self {
        self.max_executions = max;
        self
    }

    pub fn is_tool_allowed(&self, tool: &str) -> bool {
        if self.denied_tools.contains(&tool.to_string()) {
            return false;
        }
        if self.allowed_tools.is_empty() {
            true
        } else {
            self.allowed_tools.contains(&tool.to_string())
        }
    }
}

pub struct SkillPermissionManager {
    permissions: HashMap<String, SkillPermission>,
    execution_counts: HashMap<String, usize>,
}

impl SkillPermissionManager {
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            execution_counts: HashMap::new(),
        }
    }

    pub fn register_permission(&mut self, permission: SkillPermission) {
        let name = permission.skill_name.clone();
        self.permissions.insert(name, permission);
    }

    pub fn check_tool_permission(&self, skill_name: &str, tool: &str) -> bool {
        if let Some(perm) = self.permissions.get(skill_name) {
            perm.is_tool_allowed(tool)
        } else {
            true
        }
    }

    pub fn requires_confirmation(&self, skill_name: &str, tool: &str) -> bool {
        if let Some(perm) = self.permissions.get(skill_name) {
            perm.requires_confirmation && perm.is_tool_allowed(tool)
        } else {
            false
        }
    }

    pub fn record_execution(&mut self, skill_name: &str) -> Result<()> {
        let count = self.execution_counts.entry(skill_name.to_string()).or_insert(0);
        *count += 1;

        if let Some(perm) = self.permissions.get(skill_name) {
            if *count > perm.max_executions {
                return Err(anyhow::anyhow!(
                    "Skill '{}' 已达到最大执行次数 ({})",
                    skill_name,
                    perm.max_executions
                ));
            }
        }

        Ok(())
    }

    pub fn get_execution_count(&self, skill_name: &str) -> usize {
        *self.execution_counts.get(skill_name).unwrap_or(&0)
    }

    pub fn reset_execution_counts(&mut self) {
        self.execution_counts.clear();
    }

    pub fn list_permissions(&self) -> Vec<&SkillPermission> {
        self.permissions.values().collect()
    }

    pub fn remove_permission(&mut self, skill_name: &str) -> Option<SkillPermission> {
        self.permissions.remove(skill_name)
    }
}
