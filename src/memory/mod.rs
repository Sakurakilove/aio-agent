mod search;
mod prefetch;
mod tool_store;
mod trajectory;

pub use search::MemorySearch;
pub use prefetch::{MemoryPrefetcher, MemorySynchronizer};
pub use tool_store::{ToolExecutionStore, ToolExecutionRecord, ToolStats};
pub use trajectory::TrajectoryStore;

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemory {
    pub id: String,
    pub messages: Vec<serde_json::Value>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, String>,
}

pub struct MemoryManager {
    conn: Connection,
    pub search: MemorySearch,
    pub tool_store: ToolExecutionStore,
    pub trajectory: TrajectoryStore,
    pub prefetcher: MemoryPrefetcher,
    pub synchronizer: MemorySynchronizer,
}

impl MemoryManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = db_path.replace("~/", &format!("{}/", std::env::var("HOME").unwrap_or_default()));

        if let Some(parent) = PathBuf::from(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(&path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                messages TEXT NOT NULL,
                created_at REAL NOT NULL,
                updated_at REAL NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS semantic_memory (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at REAL NOT NULL
            )",
            [],
        )?;

        let search_db = db_path.replace(".db", "_search.db");
        let search = MemorySearch::new(&search_db)?;

        let tool_db = db_path.replace(".db", "_tools.db");
        let tool_store = ToolExecutionStore::new(&tool_db)?;

        let traj_db = db_path.replace(".db", "_trajectory.db");
        let trajectory = TrajectoryStore::new(&traj_db)?;

        let prefetcher = MemoryPrefetcher::new(50);
        let synchronizer = MemorySynchronizer::new();

        Ok(Self {
            conn,
            search,
            tool_store,
            trajectory,
            prefetcher,
            synchronizer,
        })
    }

    pub fn save_session(&mut self, session_id: &str, messages: &[serde_json::Value]) -> Result<()> {
        let now = Utc::now().timestamp_millis() as f64;
        let messages_json = serde_json::to_string(messages)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO sessions (id, messages, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, messages_json, now, now],
        )?;

        self.search.insert_content(&messages_json, session_id)?;

        self.synchronizer.mark_pending_change();

        Ok(())
    }

    pub fn load_session(&self, session_id: &str) -> Result<Option<SessionMemory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, messages, created_at, updated_at FROM sessions WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![session_id])?;

        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let messages_str: String = row.get(1)?;
            let created_at: f64 = row.get(2)?;
            let updated_at: f64 = row.get(3)?;

            let messages: Vec<serde_json::Value> = serde_json::from_str(&messages_str)?;
            let created_at = chrono::DateTime::from_timestamp_millis(created_at as i64).unwrap_or_default();
            let updated_at = chrono::DateTime::from_timestamp_millis(updated_at as i64).unwrap_or_default();

            Ok(Some(SessionMemory {
                id,
                messages,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn add_semantic_memory(&mut self, content: &str, metadata: &HashMap<String, String>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis() as f64;
        let metadata_json = serde_json::to_string(metadata)?;

        self.conn.execute(
            "INSERT INTO semantic_memory (id, content, metadata, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, content, metadata_json, now],
        )?;

        Ok(id)
    }

    pub fn search_semantic_memory(&self, query: &str) -> Result<Vec<SemanticMemory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, metadata FROM semantic_memory WHERE content LIKE ?1"
        )?;

        let rows = stmt.query_map(params![format!("%{}%", query)], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let metadata_str: String = row.get(2)?;
            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_str).unwrap_or_default();

            Ok(SemanticMemory {
                id,
                content,
                embedding: None,
                metadata,
            })
        })?;

        let memories: Vec<SemanticMemory> = rows.collect::<std::result::Result<_, _>>()?;
        Ok(memories)
    }

    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT id FROM sessions ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let ids: Vec<String> = rows.collect::<std::result::Result<_, _>>()?;
        Ok(ids)
    }

    pub fn sync(&mut self) -> Result<()> {
        self.synchronizer.sync()?;
        self.search.optimize_index()?;
        Ok(())
    }

    pub fn cleanup(&mut self, max_age_days: i64) -> Result<usize> {
        let cutoff = (Utc::now() - chrono::Duration::days(max_age_days)).timestamp_millis() as f64;
        let deleted = self.conn.execute(
            "DELETE FROM sessions WHERE updated_at < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }
}
