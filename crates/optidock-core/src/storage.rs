//! Persistent storage layer for OptiDoc.
//!
//! This module provides a shared local-first storage API for conversations,
//! messages, sessions, actions, docker events, artifacts, and search backed by
//! SQLite with WAL mode and JSON files for large/raw payloads.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub home: String,
    pub database: String,
    pub json_storage: String,
    pub wal: bool,
    pub server: ServerConfig,
    pub chat: ChatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub persist_messages: bool,
    pub persist_actions: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let home = default_home();
        Self {
            home: home.to_string_lossy().to_string(),
            database: "data/optidoc.db".to_string(),
            json_storage: "data".to_string(),
            wal: true,
            server: ServerConfig { host: "127.0.0.1".to_string(), port: 8787 },
            chat: ChatConfig { persist_messages: true, persist_actions: true },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageStatus {
    Ok,
    ReadOnly,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEnvelope {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub created_at: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: Option<String>,
    pub working_directory: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub parent_id: Option<String>,
    pub token_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub conversation_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub working_directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub id: String,
    pub conversation_id: Option<String>,
    pub session_id: Option<String>,
    pub action_type: String,
    pub command: Option<String>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerEvent {
    pub id: String,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub event_type: Option<String>,
    pub status: Option<String>,
    pub timestamp: String,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<i64>,
    pub network_rx_bytes: Option<i64>,
    pub network_tx_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub conversation_id: Option<String>,
    pub session_id: Option<String>,
    pub type_: String,
    pub name: Option<String>,
    pub path: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub checksum: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub relevance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSummary {
    pub query: String,
    pub total: usize,
    pub results: Vec<SearchResult>,
}

#[derive(Clone)]
pub struct Storage {
    pub config: StorageConfig,
    connection: Arc<Mutex<Connection>>,
    pub conversations: ConversationStore,
    pub messages: MessageStore,
    pub sessions: SessionStore,
    pub actions: ActionStore,
    pub docker: DockerStore,
    pub artifacts: ArtifactStore,
    pub search: SearchStore,
}

#[derive(Clone)]
pub struct ConversationStore {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct MessageStore {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct SessionStore {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct ActionStore {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct DockerStore {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct ArtifactStore {
    config: StorageConfig,
    connection: Arc<Mutex<Connection>>,
}

#[derive(Clone)]
pub struct SearchStore {
    connection: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let home = path.as_ref().to_path_buf();
        let config = StorageConfig {
            home: home.to_string_lossy().to_string(),
            database: "data/optidoc.db".to_string(),
            json_storage: "data".to_string(),
            wal: true,
            server: ServerConfig { host: "127.0.0.1".to_string(), port: 8787 },
            chat: ChatConfig { persist_messages: true, persist_actions: true },
        };

        ensure_layout(&config)?;
        write_default_config(&config)?;

        let db_path = home.join(&config.database);
        let connection = Connection::open(&db_path)
            .with_context(|| format!("failed to open SQLite database at {}", db_path.display()))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;",
        )?;
        run_migrations(&connection)?;

        let shared = Arc::new(Mutex::new(connection));
        let storage = Self {
            config: config.clone(),
            connection: Arc::clone(&shared),
            conversations: ConversationStore { connection: Arc::clone(&shared) },
            messages: MessageStore { connection: Arc::clone(&shared) },
            sessions: SessionStore { connection: Arc::clone(&shared) },
            actions: ActionStore { connection: Arc::clone(&shared) },
            docker: DockerStore { connection: Arc::clone(&shared) },
            artifacts: ArtifactStore { config: config.clone(), connection: Arc::clone(&shared) },
            search: SearchStore { connection: Arc::clone(&shared) },
        };

        storage.recover_stale_state()?;
        Ok(storage)
    }

    pub fn status(&self) -> StorageStatus {
        match self.connection.lock() {
            Ok(_) => StorageStatus::Ok,
            Err(_) => StorageStatus::Error("database lock poisoned".to_string()),
        }
    }

    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    pub fn search_messages(&self, query: &str) -> Result<Vec<SearchResult>> {
        self.search.query(query)
    }

    pub fn get_conversation_context(&self, conversation_id: &str) -> Result<Value> {
        let conversation = self.conversations.get(conversation_id)?.unwrap_or_else(|| Conversation {
            id: conversation_id.to_string(),
            title: "Unknown".to_string(),
            created_at: utc_timestamp_string(),
            updated_at: utc_timestamp_string(),
            model: None,
            working_directory: None,
            status: "active".to_string(),
            metadata: None,
        });
        let recent_messages = self.messages.list(conversation_id, 8)?;
        let relevant_messages = self.search.query(
            recent_messages
                .iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .as_str(),
        )?;
        let recent_actions = self.actions.list_for_conversation(conversation_id, 8)?;
        let relevant_artifacts = self.artifacts.list_for_conversation(conversation_id, 10)?;

        Ok(serde_json::json!({
            "conversation": conversation,
            "recent_messages": recent_messages,
            "relevant_messages": relevant_messages,
            "recent_actions": recent_actions,
            "relevant_artifacts": relevant_artifacts
        }))
    }

    pub fn append_event_log(&self, event_type: &str, data: Value) -> Result<()> {
        let date = current_date_string();
        let dir = PathBuf::from(&self.config.home).join("data").join("events").join(date);
        fs::create_dir_all(&dir)?;
        let path = dir.join("events.jsonl");
        let envelope = JsonEnvelope {
            id: Uuid::new_v4().to_string(),
            type_: event_type.to_string(),
            created_at: utc_timestamp_string(),
            data,
        };
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&path)?;
        use std::io::Write;
        writeln!(file, "{}", serde_json::to_string(&envelope)?)?;
        file.sync_all()?;
        Ok(())
    }

    fn recover_stale_state(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        let stale = utc_timestamp_string_minus(3600);
        conn.execute(
            "UPDATE sessions SET ended_at = ?1 WHERE ended_at IS NULL AND started_at < ?2",
            params![utc_timestamp_string(), stale],
        )?;
        conn.execute(
            "UPDATE agent_actions SET status = 'interrupted' WHERE status = 'running' AND started_at < ?1",
            params![stale],
        )?;
        Ok(())
    }
}

impl ConversationStore {
    pub fn create(&self, title: &str, metadata: Option<Value>) -> Result<Conversation> {
        let now = utc_timestamp_string();
        let id = Uuid::new_v4().to_string();
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at, model, working_directory, status, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, title, now, now, Option::<String>::None, Option::<String>::None, "active", json_to_string(metadata.clone())],
        )?;

        Ok(Conversation {
            id,
            title: title.to_string(),
            created_at: now.clone(),
            updated_at: now,
            model: None,
            working_directory: None,
            status: "active".to_string(),
            metadata,
        })
    }

    pub fn get(&self, conversation_id: &str) -> Result<Option<Conversation>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at, model, working_directory, status, metadata_json FROM conversations WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![conversation_id], conversation_row_to_struct).optional()?;
        Ok(row)
    }

    pub fn list(&self, status: Option<&str>, limit: usize) -> Result<Vec<Conversation>> {
        let conn = self.connection.lock().unwrap();
        let query = if status.is_some() {
            "SELECT id, title, created_at, updated_at, model, working_directory, status, metadata_json FROM conversations WHERE status = ?1 ORDER BY updated_at DESC LIMIT ?2"
        } else {
            "SELECT id, title, created_at, updated_at, model, working_directory, status, metadata_json FROM conversations ORDER BY updated_at DESC LIMIT ?1"
        };

        let mut stmt = conn.prepare(query)?;
        let rows = if status.is_some() {
            stmt.query_map(params![status, limit], conversation_row_to_struct)?
        } else {
            stmt.query_map(params![limit], conversation_row_to_struct)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update(&self, conversation_id: &str, title: Option<&str>, status: Option<&str>, metadata: Option<Value>) -> Result<Conversation> {
        let existing = self.get(conversation_id)?.context("conversation not found")?;
        let new_title = title.unwrap_or(&existing.title);
        let new_status = status.unwrap_or(&existing.status);
        let now = utc_timestamp_string();
        let metadata_json = json_to_string(metadata.clone().or_else(|| existing.metadata.clone()));

        let conn = self.connection.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET title = ?1, updated_at = ?2, status = ?3, metadata_json = ?4 WHERE id = ?5",
            params![new_title, now, new_status, metadata_json, conversation_id],
        )?;

        Ok(Conversation {
            id: existing.id,
            title: new_title.to_string(),
            created_at: existing.created_at,
            updated_at: now,
            model: existing.model,
            working_directory: existing.working_directory,
            status: new_status.to_string(),
            metadata: metadata.or(existing.metadata),
        })
    }

    pub fn delete(&self, conversation_id: &str) -> Result<bool> {
        let conn = self.connection.lock().unwrap();
        let changed = conn.execute("DELETE FROM conversations WHERE id = ?1", params![conversation_id])?;
        Ok(changed > 0)
    }
}

impl MessageStore {
    pub fn create(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        parent_id: Option<&str>,
        metadata: Option<Value>,
    ) -> Result<Message> {
        let id = Uuid::new_v4().to_string();
        let now = utc_timestamp_string();
        let redacted = redact_secrets(content);
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at, parent_id, token_count, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, conversation_id, role, redacted, now, parent_id, Option::<i64>::None, json_to_string(metadata.clone())],
        )?;

        Ok(Message {
            id,
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: redacted,
            created_at: now,
            parent_id: parent_id.map(str::to_string),
            token_count: None,
            metadata,
        })
    }

    pub fn get(&self, message_id: &str) -> Result<Option<Message>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, parent_id, token_count, metadata_json FROM messages WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![message_id], message_row_to_struct).optional()?;
        Ok(row)
    }

    pub fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<Message>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, parent_id, token_count, metadata_json FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![conversation_id, limit as i64], message_row_to_struct)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.created_at FROM messages AS m JOIN messages_fts ON messages_fts.message_id = m.id WHERE messages_fts MATCH ?1 ORDER BY m.created_at DESC LIMIT 25"
        )?;
        let rows = stmt.query_map(params![trimmed], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
                relevance: 100,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

impl SessionStore {
    pub fn create(
        &self,
        conversation_id: Option<&str>,
        working_directory: Option<&str>,
        metadata: Option<Value>,
    ) -> Result<Session> {
        let id = Uuid::new_v4().to_string();
        let started_at = utc_timestamp_string();
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, conversation_id, started_at, ended_at, working_directory, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, conversation_id, started_at, Option::<String>::None, working_directory, json_to_string(metadata.clone())],
        )?;

        Ok(Session {
            id,
            conversation_id: conversation_id.map(str::to_string),
            started_at: started_at.clone(),
            ended_at: None,
            working_directory: working_directory.map(str::to_string),
            metadata,
        })
    }

    pub fn get(&self, session_id: &str) -> Result<Option<Session>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, started_at, ended_at, working_directory, metadata_json FROM sessions WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![session_id], session_row_to_struct).optional()?;
        Ok(row)
    }

    pub fn close(&self, session_id: &str) -> Result<bool> {
        let conn = self.connection.lock().unwrap();
        let ended_at = utc_timestamp_string();
        let changed = conn.execute(
            "UPDATE sessions SET ended_at = ?1 WHERE id = ?2 AND ended_at IS NULL",
            params![ended_at, session_id],
        )?;
        Ok(changed > 0)
    }
}

