use anyhow::Result;
use rusqlite::Connection;

pub struct MemorySearch {
    conn: Connection,
}

impl MemorySearch {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = db_path.replace("~/", &format!("{}/", std::env::var("HOME").unwrap_or_default()));

        if let Some(parent) = std::path::PathBuf::from(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;

        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                content,
                session_id
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(String, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, rank
             FROM memory_fts
             WHERE memory_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;

        let rows = stmt.query_map(rusqlite::params![query, limit], |row| {
            let id: String = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok((id, rank))
        })?;

        rows.collect::<std::result::Result<_, _>>().map_err(|e| anyhow::anyhow!(e))
    }

    pub fn insert_content(&mut self, content: &str, session_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO memory_fts(content, session_id) VALUES (?1, ?2)",
            rusqlite::params![content, session_id],
        )?;
        Ok(())
    }

    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM memory_fts WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }

    pub fn rebuild_index(&mut self) -> Result<()> {
        self.conn.execute("INSERT INTO memory_fts(memory_fts) VALUES ('rebuild')", [])?;
        Ok(())
    }

    pub fn optimize_index(&mut self) -> Result<()> {
        self.conn.execute("INSERT INTO memory_fts(memory_fts) VALUES ('optimize')", [])?;
        Ok(())
    }
}
