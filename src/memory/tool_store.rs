use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    pub id: String,
    pub tool_name: String,
    pub args: String,
    pub result: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub total_executions: usize,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    pub most_used_tool: String,
    pub least_used_tool: String,
}

pub struct ToolExecutionStore {
    conn: Connection,
}

impl ToolExecutionStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = db_path.replace("~/", &format!("{}/", std::env::var("HOME").unwrap_or_else(|_| ".".to_string())));

        if let Some(parent) = std::path::PathBuf::from(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(&path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tool_executions (
                id TEXT PRIMARY KEY,
                tool_name TEXT NOT NULL,
                args TEXT NOT NULL,
                result TEXT NOT NULL,
                success INTEGER NOT NULL,
                execution_time_ms INTEGER NOT NULL,
                timestamp REAL NOT NULL,
                session_id TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_executions_session ON tool_executions(session_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_executions_tool_name ON tool_executions(tool_name)",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn record_execution(&mut self, record: &ToolExecutionRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tool_executions (id, tool_name, args, result, success, execution_time_ms, timestamp, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.tool_name,
                record.args,
                record.result,
                if record.success { 1 } else { 0 },
                record.execution_time_ms,
                record.timestamp.timestamp_millis() as f64,
                record.session_id
            ],
        )?;

        Ok(())
    }

    pub fn get_session_executions(&self, session_id: &str) -> Result<Vec<ToolExecutionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tool_name, args, result, success, execution_time_ms, timestamp, session_id
             FROM tool_executions WHERE session_id = ?1 ORDER BY timestamp"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let success: i32 = row.get(4)?;
            Ok(ToolExecutionRecord {
                id: row.get(0)?,
                tool_name: row.get(1)?,
                args: row.get(2)?,
                result: row.get(3)?,
                success: success == 1,
                execution_time_ms: row.get(5)?,
                timestamp: chrono::DateTime::from_timestamp_millis((row.get::<_, f64>(6)?) as i64).unwrap_or_default(),
                session_id: row.get(7)?,
            })
        })?;

        rows.collect::<std::result::Result<_, _>>().map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_stats(&self, session_id: Option<&str>) -> Result<ToolStats> {
        let query = match session_id {
            Some(sid) => "SELECT COUNT(*), AVG(execution_time_ms),
                         SUM(success) * 100.0 / COUNT(*),
                         tool_name
                         FROM tool_executions WHERE session_id = ?1".to_string(),
            None => "SELECT COUNT(*), AVG(execution_time_ms),
                    SUM(success) * 100.0 / COUNT(*),
                    tool_name
                    FROM tool_executions".to_string(),
        };

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query(params![session_id.unwrap_or("")])?;

        if let Some(row) = rows.next()? {
            Ok(ToolStats {
                total_executions: row.get(0).unwrap_or(0),
                avg_execution_time_ms: row.get(1).unwrap_or(0.0),
                success_rate: row.get(2).unwrap_or(0.0),
                most_used_tool: row.get(3).unwrap_or_default(),
                least_used_tool: String::new(),
            })
        } else {
            Ok(ToolStats {
                total_executions: 0,
                avg_execution_time_ms: 0.0,
                success_rate: 0.0,
                most_used_tool: String::new(),
                least_used_tool: String::new(),
            })
        }
    }

    pub fn cleanup_old_records(&mut self, days_old: i64) -> Result<usize> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days_old)).timestamp_millis() as f64;

        let deleted = self.conn.execute(
            "DELETE FROM tool_executions WHERE timestamp < ?1",
            params![cutoff],
        )?;

        Ok(deleted)
    }
}