impl ActionStore {
    pub fn start(
        &self,
        conversation_id: Option<&str>,
        session_id: Option<&str>,
        action_type: &str,
        command: Option<&str>,
        metadata: Option<Value>,
    ) -> Result<AgentAction> {
        let id = Uuid::new_v4().to_string();
        let started_at = utc_timestamp_string();
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_actions (id, conversation_id, session_id, action_type, command, status, started_at, completed_at, output, error, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, conversation_id, session_id, action_type, command, "running", started_at, Option::<String>::None, Option::<String>::None, Option::<String>::None, json_to_string(metadata.clone())],
        )?;

        Ok(AgentAction {
            id: id.clone(),
            conversation_id: conversation_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            action_type: action_type.to_string(),
            command: command.map(str::to_string),
            status: "running".to_string(),
            started_at: started_at.clone(),
            completed_at: None,
            output: None,
            error: None,
            metadata,
        })
    }

    pub fn complete(
        &self,
        action_id: &str,
        status: &str,
        output: Option<&str>,
        error: Option<&str>,
    ) -> Result<AgentAction> {
        let conn = self.connection.lock().unwrap();
        let completed_at = utc_timestamp_string();
        conn.execute(
            "UPDATE agent_actions SET status = ?1, completed_at = ?2, output = ?3, error = ?4 WHERE id = ?5",
            params![status, completed_at, output, error, action_id],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, session_id, action_type, command, status, started_at, completed_at, output, error, metadata_json FROM agent_actions WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![action_id], action_row_to_struct)?;
        Ok(row)
    }

    pub fn list(&self, limit: usize) -> Result<Vec<AgentAction>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, session_id, action_type, command, status, started_at, completed_at, output, error, metadata_json FROM agent_actions ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], action_row_to_struct)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_for_conversation(&self, conversation_id: &str, limit: usize) -> Result<Vec<AgentAction>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, session_id, action_type, command, status, started_at, completed_at, output, error, metadata_json FROM agent_actions WHERE conversation_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![conversation_id, limit as i64], action_row_to_struct)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

