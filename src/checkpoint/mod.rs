use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use rusqlite::{Connection, params};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub session_id: String,
    pub agent_name: String,
    pub step: usize,
    pub state: serde_json::Value,
    pub messages_summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub checkpoint_id: String,
    pub session_id: String,
    pub agent_state: HashMap<String, serde_json::Value>,
    pub tool_results: Vec<serde_json::Value>,
    pub iteration: usize,
    pub timestamp: i64,
}

pub struct CheckpointManager {
    conn: Connection,
}

impl CheckpointManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = if db_path.starts_with("~/") {
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                format!("{}{}", home, &db_path[1..])
            } else {
                db_path.to_string()
            }
        } else {
            db_path.to_string()
        };

        if let Some(parent) = PathBuf::from(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                step INTEGER NOT NULL,
                state TEXT NOT NULL,
                messages_summary TEXT,
                created_at REAL NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS state_snapshots (
                checkpoint_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                agent_state TEXT NOT NULL,
                tool_results TEXT NOT NULL,
                iteration INTEGER NOT NULL,
                timestamp REAL NOT NULL,
                FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id)",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let state_json = serde_json::to_string(&checkpoint.state)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO checkpoints (id, session_id, agent_name, step, state, messages_summary, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                checkpoint.id,
                checkpoint.session_id,
                checkpoint.agent_name,
                checkpoint.step,
                state_json,
                checkpoint.messages_summary,
                checkpoint.created_at as f64,
            ],
        )?;
        Ok(())
    }

    pub fn load_checkpoint(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, agent_name, step, state, messages_summary, created_at FROM checkpoints WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![checkpoint_id])?;
        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let session_id: String = row.get(1)?;
            let agent_name: String = row.get(2)?;
            let step: usize = row.get(3)?;
            let state_str: String = row.get(4)?;
            let messages_summary: String = row.get(5)?;
            let created_at: f64 = row.get(6)?;

            Ok(Some(Checkpoint {
                id,
                session_id,
                agent_name,
                step,
                state: serde_json::from_str(&state_str)?,
                messages_summary,
                created_at: created_at as i64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn save_snapshot(&self, snapshot: &StateSnapshot) -> Result<()> {
        let agent_state_json = serde_json::to_string(&snapshot.agent_state)?;
        let tool_results_json = serde_json::to_string(&snapshot.tool_results)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO state_snapshots (checkpoint_id, session_id, agent_state, tool_results, iteration, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot.checkpoint_id,
                snapshot.session_id,
                agent_state_json,
                tool_results_json,
                snapshot.iteration,
                snapshot.timestamp as f64,
            ],
        )?;
        Ok(())
    }

    pub fn load_snapshot(&self, checkpoint_id: &str) -> Result<Option<StateSnapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT checkpoint_id, session_id, agent_state, tool_results, iteration, timestamp FROM state_snapshots WHERE checkpoint_id = ?1"
        )?;

        let mut rows = stmt.query(params![checkpoint_id])?;
        if let Some(row) = rows.next()? {
            let checkpoint_id: String = row.get(0)?;
            let session_id: String = row.get(1)?;
            let agent_state_str: String = row.get(2)?;
            let tool_results_str: String = row.get(3)?;
            let iteration: usize = row.get(4)?;
            let timestamp: f64 = row.get(5)?;

            Ok(Some(StateSnapshot {
                checkpoint_id,
                session_id,
                agent_state: serde_json::from_str(&agent_state_str)?,
                tool_results: serde_json::from_str(&tool_results_str)?,
                iteration,
                timestamp: timestamp as i64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_checkpoints(&self, session_id: &str) -> Result<Vec<Checkpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, agent_name, step, state, messages_summary, created_at FROM checkpoints WHERE session_id = ?1 ORDER BY step ASC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let id: String = row.get(0)?;
            let session_id: String = row.get(1)?;
            let agent_name: String = row.get(2)?;
            let step: usize = row.get(3)?;
            let state_str: String = row.get(4)?;
            let messages_summary: String = row.get(5)?;
            let created_at: f64 = row.get(6)?;

            Ok(Checkpoint {
                id,
                session_id,
                agent_name,
                step,
                state: serde_json::from_str(&state_str).unwrap_or(serde_json::json!(null)),
                messages_summary,
                created_at: created_at as i64,
            })
        })?;

        let checkpoints: Vec<Checkpoint> = rows.collect::<Result<_, _>>()?;
        Ok(checkpoints)
    }

    pub fn get_latest_checkpoint(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self.list_checkpoints(session_id)?;
        Ok(checkpoints.into_iter().last())
    }

    pub fn cleanup_old_checkpoints(&self, session_id: &str, keep_last: usize) -> Result<usize> {
        let checkpoints = self.list_checkpoints(session_id)?;
        if checkpoints.len() <= keep_last {
            return Ok(0);
        }

        let to_delete: Vec<&str> = checkpoints[..checkpoints.len() - keep_last]
            .iter()
            .map(|c| c.id.as_str())
            .collect();

        let mut deleted = 0;
        for id in to_delete {
            self.conn.execute("DELETE FROM state_snapshots WHERE checkpoint_id = ?1", params![id])?;
            self.conn.execute("DELETE FROM checkpoints WHERE id = ?1", params![id])?;
            deleted += 1;
        }

        Ok(deleted)
    }
}
