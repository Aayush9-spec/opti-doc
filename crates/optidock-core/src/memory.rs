use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Option<i64>,
    pub scope: String,
    pub kind: MemoryKind,
    pub title: String,
    pub summary: String,
    pub payload: String,
    pub confidence: f64,
    pub importance: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryKind {
    Working,
    ShortTerm,
    LongTerm,
    Project,
    Research,
    Code,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryKind::Working => write!(f, "working"),
            MemoryKind::ShortTerm => write!(f, "short_term"),
            MemoryKind::LongTerm => write!(f, "long_term"),
            MemoryKind::Project => write!(f, "project"),
            MemoryKind::Research => write!(f, "research"),
            MemoryKind::Code => write!(f, "code"),
        }
    }
}

impl std::str::FromStr for MemoryKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "working" => Ok(MemoryKind::Working),
            "short_term" => Ok(MemoryKind::ShortTerm),
            "long_term" => Ok(MemoryKind::LongTerm),
            "project" => Ok(MemoryKind::Project),
            "research" => Ok(MemoryKind::Research),
            "code" => Ok(MemoryKind::Code),
            other => anyhow::bail!("unsupported memory kind: {other}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalMemoryStore {
    path: PathBuf,
}

impl LocalMemoryStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create memory store directory {}", parent.display())
            })?;
        }

        let connection = Connection::open(&path).context("failed to open memory SQLite database")?;
        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scope TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                payload TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.0,
                importance REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope);
            CREATE INDEX IF NOT EXISTS idx_memories_kind ON memories(kind);
            CREATE INDEX IF NOT EXISTS idx_memories_updated_at ON memories(updated_at);
            "#,
        )?;

        Ok(Self { path })
    }

    pub fn connection(&self) -> Result<Connection> {
        Connection::open(&self.path).context("failed to reopen memory SQLite database")
    }

    pub fn save(&self, entry: &MemoryEntry) -> Result<MemoryEntry> {
        let connection = self.connection()?;
        let now = current_timestamp();
        let stored = MemoryEntry {
            id: entry.id,
            scope: entry.scope.clone(),
            kind: entry.kind,
            title: entry.title.clone(),
            summary: entry.summary.clone(),
            payload: entry.payload.clone(),
            confidence: entry.confidence,
            importance: entry.importance,
            created_at: entry.created_at.max(now),
            updated_at: now,
        };

        let row_id = if let Some(id) = stored.id {
            let changed = connection.execute(
                r#"
                UPDATE memories
                SET scope = ?, kind = ?, title = ?, summary = ?, payload = ?, confidence = ?, importance = ?, updated_at = ?
                WHERE id = ?
                "#,
                params!(
                    stored.scope,
                    stored.kind.to_string(),
                    stored.title,
                    stored.summary,
                    stored.payload,
                    stored.confidence,
                    stored.importance,
                    now,
                    id
                ),
            )?;
            if changed == 0 {
                return Err(anyhow::anyhow!("memory entry {id} was not found"));
            }
            id
        } else {
            connection.execute(
                r#"
                INSERT INTO memories (scope, kind, title, summary, payload, confidence, importance, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params!(
                    stored.scope,
                    stored.kind.to_string(),
                    stored.title,
                    stored.summary,
                    stored.payload,
                    stored.confidence,
                    stored.importance,
                    now,
                    now
                ),
            )?;
            connection.last_insert_rowid()
        };

        Ok(MemoryEntry {
            id: Some(row_id),
            ..stored
        })
    }

    pub fn load(&self, id: i64) -> Result<Option<MemoryEntry>> {
        let connection = self.connection()?;
        let mut stmt = connection.prepare(
            r#"
            SELECT id, scope, kind, title, summary, payload, confidence, importance, created_at, updated_at
            FROM memories
            WHERE id = ?1
            "#,
        )?;

        let row = stmt.query_row(params![id], memory_entry_from_row).optional()?;

        Ok(row)
    }

    pub fn list(&self, scope: Option<&str>, limit: usize) -> Result<Vec<MemoryEntry>> {
        let connection = self.connection()?;
        let query = if let Some(_scope) = scope {
            "SELECT id, scope, kind, title, summary, payload, confidence, importance, created_at, updated_at FROM memories WHERE scope = ?1 ORDER BY updated_at DESC LIMIT ?2"
        } else {
            "SELECT id, scope, kind, title, summary, payload, confidence, importance, created_at, updated_at FROM memories ORDER BY updated_at DESC LIMIT ?1"
        };

        let mut stmt = connection.prepare(query)?;
        let rows = if scope.is_some() {
            stmt.query_map(params![scope, limit], memory_entry_from_row)?
        } else {
            stmt.query_map(params![limit], memory_entry_from_row)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete(&self, id: i64) -> Result<bool> {
        let connection = self.connection()?;
        let deleted = connection.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    pub fn import_json(&self, scope: &str, kind: MemoryKind, title: &str, summary: &str, payload: &str) -> Result<MemoryEntry> {
        let entry = MemoryEntry {
            id: None,
            scope: scope.to_string(),
            kind,
            title: title.to_string(),
            summary: summary.to_string(),
            payload: payload.to_string(),
            confidence: 0.5,
            importance: 0.5,
            created_at: 0,
            updated_at: 0,
        };
        self.save(&entry)
    }
}

fn memory_entry_from_row(row: &Row<'_>) -> rusqlite::Result<MemoryEntry> {
    Ok(MemoryEntry {
        id: row.get(0)?,
        scope: row.get(1)?,
        kind: row.get::<_, String>(2)?.parse().unwrap_or(MemoryKind::Working),
        title: row.get(3)?,
        summary: row.get(4)?,
        payload: row.get(5)?,
        confidence: row.get(6)?,
        importance: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{LocalMemoryStore, MemoryKind};
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn persists_and_retrieves_entries() {
        let mut path = PathBuf::from(env::temp_dir());
        let thread_name = std::thread::current()
            .name()
            .unwrap_or("default")
            .chars()
            .filter(|ch| ch.is_alphanumeric() || *ch == '-')
            .collect::<String>();
        path.push(format!("optidock-memory-test-{}-{thread_name}.db", std::process::id()));
        let store = LocalMemoryStore::new(&path).unwrap();

        let entry = store
            .import_json("demo", MemoryKind::Working, "Context", "task context", "{\"step\": 1}")
            .unwrap();

        let loaded = store.load(entry.id.unwrap()).unwrap().unwrap();
        assert_eq!(loaded.title, "Context");
        assert_eq!(loaded.kind, MemoryKind::Working);
        assert_eq!(loaded.summary, "task context");

        let listed = store.list(Some("demo"), 10).unwrap();
        assert_eq!(listed.len(), 1);

        fs::remove_file(path).unwrap();
    }
}