impl DockerStore {
    pub fn record_event(
        &self,
        container_id: Option<&str>,
        container_name: Option<&str>,
        event_type: Option<&str>,
        status: Option<&str>,
        cpu_percent: Option<f64>,
        memory_bytes: Option<i64>,
        network_rx_bytes: Option<i64>,
        network_tx_bytes: Option<i64>,
        metadata: Option<Value>,
    ) -> Result<DockerEvent> {
        let id = Uuid::new_v4().to_string();
        let ts = utc_timestamp_string();
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO docker_events (id, container_id, container_name, event_type, status, timestamp, cpu_percent, memory_bytes, network_rx_bytes, network_tx_bytes, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, container_id, container_name, event_type, status, ts, cpu_percent, memory_bytes, network_rx_bytes, network_tx_bytes, json_to_string(metadata.clone())],
        )?;

        Ok(DockerEvent {
            id,
            container_id: container_id.map(str::to_string),
            container_name: container_name.map(str::to_string),
            event_type: event_type.map(str::to_string),
            status: status.map(str::to_string),
            timestamp: ts,
            cpu_percent,
            memory_bytes,
            network_rx_bytes,
            network_tx_bytes,
            metadata,
        })
    }

    pub fn get_stats(&self, limit: usize) -> Result<Vec<DockerEvent>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, container_id, container_name, event_type, status, timestamp, cpu_percent, memory_bytes, network_rx_bytes, network_tx_bytes, metadata_json FROM docker_events ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], docker_event_row_to_struct)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

