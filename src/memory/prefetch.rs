use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchedMemory {
    pub session_id: String,
    pub context: String,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySyncStatus {
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub sync_count: usize,
    pub pending_changes: usize,
    pub conflicts: Vec<String>,
}

pub struct MemoryPrefetcher {
    cache: HashMap<String, PrefetchedMemory>,
    max_cache_size: usize,
}

impl MemoryPrefetcher {
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_cache_size,
        }
    }

    pub fn prefetch(&mut self, session_id: &str, context: &str) {
        if self.cache.len() >= self.max_cache_size {
            if let Some(oldest) = self.cache
                .iter()
                .min_by_key(|(_, v)| v.last_accessed)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest);
            }
        }

        self.cache.insert(session_id.to_string(), PrefetchedMemory {
            session_id: session_id.to_string(),
            context: context.to_string(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
        });
    }

    pub fn get(&self, session_id: &str) -> Option<&PrefetchedMemory> {
        self.cache.get(session_id)
    }

    pub fn touch(&mut self, session_id: &str) {
        if let Some(memory) = self.cache.get_mut(session_id) {
            memory.last_accessed = chrono::Utc::now();
            memory.access_count += 1;
        }
    }

    pub fn is_cached(&self, session_id: &str) -> bool {
        self.cache.contains_key(session_id)
    }

    pub fn clear_expired(&mut self, max_age_seconds: u64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        self.cache.retain(|_, v| v.last_accessed > cutoff);
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn evict(&mut self, session_id: &str) {
        self.cache.remove(session_id);
    }
}

pub struct MemorySynchronizer {
    status: MemorySyncStatus,
}

impl MemorySynchronizer {
    pub fn new() -> Self {
        Self {
            status: MemorySyncStatus {
                last_sync: chrono::Utc::now(),
                sync_count: 0,
                pending_changes: 0,
                conflicts: Vec::new(),
            },
        }
    }

    pub fn sync(&mut self) -> Result<()> {
        self.status.last_sync = chrono::Utc::now();
        self.status.sync_count += 1;
        self.status.pending_changes = 0;
        Ok(())
    }

    pub fn mark_pending_change(&mut self) {
        self.status.pending_changes += 1;
    }

    pub fn add_conflict(&mut self, conflict: &str) {
        self.status.conflicts.push(conflict.to_string());
    }

    pub fn get_status(&self) -> &MemorySyncStatus {
        &self.status
    }

    pub fn has_pending_changes(&self) -> bool {
        self.status.pending_changes > 0
    }

    pub fn clear_conflicts(&mut self) {
        self.status.conflicts.clear();
    }
}
