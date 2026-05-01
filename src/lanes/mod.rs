use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lane {
    Main,
    Nested,
    Subagent,
}

impl Lane {
    pub fn resolve_nested(lane_id: Option<&str>) -> Self {
        if lane_id.is_some() {
            Lane::Nested
        } else {
            Lane::Main
        }
    }

    pub fn resolve_cron(lane_id: Option<&str>) -> Self {
        if lane_id.is_some() {
            Lane::Nested
        } else {
            Lane::Main
        }
    }

    pub fn is_nested(&self) -> bool {
        matches!(self, Lane::Nested)
    }
}
