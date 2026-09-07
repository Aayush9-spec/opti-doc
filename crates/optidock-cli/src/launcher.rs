use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use optidock_core::{LocalMemoryStore, MemoryEntry, MemoryKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{broadcast, Mutex},
    time::sleep,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeEventBus {
    tx: broadcast::Sender<RuntimeEvent>,
}

impl RuntimeEventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event_type: impl Into<String>, message: impl Into<String>) {
        let timestamp = current_timestamp();
        let _ = self.tx.send(RuntimeEvent {
            timestamp,
            event_type: event_type.into(),
            message: message.into(),
        });
    }
}

#[derive(Debug, Default)]
pub struct ActionStream {
    entries: VecDeque<RuntimeEvent>,
}

impl ActionStream {
    pub fn push(&mut self, event: RuntimeEvent) {
        if self.entries.len() >= 120 {
            self.entries.pop_front();
        }
        self.entries.push_back(event);
    }

    pub fn recent(&self) -> Vec<RuntimeEvent> {
        self.entries.iter().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveContextMemoryEntry {
    pub scope: String,
    pub source: String,
    pub message: String,
    pub payload: serde_json::Value,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveContextMemorySnapshot {
    pub entries: Vec<CognitiveContextMemoryEntry>,
    pub current_goal: String,
}

#[derive(Debug)]
pub struct CognitiveContextMemoryEngine {
    root: PathBuf,
    entries: Vec<CognitiveContextMemoryEntry>,
}

impl CognitiveContextMemoryEngine {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).context("unable to create memory root")?;
        let path = root.join("ccme.json");
        let entries = if path.exists() {
            let data = fs::read_to_string(&path).context("unable to read memory index")?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(Self { root, entries })
    }

    pub fn record(&mut self, source: &str, message: &str, payload: serde_json::Value) -> Result<()> {
        let entry = CognitiveContextMemoryEntry {
            scope: self.root.to_string_lossy().into_owned(),
            source: source.to_string(),
            message: message.to_string(),
            payload,
            created_at: current_timestamp(),
        };
        self.entries.push(entry);
        self.entries.truncate(64);
        self.persist()
    }

    pub fn snapshot(&self) -> CognitiveContextMemorySnapshot {
        let current_goal = self
            .entries
            .iter()
            .filter(|entry| entry.message.contains("backend") || entry.message.contains("docker"))
            .map(|entry| entry.message.clone())
            .last()
            .unwrap_or_else(|| "Monitor runtime health".to_string());

        CognitiveContextMemorySnapshot {
            entries: self.entries.clone(),
            current_goal,
        }
    }

    fn persist(&self) -> Result<()> {
        let path = self.root.join("ccme.json");
        let data = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&path, data).context("unable to write memory index")?;
        Ok(())
    }
}

pub async fn run_launcher(path: &str) -> Result<()> {
    let bus = RuntimeEventBus::new(256);
    let memory_path = PathBuf::from(path).join(".optidock").join("runtime-memory.db");
    let store = LocalMemoryStore::new(&memory_path)?;
    let memory_root = PathBuf::from(path).join(".optidock").join("ccme");
    let mut memory = CognitiveContextMemoryEngine::new(&memory_root)?;
    let stream_state = Arc::new(Mutex::new(ActionStream::default()));
    let stream_state_for_task = Arc::clone(&stream_state);

    bus.publish("launcher", "starting OptiDoc runtime");
    bus.publish("launcher", "initializing action stream");
    bus.publish("launcher", format!("loading project context from {}", path));

    let runtime_entry = MemoryEntry {
        id: None,
        scope: path.to_string(),
        kind: MemoryKind::Project,
        title: "runtime-launch".to_string(),
        summary: format!("OptiDoc runtime launched for {}", path),
        payload: serde_json::json!({"path": path, "kind": "launcher"}).to_string(),
        confidence: 0.95,
        importance: 0.9,
        created_at: 0,
        updated_at: 0,
    };
    let _ = store.save(&runtime_entry);

    memory.record("launcher", "started runtime command center", serde_json::json!({"path": path}))?;
    memory.record("launcher", "prepared multi-terminal runtime layout", serde_json::json!({"mode": "dashboard"}))?;

    let mut subscriber = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match subscriber.recv().await {
                Ok(event) => {
                    let mut stream = stream_state_for_task.lock().await;
                    stream.push(event);
                }
                Err(_) => break,
            }
        }
    });

    let initial_snapshot = memory.snapshot();
    run_dashboard_ui(path, &initial_snapshot, &stream_state, &memory_root).await?;

    Ok(())
}