impl ArtifactStore {
    pub fn save(
        &self,
        conversation_id: Option<&str>,
        session_id: Option<&str>,
        type_: &str,
        name: Option<&str>,
        mime_type: Option<&str>,
        body: &[u8],
        metadata: Option<Value>,
    ) -> Result<Artifact> {
        let id = Uuid::new_v4().to_string();
        let created_at = utc_timestamp_string();
        let root = PathBuf::from(&self.config.home).join("data").join("conversations");
        let conversation_dir = conversation_id
            .map(|id| root.join(id))
            .unwrap_or_else(|| root.join("scratch"));
        let artifact_dir = conversation_dir.join("artifacts");
        fs::create_dir_all(&artifact_dir)?;
        let path = artifact_dir.join(format!("{id}.json"));

        let payload = JsonEnvelope {
            id: id.clone(),
            type_: type_.to_string(),
            created_at: created_at.clone(),
            data: metadata.clone().unwrap_or_else(|| serde_json::json!({"name": name, "size_bytes": body.len()})),
        };
        atomic_write_json(&path, &payload)?;

        let checksum = format!("{:x}", Sha256::digest(body));
        let size_bytes = body.len() as i64;

        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, conversation_id, session_id, type, name, path, mime_type, size_bytes, checksum, created_at, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, conversation_id, session_id, type_, name, path.display().to_string(), mime_type, size_bytes, checksum, created_at, json_to_string(metadata.clone())],
        )?;

        Ok(Artifact {
            id,
            conversation_id: conversation_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            type_: type_.to_string(),
            name: name.map(str::to_string),
            path: path.display().to_string(),
            mime_type: mime_type.map(str::to_string),
            size_bytes: Some(size_bytes),
            checksum: Some(checksum),
            created_at,
            metadata,
        })
    }

    pub fn get(&self, artifact_id: &str) -> Result<Option<Artifact>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, session_id, type, name, path, mime_type, size_bytes, checksum, created_at, metadata_json FROM artifacts WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![artifact_id], artifact_row_to_struct).optional()?;
        Ok(row)
    }

    pub fn delete(&self, artifact_id: &str) -> Result<bool> {
        let conn = self.connection.lock().unwrap();
        let changed = conn.execute("DELETE FROM artifacts WHERE id = ?1", params![artifact_id])?;
        Ok(changed > 0)
    }

    pub fn list_for_conversation(&self, conversation_id: &str, limit: usize) -> Result<Vec<Artifact>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, session_id, type, name, path, mime_type, size_bytes, checksum, created_at, metadata_json FROM artifacts WHERE conversation_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![conversation_id, limit as i64], artifact_row_to_struct)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

