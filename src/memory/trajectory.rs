use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub step_id: String,
    pub action: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<String>,
    pub tool_result: Option<String>,
    pub success: bool,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub id: String,
    pub session_id: String,
    pub steps: Vec<TrajectoryStep>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub success: bool,
    pub total_steps: usize,
    pub successful_steps: usize,
    pub total_duration_ms: u64,
}

pub struct TrajectoryStore {
    conn: Connection,
}

impl TrajectoryStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = db_path.replace("~/", &format!("{}/", std::env::var("HOME").unwrap_or_default()));

        if let Some(parent) = std::path::PathBuf::from(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(&path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS trajectories (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                start_time REAL NOT NULL,
                end_time REAL,
                success INTEGER NOT NULL,
                total_steps INTEGER NOT NULL,
                successful_steps INTEGER NOT NULL,
                total_duration_ms INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS trajectory_steps (
                id TEXT PRIMARY KEY,
                trajectory_id TEXT NOT NULL,
                action TEXT NOT NULL,
                tool_name TEXT,
                tool_args TEXT,
                tool_result TEXT,
                success INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                timestamp REAL NOT NULL,
                metadata TEXT,
                FOREIGN KEY (trajectory_id) REFERENCES trajectories(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trajectory_session ON trajectories(session_id)",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn create_trajectory(&mut self, trajectory_id: &str, session_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis() as f64;

        self.conn.execute(
            "INSERT INTO trajectories (id, session_id, start_time, end_time, success, total_steps, successful_steps, total_duration_ms)
             VALUES (?1, ?2, ?3, NULL, 0, 0, 0, 0)",
            params![trajectory_id, session_id, now],
        )?;

        Ok(())
    }

    pub fn add_step(&mut self, trajectory_id: &str, step: &TrajectoryStep) -> Result<()> {
        let timestamp = step.timestamp.timestamp_millis() as f64;
        let metadata = serde_json::to_string(&step.metadata).unwrap_or_default();

        self.conn.execute(
            "INSERT INTO trajectory_steps (id, trajectory_id, action, tool_name, tool_args, tool_result, success, duration_ms, timestamp, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                step.step_id,
                trajectory_id,
                step.action,
                step.tool_name,
                step.tool_args,
                step.tool_result,
                if step.success { 1 } else { 0 },
                step.duration_ms,
                timestamp,
                metadata
            ],
        )?;

        Ok(())
    }

    pub fn complete_trajectory(&mut self, trajectory_id: &str, success: bool) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis() as f64;

        self.conn.execute(
            "UPDATE trajectories
             SET end_time = ?1,
                 success = ?2,
                 total_steps = (SELECT COUNT(*) FROM trajectory_steps WHERE trajectory_id = ?3),
                 successful_steps = (SELECT COUNT(*) FROM trajectory_steps WHERE trajectory_id = ?3 AND success = 1),
                 total_duration_ms = (SELECT COALESCE(SUM(duration_ms), 0) FROM trajectory_steps WHERE trajectory_id = ?3)
             WHERE id = ?3",
            params![now, if success { 1 } else { 0 }, trajectory_id],
        )?;

        Ok(())
    }

    pub fn get_trajectory(&self, trajectory_id: &str) -> Result<Option<Trajectory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, start_time, end_time, success, total_steps, successful_steps, total_duration_ms
             FROM trajectories WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![trajectory_id])?;

        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let session_id: String = row.get(1)?;
            let start_time: f64 = row.get(2)?;
            let end_time_opt: Option<f64> = row.get(3)?;
            let success: i32 = row.get(4)?;
            let total_steps: usize = row.get(5)?;
            let successful_steps: usize = row.get(6)?;
            let total_duration_ms: u64 = row.get(7)?;

            let start_time = chrono::DateTime::from_timestamp_millis(start_time as i64).unwrap_or_default();
            let end_time = end_time_opt.map(|t| chrono::DateTime::from_timestamp_millis(t as i64).unwrap_or_default());

            let steps = self.get_trajectory_steps(&id)?;

            Ok(Some(Trajectory {
                id,
                session_id,
                steps,
                start_time,
                end_time,
                success: success == 1,
                total_steps,
                successful_steps,
                total_duration_ms,
            }))
        } else {
            Ok(None)
        }
    }

    fn get_trajectory_steps(&self, trajectory_id: &str) -> Result<Vec<TrajectoryStep>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, tool_name, tool_args, tool_result, success, duration_ms, timestamp, metadata
             FROM trajectory_steps WHERE trajectory_id = ?1 ORDER BY timestamp"
        )?;

        let rows = stmt.query_map(params![trajectory_id], |row| {
            let success: i32 = row.get(5)?;
            let metadata_str: String = row.get(8)?;
            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_str).unwrap_or_default();

            Ok(TrajectoryStep {
                step_id: row.get(0)?,
                action: row.get(1)?,
                tool_name: row.get(2)?,
                tool_args: row.get(3)?,
                tool_result: row.get(4)?,
                success: success == 1,
                duration_ms: row.get(6)?,
                timestamp: chrono::DateTime::from_timestamp_millis((row.get::<_, f64>(7)?) as i64).unwrap_or_default(),
                metadata,
            })
        })?;

        rows.collect::<std::result::Result<_, _>>().map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_session_trajectories(&self, session_id: &str) -> Result<Vec<Trajectory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM trajectories WHERE session_id = ?1 ORDER BY start_time DESC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;
        let ids: Vec<String> = rows.collect::<std::result::Result<_, _>>()?;

        let mut trajectories = Vec::new();
        for id in ids {
            if let Some(traj) = self.get_trajectory(&id)? {
                trajectories.push(traj);
            }
        }

        Ok(trajectories)
    }

    pub fn delete_trajectory(&mut self, trajectory_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM trajectory_steps WHERE trajectory_id = ?1",
            params![trajectory_id],
        )?;
        self.conn.execute(
            "DELETE FROM trajectories WHERE id = ?1",
            params![trajectory_id],
        )?;
        Ok(())
    }
}
