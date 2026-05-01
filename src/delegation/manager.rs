use super::types::{DelegationPolicy, SubAgentRequest, SubAgentResult};
use std::collections::HashMap;

pub struct DelegationManager {
    pub policy: DelegationPolicy,
    active_delegations: HashMap<String, SubAgentResult>,
    delegation_count: usize,
}

impl DelegationManager {
    pub fn new(policy: DelegationPolicy) -> Self {
        Self {
            policy,
            active_delegations: HashMap::new(),
            delegation_count: 0,
        }
    }

    pub fn can_delegate(&self) -> bool {
        self.delegation_count < self.policy.max_subagents
    }

    pub fn create_delegation(&mut self, request: SubAgentRequest) -> String {
        let delegation_id = format!("delegation_{}", self.delegation_count);
        self.delegation_count += 1;

        let result = SubAgentResult {
            agent_id: delegation_id.clone(),
            task: request.task.clone(),
            success: false,
            output: String::new(),
            iterations_used: 0,
            error: None,
        };

        self.active_delegations.insert(delegation_id.clone(), result);
        delegation_id
    }

    pub fn complete_delegation(
        &mut self,
        delegation_id: &str,
        success: bool,
        output: String,
        iterations_used: usize,
    ) {
        if let Some(result) = self.active_delegations.get_mut(delegation_id) {
            result.success = success;
            result.output = output;
            result.iterations_used = iterations_used;
        }
    }

    pub fn fail_delegation(&mut self, delegation_id: &str, error: String) {
        if let Some(result) = self.active_delegations.get_mut(delegation_id) {
            result.success = false;
            result.error = Some(error);
        }
    }

    pub fn get_result(&self, delegation_id: &str) -> Option<&SubAgentResult> {
        self.active_delegations.get(delegation_id)
    }

    pub fn list_delegations(&self) -> Vec<&SubAgentResult> {
        self.active_delegations.values().collect()
    }

    pub fn active_count(&self) -> usize {
        self.active_delegations.len()
    }

    pub fn successful_count(&self) -> usize {
        self.active_delegations.values().filter(|r| r.success).count()
    }

    pub fn total_iterations_used(&self) -> usize {
        self.active_delegations.values().map(|r| r.iterations_used).sum()
    }

    pub fn remaining_delegation_slots(&self) -> usize {
        self.policy.max_subagents.saturating_sub(self.delegation_count)
    }
}