impl SearchStore {
    pub fn query(&self, query: &str) -> Result<Vec<SearchResult>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.created_at FROM messages AS m JOIN messages_fts ON messages_fts.message_id = m.id WHERE messages_fts MATCH ?1 ORDER BY m.created_at DESC LIMIT 25",
        )?;
        let rows = stmt.query_map(params![trimmed], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
                relevance: 100,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

fn ensure_layout(config: &StorageConfig) -> Result<()> {
    let root = PathBuf::from(&config.home);
    fs::create_dir_all(root.join("config"))?;
    fs::create_dir_all(root.join("data").join("conversations"))?;
    fs::create_dir_all(root.join("data").join("sessions"))?;
    fs::create_dir_all(root.join("data").join("cache"))?;
    fs::create_dir_all(root.join("data").join("events"))?;
    fs::create_dir_all(root.join("logs"))?;
    Ok(())
}

fn write_default_config(config: &StorageConfig) -> Result<()> {
    let path = PathBuf::from(&config.home).join("config").join("config.json");
    if path.exists() {
        return Ok(());
    }
    atomic_write_text(&path, &serde_json::to_string_pretty(config)?)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL)",
        [],
    )?;

    let migrations = [(
        "001_init_storage",
        r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                model TEXT,
                working_directory TEXT,
                status TEXT DEFAULT 'active',
                metadata_json TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                parent_id TEXT,
                token_count INTEGER,
                metadata_json TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);

            CREATE TABLE IF NOT EXISTS agent_actions (
                id TEXT PRIMARY KEY,
                conversation_id TEXT,
                session_id TEXT,
                action_type TEXT NOT NULL,
                command TEXT,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                output TEXT,
                error TEXT,
                metadata_json TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS docker_events (
                id TEXT PRIMARY KEY,
                container_id TEXT,
                container_name TEXT,
                event_type TEXT,
                status TEXT,
                timestamp TEXT NOT NULL,
                cpu_percent REAL,
                memory_bytes INTEGER,
                network_rx_bytes INTEGER,
                network_tx_bytes INTEGER,
                metadata_json TEXT
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                conversation_id TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                working_directory TEXT,
                metadata_json TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                conversation_id TEXT,
                session_id TEXT,
                type TEXT NOT NULL,
                name TEXT,
                path TEXT NOT NULL,
                mime_type TEXT,
                size_bytes INTEGER,
                checksum TEXT,
                created_at TEXT NOT NULL,
                metadata_json TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                message_id UNINDEXED,
                conversation_id UNINDEXED,
                content
            );

            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(message_id, conversation_id, content)
                VALUES (new.id, new.conversation_id, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, message_id, conversation_id, content)
                VALUES ('delete', old.rowid, old.id, old.conversation_id, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, message_id, conversation_id, content)
                VALUES ('delete', old.rowid, old.id, old.conversation_id, old.content);
                INSERT INTO messages_fts(message_id, conversation_id, content)
                VALUES (new.id, new.conversation_id, new.content);
            END;
        "#,
    )];

    for (name, sql) in migrations {
        let applied = conn
            .query_row("SELECT 1 FROM schema_migrations WHERE name = ?1", params![name], |row| row.get::<_, i32>(0))
            .optional()?;
        if applied.is_none() {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (name, applied_at) VALUES (?1, ?2)",
                params![name, utc_timestamp_string()],
            )?;
        }
    }

    Ok(())
}

fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp, bytes)?;
    // Close the handle before renaming — on Windows, renaming an open file
    // fails with "Access is denied" (os error 5).
    {
        let file = fs::File::open(&tmp)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn atomic_write_text(path: &Path, contents: &str) -> Result<()> {
    // Callers guard existence already; write-in-place avoids Windows rename
    // semantics (renaming over/flushing a just-written file can fail hard).
    fs::write(path, contents)?;
    if let Ok(file) = fs::File::open(path) {
        if let Ok(h) = file.try_clone() {
            // Best-effort sync so the config reaches disk.
            let _ = h.sync_all();
        }
    }
    Ok(())
}

fn redact_secrets(input: &str) -> String {
    let mut output = input.replace("API_KEY=", "API_KEY=[REDACTED]\n");
    output = output.replace("TOKEN=", "TOKEN=[REDACTED]\n");
    output = output.replace("PASSWORD=", "PASSWORD=[REDACTED]\n");
    output = output.replace("Authorization: Bearer ", "Authorization: Bearer [REDACTED]");
    output
}

fn json_to_string(value: Option<Value>) -> Option<String> {
    value.map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()))
}

fn option_json_from_sql(value: Option<String>) -> Option<Value> {
    value.and_then(|text| serde_json::from_str(&text).ok())
}

fn default_home() -> PathBuf {
    if let Ok(value) = std::env::var("OPTIDOC_HOME") {
        return PathBuf::from(value);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".optidoc")
}

fn utc_timestamp_string() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    format_utc(now)
}

fn utc_timestamp_string_minus(seconds: i64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64 - seconds;
    format_utc(now)
}

fn format_utc(secs: i64) -> String {
    let secs = secs.max(0);
    let date = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_else(chrono::Utc::now);
    date.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn current_date_string() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%d")
        .to_string()
}

fn conversation_row_to_struct(row: &Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        model: row.get(4)?,
        working_directory: row.get(5)?,
        status: row.get(6)?,
        metadata: option_json_from_sql(row.get(7)?),
    })
}

fn message_row_to_struct(row: &Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        parent_id: row.get(5)?,
        token_count: row.get(6)?,
        metadata: option_json_from_sql(row.get(7)?),
    })
}

fn session_row_to_struct(row: &Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        working_directory: row.get(4)?,
        metadata: option_json_from_sql(row.get(5)?),
    })
}

fn action_row_to_struct(row: &Row<'_>) -> rusqlite::Result<AgentAction> {
    Ok(AgentAction {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        session_id: row.get(2)?,
        action_type: row.get(3)?,
        command: row.get(4)?,
        status: row.get(5)?,
        started_at: row.get(6)?,
        completed_at: row.get(7)?,
        output: row.get(8)?,
        error: row.get(9)?,
        metadata: option_json_from_sql(row.get(10)?),
    })
}

fn docker_event_row_to_struct(row: &Row<'_>) -> rusqlite::Result<DockerEvent> {
    Ok(DockerEvent {
        id: row.get(0)?,
        container_id: row.get(1)?,
        container_name: row.get(2)?,
        event_type: row.get(3)?,
        status: row.get(4)?,
        timestamp: row.get(5)?,
        cpu_percent: row.get(6)?,
        memory_bytes: row.get(7)?,
        network_rx_bytes: row.get(8)?,
        network_tx_bytes: row.get(9)?,
        metadata: option_json_from_sql(row.get(10)?),
    })
}

fn artifact_row_to_struct(row: &Row<'_>) -> rusqlite::Result<Artifact> {
    Ok(Artifact {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        session_id: row.get(2)?,
        type_: row.get(3)?,
        name: row.get(4)?,
        path: row.get(5)?,
        mime_type: row.get(6)?,
        size_bytes: row.get(7)?,
        checksum: row.get(8)?,
        created_at: row.get(9)?,
        metadata: option_json_from_sql(row.get(10)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_storage_and_persists_message() -> Result<()> {
        let home = std::env::temp_dir().join(format!("optidock-storage-{}", Uuid::new_v4()));
        let storage = Storage::open(&home)?;
        let conversation = storage.conversations.create("Test conversation", None)?;
        storage.messages.create(&conversation.id, "user", "Hello local storage", None, None)?;
        let messages = storage.messages.list(&conversation.id, 10)?;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        Ok(())
    }
}