async fn run_dashboard_ui(
    path: &str,
    initial_snapshot: &CognitiveContextMemorySnapshot,
    stream_state: &Arc<Mutex<ActionStream>>,
    memory_root: &Path,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let stream = stream_state.lock().await.recent();
            terminal.draw(|frame| {
                render_dashboard(frame, path, initial_snapshot, &stream, memory_root);
            })?;
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_dashboard(
    frame: &mut ratatui::Frame<'_>,
    path: &str,
    initial_snapshot: &CognitiveContextMemorySnapshot,
    recent_events: &[RuntimeEvent],
    memory_root: &Path,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(5), Constraint::Length(5)])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled("OptiDoc Runtime Command Center", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(format!("Workspace: {}", path)),
        Line::from(format!("CCME Store: {}", memory_root.display())),
        Line::from(format!("Current Goal: {}", initial_snapshot.current_goal)),
    ])
    .block(Block::default().borders(Borders::ALL).title("Runtime"));
    frame.render_widget(header, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    let events: Vec<Line> = recent_events
        .iter()
        .rev()
        .take(10)
        .map(|event| Line::from(format!("[{}] {}: {}", event.timestamp, event.event_type, event.message)))
        .collect();
    let action_stream = Paragraph::new(events)
        .block(Block::default().borders(Borders::ALL).title("Action Stream"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(action_stream, body_chunks[0]);

    let terminal_rows = vec![
        Row::new(vec!["Terminal 1", "Agent CLI", "Active"]),
        Row::new(vec!["Terminal 2", "Action Stream", "Streaming"]),
        Row::new(vec!["Terminal 3", "Docker Dashboard", "Refreshing"]),
    ];
    let terminal_table = Table::new(
        terminal_rows,
        [Constraint::Length(12), Constraint::Length(20), Constraint::Length(12)],
    )
    .header(Row::new(vec!["Terminal", "Role", "Status"]).style(Style::default().add_modifier(Modifier::BOLD)))
    .block(Block::default().borders(Borders::ALL).title("Terminals"))
    .widths([Constraint::Length(12), Constraint::Length(20), Constraint::Length(12)]);
    frame.render_widget(terminal_table, body_chunks[1]);

    let memory_summary = Paragraph::new(vec![
        Line::from(format!("Stored Entries: {}", initial_snapshot.entries.len())),
        Line::from("Mode: live event stream"),
        Line::from("Press q to exit the dashboard"),
    ])
    .block(Block::default().borders(Borders::ALL).title("Memory"))
    .alignment(Alignment::Left);
    frame.render_widget(memory_summary, chunks[2]);
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_runtime_context_for_dashboard_usage() {
        let temp_dir = std::env::temp_dir().join(format!(
            "optidock-ccme-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut engine = CognitiveContextMemoryEngine::new(&temp_dir).unwrap();
        engine
            .record(
                "launcher",
                "started runtime command center",
                serde_json::json!({"path": "/tmp/workspace"}),
            )
            .unwrap();
        engine
            .record(
                "docker",
                "backend container healthy",
                serde_json::json!({"container": "backend"}),
            )
            .unwrap();

        let snapshot = engine.snapshot();
        assert_eq!(snapshot.entries.len(), 2);
        assert!(snapshot.current_goal.contains("backend"));
    }
}
