use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    process::Stdio,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::process::Command;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct RecoveryAgentConfig {
    pub heartbeat_interval: Duration,
    pub health_check_interval: Duration,
    pub max_recovery_attempts: usize,
    pub log_tail: usize,
    pub response_timeout: Duration,
    pub memory_warning_percent: f64,
    pub memory_critical_percent: f64,
    pub health_path: String,
    pub dry_run: bool,
    pub allow_container_restart: bool,
    pub app_restart_command: Option<String>,
    pub worker_restart_command: Option<String>,
    pub reload_command: Option<String>,
    pub service_restart_command: Option<String>,
}

impl Default for RecoveryAgentConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            max_recovery_attempts: 3,
            log_tail: 200,
            response_timeout: Duration::from_millis(2_000),
            memory_warning_percent: 80.0,
            memory_critical_percent: 95.0,
            health_path: "/".to_string(),
            dry_run: true,
            allow_container_restart: false,
            app_restart_command: None,
            worker_restart_command: None,
            reload_command: None,
            service_restart_command: None,
        }
    }
}

impl RecoveryAgentConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        config.heartbeat_interval =
            Duration::from_secs(env_u64("OPTIDOCK_AGENT_HEARTBEAT_SECS", 30));
        config.health_check_interval =
            Duration::from_secs(env_u64("OPTIDOCK_AGENT_HEALTH_CHECK_SECS", 10));
        config.max_recovery_attempts = env_usize("OPTIDOCK_AGENT_MAX_RECOVERY_ATTEMPTS", 3).max(1);
        config.log_tail = env_usize("OPTIDOCK_AGENT_LOG_TAIL", 200).max(1);
        config.response_timeout =
            Duration::from_millis(env_u64("OPTIDOCK_AGENT_RESPONSE_TIMEOUT_MS", 2_000));
        config.memory_warning_percent =
            env_f64("OPTIDOCK_AGENT_MEMORY_WARN_PERCENT", 80.0).clamp(1.0, 100.0);
        config.memory_critical_percent =
            env_f64("OPTIDOCK_AGENT_MEMORY_CRITICAL_PERCENT", 95.0).clamp(1.0, 100.0);
        config.health_path = normalize_health_path(
            &std::env::var("OPTIDOCK_AGENT_HEALTH_PATH").unwrap_or_else(|_| "/".to_string()),
        );
        config.dry_run = !env_bool("OPTIDOCK_AGENT_APPLY", false);
        config.allow_container_restart = env_bool("OPTIDOCK_AGENT_ALLOW_CONTAINER_RESTART", false);
        config.app_restart_command = env_nonempty("OPTIDOCK_AGENT_APP_RESTART_COMMAND");
        config.worker_restart_command = env_nonempty("OPTIDOCK_AGENT_WORKER_RESTART_COMMAND");
        config.reload_command = env_nonempty("OPTIDOCK_AGENT_RELOAD_COMMAND");
        config.service_restart_command = env_nonempty("OPTIDOCK_AGENT_SERVICE_RESTART_COMMAND");

        config
    }

    pub fn with_cli_permissions(mut self, apply: bool, restart_containers: bool) -> Self {
        self.dry_run = !apply;
        self.allow_container_restart = restart_containers;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerIdentity {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
}

impl ContainerIdentity {
    fn short_id(&self) -> String {
        self.id.chars().take(12).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerRuntimeState {
    Created,
    Running,
    Restarting,
    Paused,
    Exited,
    Dead,
    Unknown,
}

impl ContainerRuntimeState {
    fn from_docker_status(status: &str) -> Self {
        match status.to_ascii_lowercase().as_str() {
            "created" => Self::Created,
            "running" => Self::Running,
            "restarting" => Self::Restarting,
            "paused" => Self::Paused,
            "exited" => Self::Exited,
            "dead" => Self::Dead,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInspection {
    pub identity: ContainerIdentity,
    pub runtime_state: ContainerRuntimeState,
    pub docker_health: Option<String>,
    pub exit_code: Option<i64>,
    pub error: Option<String>,
    pub oom_killed: bool,
    pub restart_count: u64,
    pub env: Vec<String>,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub memory_usage: Option<String>,
    pub network_io: Option<String>,
    pub block_io: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointProbe {
    pub url: String,
    pub status_code: Option<u16>,
    pub response_time_ms: u64,
    pub error: Option<String>,
}

impl EndpointProbe {
    fn healthy(&self) -> bool {
        self.error.is_none() && self.status_code.map(|status| status < 500).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait ContainerRuntime: Send + Sync {
    fn list_active_containers<'a>(&'a self) -> BoxFuture<'a, Result<Vec<ContainerIdentity>>>;
    fn inspect_container<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<ContainerInspection>>;
    fn container_logs<'a>(
        &'a self,
        container_id: &'a str,
        tail: usize,
    ) -> BoxFuture<'a, Result<String>>;
    fn container_stats<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<ResourceSnapshot>>;
    fn exec_container<'a>(
        &'a self,
        container_id: &'a str,
        command: &'a [&'a str],
    ) -> BoxFuture<'a, Result<CommandResult>>;
    fn restart_container<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<CommandResult>>;
    fn probe_endpoint<'a>(
        &'a self,
        url: &'a str,
        timeout: Duration,
    ) -> BoxFuture<'a, Result<EndpointProbe>>;
}

#[derive(Debug, Clone)]
pub struct DockerCliRuntime {
    client: reqwest::Client,
}

impl DockerCliRuntime {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DockerCliRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerRuntime for DockerCliRuntime {
    fn list_active_containers<'a>(&'a self) -> BoxFuture<'a, Result<Vec<ContainerIdentity>>> {
        Box::pin(async move {
            let output = docker_command(["ps", "-a", "--format", "{{json .}}"]).await?;
            let mut containers = Vec::new();

            for line in output.lines().filter(|line| !line.trim().is_empty()) {
                let row: DockerPsRow = serde_json::from_str(line)
                    .with_context(|| format!("failed to parse docker ps row: {line}"))?;
                containers.push(ContainerIdentity {
                    id: row.id,
                    name: row.names,
                    image: row.image,
                    status: row.status,
                    ports: row.ports.unwrap_or_default(),
                });
            }

            Ok(containers)
        })
    }

    fn inspect_container<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<ContainerInspection>> {
        Box::pin(async move {
            let output = docker_command(["inspect", container_id]).await?;
            let values: Vec<Value> = serde_json::from_str(&output)
                .with_context(|| format!("failed to parse docker inspect for {container_id}"))?;
            let value = values
                .first()
                .with_context(|| format!("docker inspect returned no data for {container_id}"))?;

            Ok(parse_inspection(value))
        })
    }

    fn container_logs<'a>(
        &'a self,
        container_id: &'a str,
        tail: usize,
    ) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            docker_command(vec![
                "logs".to_string(),
                "--tail".to_string(),
                tail.to_string(),
                container_id.to_string(),
            ])
            .await
        })
    }

    fn container_stats<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<ResourceSnapshot>> {
        Box::pin(async move {
            let output = docker_command([
                "stats",
                "--no-stream",
                "--format",
                "{{json .}}",
                container_id,
            ])
            .await?;

            let row: DockerStatsRow = serde_json::from_str(output.trim())
                .with_context(|| format!("failed to parse docker stats for {container_id}"))?;

            Ok(ResourceSnapshot {
                cpu_percent: parse_percent(&row.cpu_perc),
                memory_percent: parse_percent(&row.mem_perc),
                memory_usage: row.mem_usage,
                network_io: row.net_io,
                block_io: row.block_io,
            })
        })
    }

    fn exec_container<'a>(
        &'a self,
        container_id: &'a str,
        command: &'a [&'a str],
    ) -> BoxFuture<'a, Result<CommandResult>> {
        Box::pin(async move {
            let mut args = vec!["exec", container_id];
            args.extend(command.iter().copied());
            docker_command_result(args).await
        })
    }

    fn restart_container<'a>(
        &'a self,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<CommandResult>> {
        Box::pin(async move { docker_command_result(["restart", container_id]).await })
    }

    fn probe_endpoint<'a>(
        &'a self,
        url: &'a str,
        timeout: Duration,
    ) -> BoxFuture<'a, Result<EndpointProbe>> {
        Box::pin(async move {
            let start = Instant::now();
            let result = self.client.get(url).timeout(timeout).send().await;
            let response_time_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(response) => Ok(EndpointProbe {
                    url: url.to_string(),
                    status_code: Some(response.status().as_u16()),
                    response_time_ms,
                    error: None,
                }),
                Err(error) => Ok(EndpointProbe {
                    url: url.to_string(),
                    status_code: None,
                    response_time_ms,
                    error: Some(error.to_string()),
                }),
            }
        })
    }
}

#[derive(Debug, Deserialize)]
struct DockerPsRow {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Image")]
    image: String,
    #[serde(rename = "Names")]
    names: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "Ports")]
    ports: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DockerStatsRow {
    #[serde(rename = "CPUPerc")]
    cpu_perc: Option<String>,
    #[serde(rename = "MemPerc")]
    mem_perc: Option<String>,
    #[serde(rename = "MemUsage")]
    mem_usage: Option<String>,
    #[serde(rename = "NetIO")]
    net_io: Option<String>,
    #[serde(rename = "BlockIO")]
    block_io: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerObservation {
    pub container: ContainerIdentity,
    pub inspection: ContainerInspection,
    pub logs: String,
    pub resources: ResourceSnapshot,
    pub endpoint_probe: Option<EndpointProbe>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Crashed,
    Escalated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorKind {
    Http500,
    InternalServerError,
    RuntimeException,
    Crash,
    UnhealthyDockerStatus,
    OomKilled,
    MemoryLeak,
    PortFailure,
    DependencyFailure,
    MissingEnvironmentVariable,
    DatabaseConnectionFailure,
    RedisFailure,
    ApiFailure,
    Timeout,
    NetworkFailure,
    SegmentationFault,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorSignal {
    pub kind: ErrorKind,
    pub severity: SignalSeverity,
    pub message: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RootCauseCategory {
    ApplicationException,
    DatabaseUnavailable,
    RedisUnavailable,
    MissingConfiguration,
    DependencyMissing,
    ResourceExhaustion,
    ProcessCrash,
    NetworkFailure,
    PortFailure,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {
    pub category: RootCauseCategory,
    pub summary: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecoveryAction {
    RetryRequest,
    RestartApplicationProcess,
    RestartWorker,
    ReloadConfiguration,
    RestartService,
    RestartContainer,
    Escalate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecoveryAttemptStatus {
    Planned,
    Skipped,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub action: RecoveryAction,
    pub status: RecoveryAttemptStatus,
    pub detail: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecoveryStatus {
    NotNeeded,
    Planned,
    Recovered,
    Failed,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationReport {
    pub container_id: String,
    pub container_name: String,
    pub root_cause: RootCause,
    pub recovery_attempts: Vec<RecoveryAttempt>,
    pub logs: Vec<String>,
    pub suggested_resolution: String,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub status: RecoveryStatus,
    pub root_cause: Option<RootCause>,
    pub attempts: Vec<RecoveryAttempt>,
    pub verification: String,
    pub escalation: Option<EscalationReport>,
}

impl RecoveryReport {
    fn not_needed() -> Self {
        Self {
            status: RecoveryStatus::NotNeeded,
            root_cause: None,
            attempts: Vec::new(),
            verification: "no active error signals".to_string(),
            escalation: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub agent_id: String,
    pub container_id: String,
    pub container_name: String,
    pub status: AgentHealthStatus,
    pub cpu_percent: Option<f64>,
    pub ram_percent: Option<f64>,
    pub response_time_ms: Option<u64>,
    pub running_services: Vec<String>,
    pub current_errors: Vec<ErrorSignal>,
    pub recovery_status: RecoveryStatus,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAgentRecord {
    pub agent_id: String,
    pub container_id: String,
    pub container_name: String,
    pub status: AgentHealthStatus,
    pub last_heartbeat: Option<Heartbeat>,
    pub recovery_attempt_count: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentRegistry {
    pub agents: BTreeMap<String, LocalAgentRecord>,
}

impl AgentRegistry {
    pub fn ensure_local_agent(&mut self, container: &ContainerIdentity) -> bool {
        if let Some(record) = self.agents.get_mut(&container.id) {
            record.container_name = container.name.clone();
            record.active = true;
            return false;
        }

        let agent_id = format!("local-agent-{}", container.short_id());
        self.agents.insert(
            container.id.clone(),
            LocalAgentRecord {
                agent_id,
                container_id: container.id.clone(),
                container_name: container.name.clone(),
                status: AgentHealthStatus::Healthy,
                last_heartbeat: None,
                recovery_attempt_count: 0,
                active: true,
            },
        );
        true
    }

    pub fn deactivate_missing(&mut self, active_container_ids: &BTreeSet<String>) {
        for record in self.agents.values_mut() {
            if !active_container_ids.contains(&record.container_id) {
                record.active = false;
                record.status = AgentHealthStatus::Crashed;
            }
        }
    }

    pub fn local_agent_count_for(&self, container_id: &str) -> usize {
        self.agents
            .values()
            .filter(|record| record.container_id == container_id && record.active)
            .count()
    }

    fn recovery_attempt_count(&self, container_id: &str) -> usize {
        self.agents
            .get(container_id)
            .map(|record| record.recovery_attempt_count)
            .unwrap_or(0)
    }

    fn record_heartbeat(&mut self, heartbeat: Heartbeat) {
        if let Some(record) = self.agents.get_mut(&heartbeat.container_id) {
            record.status = heartbeat.status;
            record.last_heartbeat = Some(heartbeat);
        }
    }

    fn record_recovery_cycle(&mut self, container_id: &str, report: &RecoveryReport) {
        if let Some(record) = self.agents.get_mut(container_id) {
            match report.status {
                RecoveryStatus::NotNeeded | RecoveryStatus::Recovered => {
                    record.recovery_attempt_count = 0;
                }
                RecoveryStatus::Planned | RecoveryStatus::Failed | RecoveryStatus::Escalated => {
                    record.recovery_attempt_count += 1;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogEntry {
    pub level: String,
    pub container: String,
    pub issue: String,
    pub root_cause: String,
    pub recovery: String,
    pub verification: String,
    pub duration_ms: u64,
    pub status: RecoveryStatus,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterAgentReport {
    pub status: AgentHealthStatus,
    pub registry: AgentRegistry,
    pub heartbeats: Vec<Heartbeat>,
    pub escalations: Vec<EscalationReport>,
    pub audit_log: Vec<AgentLogEntry>,
}

pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &'static str;
    fn inspect(
        &self,
        observation: &ContainerObservation,
        config: &RecoveryAgentConfig,
    ) -> Vec<ErrorSignal>;
}

pub struct DockerStateHealthCheck;
pub struct ResourceHealthCheck;
pub struct EndpointHealthCheck;
pub struct LogPatternHealthCheck;

impl HealthCheck for DockerStateHealthCheck {
    fn name(&self) -> &'static str {
        "docker-state"
    }

    fn inspect(
        &self,
        observation: &ContainerObservation,
        _config: &RecoveryAgentConfig,
    ) -> Vec<ErrorSignal> {
        let mut signals = Vec::new();

        match observation.inspection.runtime_state {
            ContainerRuntimeState::Running => {}
            ContainerRuntimeState::Restarting => push_signal(
                &mut signals,
                ErrorKind::Crash,
                SignalSeverity::Warning,
                "container is restarting",
                vec![observation.inspection.identity.status.clone()],
            ),
            ContainerRuntimeState::Exited | ContainerRuntimeState::Dead => push_signal(
                &mut signals,
                ErrorKind::Crash,
                SignalSeverity::Critical,
                "container is not running",
                vec![format!(
                    "state={:?}, exit_code={:?}",
                    observation.inspection.runtime_state, observation.inspection.exit_code
                )],
            ),
            _ => push_signal(
                &mut signals,
                ErrorKind::Unknown,
                SignalSeverity::Warning,
                "container state needs review",
                vec![format!("{:?}", observation.inspection.runtime_state)],
            ),
        }

        if observation.inspection.oom_killed {
            push_signal(
                &mut signals,
                ErrorKind::OomKilled,
                SignalSeverity::Critical,
                "container was killed by the OOM killer",
                vec!["State.OOMKilled=true".to_string()],
            );
        }

        if matches!(
            observation
                .inspection
                .docker_health
                .as_deref()
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("unhealthy")
        ) {
            push_signal(
                &mut signals,
                ErrorKind::UnhealthyDockerStatus,
                SignalSeverity::Critical,
                "docker healthcheck reports unhealthy",
                vec!["State.Health.Status=unhealthy".to_string()],
            );
        }

        signals
    }
}

impl HealthCheck for ResourceHealthCheck {
    fn name(&self) -> &'static str {
        "resource-pressure"
    }

    fn inspect(
        &self,
        observation: &ContainerObservation,
        config: &RecoveryAgentConfig,
    ) -> Vec<ErrorSignal> {
        let mut signals = Vec::new();
        let Some(memory_percent) = observation.resources.memory_percent else {
            return signals;
        };

        if memory_percent >= config.memory_critical_percent {
            push_signal(
                &mut signals,
                ErrorKind::MemoryLeak,
                SignalSeverity::Critical,
                "container memory usage is critical",
                vec![format!("memory={memory_percent:.1}%")],
            );
        } else if memory_percent >= config.memory_warning_percent {
            push_signal(
                &mut signals,
                ErrorKind::MemoryLeak,
                SignalSeverity::Warning,
                "container memory usage is high",
                vec![format!("memory={memory_percent:.1}%")],
            );
        }

        signals
    }
}

impl HealthCheck for EndpointHealthCheck {
    fn name(&self) -> &'static str {
        "endpoint"
    }

    fn inspect(
        &self,
        observation: &ContainerObservation,
        _config: &RecoveryAgentConfig,
    ) -> Vec<ErrorSignal> {
        let mut signals = Vec::new();
        let Some(probe) = &observation.endpoint_probe else {
            return signals;
        };

        match probe.status_code {
            Some(500) => push_signal(
                &mut signals,
                ErrorKind::Http500,
                SignalSeverity::Critical,
                "endpoint returned HTTP 500",
                vec![probe.url.clone()],
            ),
            Some(status) if status >= 500 => push_signal(
                &mut signals,
                ErrorKind::ApiFailure,
                SignalSeverity::Critical,
                "endpoint returned a server error",
                vec![format!("{} -> HTTP {}", probe.url, status)],
            ),
            _ => {}
        }

        if let Some(error) = &probe.error {
            let lower = error.to_ascii_lowercase();
            let kind = if lower.contains("connection refused") || lower.contains("tcp connect") {
                ErrorKind::PortFailure
            } else if lower.contains("timed out") {
                ErrorKind::Timeout
            } else {
                ErrorKind::NetworkFailure
            };
            push_signal(
                &mut signals,
                kind,
                SignalSeverity::Warning,
                "endpoint probe failed",
                vec![error.clone()],
            );
        }

        signals
    }
}

impl HealthCheck for LogPatternHealthCheck {
    fn name(&self) -> &'static str {
        "log-patterns"
    }

    fn inspect(
        &self,
        observation: &ContainerObservation,
        _config: &RecoveryAgentConfig,
    ) -> Vec<ErrorSignal> {
        detect_error_signals(&observation.logs)
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryPolicy {
    pub priority: Vec<RecoveryAction>,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            priority: vec![
                RecoveryAction::RetryRequest,
                RecoveryAction::RestartApplicationProcess,
                RecoveryAction::RestartWorker,
                RecoveryAction::ReloadConfiguration,
                RecoveryAction::RestartService,
                RecoveryAction::RestartContainer,
                RecoveryAction::Escalate,
            ],
        }
    }
}

pub fn default_recovery_policy() -> RecoveryPolicy {
    RecoveryPolicy::default()
}

#[derive(Debug, Clone)]
pub struct LocalAgent {
    pub agent_id: String,
    pub container: ContainerIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAgentTick {
    pub heartbeat: Heartbeat,
    pub recovery: RecoveryReport,
    pub audit_log: Vec<AgentLogEntry>,
}

impl LocalAgent {
    fn from_record(record: &LocalAgentRecord, container: ContainerIdentity) -> Self {
        Self {
            agent_id: record.agent_id.clone(),
            container,
        }
    }

    pub async fn tick<R: ContainerRuntime>(
        &self,
        runtime: &R,
        config: &RecoveryAgentConfig,
        health_checks: &[Box<dyn HealthCheck>],
        policy: &RecoveryPolicy,
        previous_recovery_attempts: usize,
    ) -> Result<LocalAgentTick> {
        let observation = collect_observation(runtime, config, &self.container).await?;
        let errors = inspect_observation(&observation, config, health_checks);
        let status = classify_health_status(&observation, &errors);
        let mut audit_log = Vec::new();

        let recovery = if errors.is_empty() {
            RecoveryReport::not_needed()
        } else if previous_recovery_attempts >= config.max_recovery_attempts {
            let root_cause = determine_root_cause(&errors, &observation);
            let escalation = build_escalation(
                &self.container,
                &root_cause,
                Vec::new(),
                &observation.logs,
                "recovery retry limit exceeded before another automatic action".to_string(),
            );
            RecoveryReport {
                status: RecoveryStatus::Escalated,
                root_cause: Some(root_cause),
                attempts: Vec::new(),
                verification: format!(
                    "recovery attempted {} time(s), limit is {}",
                    previous_recovery_attempts, config.max_recovery_attempts
                ),
                escalation: Some(escalation),
            }
        } else {
            attempt_recovery(
                runtime,
                config,
                policy,
                &self.container,
                &observation,
                &errors,
            )
            .await?
        };

        if recovery.status != RecoveryStatus::NotNeeded {
            audit_log.push(build_audit_entry(&self.container, &errors, &recovery));
        }

        let heartbeat = Heartbeat {
            agent_id: self.agent_id.clone(),
            container_id: self.container.id.clone(),
            container_name: self.container.name.clone(),
            status: match recovery.status {
                RecoveryStatus::Escalated => AgentHealthStatus::Escalated,
                RecoveryStatus::Recovered => AgentHealthStatus::Healthy,
                RecoveryStatus::NotNeeded | RecoveryStatus::Planned | RecoveryStatus::Failed => {
                    status
                }
            },
            cpu_percent: observation.resources.cpu_percent,
            ram_percent: observation.resources.memory_percent,
            response_time_ms: observation
                .endpoint_probe
                .as_ref()
                .map(|probe| probe.response_time_ms),
            running_services: infer_running_services(&observation),
            current_errors: errors,
            recovery_status: recovery.status,
            timestamp: now_timestamp(),
        };

        Ok(LocalAgentTick {
            heartbeat,
            recovery,
            audit_log,
        })
    }
}

pub struct MasterAgent<R: ContainerRuntime> {
    runtime: R,
    config: RecoveryAgentConfig,
    registry: AgentRegistry,
    health_checks: Vec<Box<dyn HealthCheck>>,
    policy: RecoveryPolicy,
    audit_log: Vec<AgentLogEntry>,
}

impl<R: ContainerRuntime> MasterAgent<R> {
    pub fn new(runtime: R, config: RecoveryAgentConfig) -> Self {
        Self {
            runtime,
            config,
            registry: AgentRegistry::default(),
            health_checks: default_health_checks(),
            policy: RecoveryPolicy::default(),
            audit_log: Vec::new(),
        }
    }

    pub fn with_health_check(mut self, check: Box<dyn HealthCheck>) -> Self {
        self.health_checks.push(check);
        self
    }

    pub fn with_recovery_policy(mut self, policy: RecoveryPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    pub fn config(&self) -> &RecoveryAgentConfig {
        &self.config
    }

    pub async fn run_once(&mut self) -> Result<MasterAgentReport> {
        let containers = self.runtime.list_active_containers().await?;
        let active_ids = containers
            .iter()
            .map(|container| container.id.clone())
            .collect::<BTreeSet<_>>();

        for container in &containers {
            let created = self.registry.ensure_local_agent(container);
            if created {
                tracing::info!(
                    container = %container.name,
                    agent = %format!("local-agent-{}", container.short_id()),
                    "initialized local container agent"
                );
            }
        }

        self.registry.deactivate_missing(&active_ids);

        let mut heartbeats = Vec::new();
        let mut escalations = Vec::new();

        for container in containers {
            let previous_attempts = self.registry.recovery_attempt_count(&container.id);
            let record = self
                .registry
                .agents
                .get(&container.id)
                .cloned()
                .with_context(|| format!("missing local agent for {}", container.name))?;
            let local_agent = LocalAgent::from_record(&record, container.clone());
            let tick = local_agent
                .tick(
                    &self.runtime,
                    &self.config,
                    &self.health_checks,
                    &self.policy,
                    previous_attempts,
                )
                .await?;

            if let Some(escalation) = tick.recovery.escalation.clone() {
                escalations.push(escalation);
            }

            self.registry.record_heartbeat(tick.heartbeat.clone());
            self.registry
                .record_recovery_cycle(&container.id, &tick.recovery);
            self.audit_log.extend(tick.audit_log);
            heartbeats.push(tick.heartbeat);
        }

        let status = classify_infrastructure_status(&heartbeats, &escalations);

        Ok(MasterAgentReport {
            status,
            registry: self.registry.clone(),
            heartbeats,
            escalations,
            audit_log: self.audit_log.clone(),
        })
    }
}

pub async fn run_recovery_supervision_once(
    config: RecoveryAgentConfig,
) -> Result<MasterAgentReport> {
    let runtime = DockerCliRuntime::new();
    let mut master = MasterAgent::new(runtime, config);
    master.run_once().await
}

pub fn default_health_checks() -> Vec<Box<dyn HealthCheck>> {
    vec![
        Box::new(DockerStateHealthCheck),
        Box::new(ResourceHealthCheck),
        Box::new(EndpointHealthCheck),
        Box::new(LogPatternHealthCheck),
    ]
}

pub fn inspect_observation(
    observation: &ContainerObservation,
    config: &RecoveryAgentConfig,
    health_checks: &[Box<dyn HealthCheck>],
) -> Vec<ErrorSignal> {
    let mut signals = Vec::new();
    for check in health_checks {
        for signal in check.inspect(observation, config) {
            push_signal_owned(&mut signals, signal);
        }
    }
    signals
}

pub fn detect_error_signals(logs: &str) -> Vec<ErrorSignal> {
    let lower = logs.to_ascii_lowercase();
    let mut signals = Vec::new();

    let patterns = [
        (
            ErrorKind::Http500,
            SignalSeverity::Critical,
            ["http 500", "status=500", " 500 ", "500 internal"].as_slice(),
            "HTTP 500 detected in application logs",
        ),
        (
            ErrorKind::InternalServerError,
            SignalSeverity::Critical,
            ["internal server error"].as_slice(),
            "internal server error detected",
        ),
        (
            ErrorKind::RuntimeException,
            SignalSeverity::Critical,
            [
                "traceback",
                "exception",
                "panic",
                "unhandledpromiserejection",
                "uncaught exception",
                "fatal error",
                "stacktrace",
            ]
            .as_slice(),
            "runtime exception detected",
        ),
        (
            ErrorKind::Crash,
            SignalSeverity::Critical,
            [
                "node crashed",
                "process exited",
                "worker crashed",
                "crashed",
            ]
            .as_slice(),
            "application process crash detected",
        ),
        (
            ErrorKind::OomKilled,
            SignalSeverity::Critical,
            [
                "oomkilled",
                "outofmemory",
                "out of memory",
                "memory allocation failed",
            ]
            .as_slice(),
            "out-of-memory condition detected",
        ),
        (
            ErrorKind::SegmentationFault,
            SignalSeverity::Critical,
            ["segmentation fault", "sigsegv"].as_slice(),
            "segmentation fault detected",
        ),
        (
            ErrorKind::DependencyFailure,
            SignalSeverity::Critical,
            [
                "modulenotfounderror",
                "cannot find module",
                "classnotfoundexception",
                "no module named",
                "failed to load dependency",
            ]
            .as_slice(),
            "dependency failure detected",
        ),
        (
            ErrorKind::MissingEnvironmentVariable,
            SignalSeverity::Critical,
            [
                "missing environment variable",
                "environment variable is required",
                "env var",
                "undefined environment",
            ]
            .as_slice(),
            "missing environment variable detected",
        ),
        (
            ErrorKind::DatabaseConnectionFailure,
            SignalSeverity::Critical,
            [
                "database connection",
                "db connection",
                "connection refused",
                "database timeout",
                "could not connect to database",
                "sqlstate",
                "postgres",
                "mysql",
            ]
            .as_slice(),
            "database connectivity failure detected",
        ),
        (
            ErrorKind::RedisFailure,
            SignalSeverity::Critical,
            ["redis connection", "redis timeout", "redis unavailable"].as_slice(),
            "redis connectivity failure detected",
        ),
        (
            ErrorKind::ApiFailure,
            SignalSeverity::Warning,
            ["upstream api", "external api", "api failure", "bad gateway"].as_slice(),
            "external API failure detected",
        ),
        (
            ErrorKind::Timeout,
            SignalSeverity::Warning,
            ["timeout", "timed out", "deadline exceeded", "etimedout"].as_slice(),
            "timeout detected",
        ),
    ];

    for (kind, severity, needles, message) in patterns {
        if let Some(evidence) = first_matching_line(logs, &lower, needles) {
            push_signal(
                &mut signals,
                kind,
                severity,
                message,
                vec![truncate_evidence(&evidence)],
            );
        }
    }

    signals
}

pub fn determine_root_cause(
    signals: &[ErrorSignal],
    observation: &ContainerObservation,
) -> RootCause {
    let mut evidence = signals
        .iter()
        .flat_map(|signal| signal.evidence.iter().cloned())
        .take(5)
        .collect::<Vec<_>>();

    if let Some(error) = &observation.inspection.error {
        if !error.trim().is_empty() {
            evidence.push(error.clone());
        }
    }

    if signals
        .iter()
        .any(|signal| signal.kind == ErrorKind::DatabaseConnectionFailure)
    {
        return RootCause {
            category: RootCauseCategory::DatabaseUnavailable,
            summary: "database connectivity or timeout failure".to_string(),
            confidence: 0.86,
            evidence,
        };
    }

    if signals
        .iter()
        .any(|signal| signal.kind == ErrorKind::RedisFailure)
    {
        return RootCause {
            category: RootCauseCategory::RedisUnavailable,
            summary: "redis connectivity failure".to_string(),
            confidence: 0.84,
            evidence,
        };
    }

    if signals
        .iter()
        .any(|signal| signal.kind == ErrorKind::MissingEnvironmentVariable)
    {
        return RootCause {
            category: RootCauseCategory::MissingConfiguration,
            summary: "required runtime configuration is missing".to_string(),
            confidence: 0.82,
            evidence,
        };
    }

    if signals
        .iter()
        .any(|signal| signal.kind == ErrorKind::DependencyFailure)
    {
        return RootCause {
            category: RootCauseCategory::DependencyMissing,
            summary: "runtime dependency is missing or cannot be loaded".to_string(),
            confidence: 0.8,
            evidence,
        };
    }

    if signals
        .iter()
        .any(|signal| matches!(signal.kind, ErrorKind::OomKilled | ErrorKind::MemoryLeak))
    {
        return RootCause {
            category: RootCauseCategory::ResourceExhaustion,
            summary: "container is under memory pressure or was OOM-killed".to_string(),
            confidence: 0.88,
            evidence,
        };
    }

    if signals.iter().any(|signal| signal.kind == ErrorKind::Crash) {
        return RootCause {
            category: RootCauseCategory::ProcessCrash,
            summary: "application process crashed or exited unexpectedly".to_string(),
            confidence: 0.78,
            evidence,
        };
    }

    if signals
        .iter()
        .any(|signal| matches!(signal.kind, ErrorKind::NetworkFailure | ErrorKind::Timeout))
    {
        return RootCause {
            category: RootCauseCategory::NetworkFailure,
            summary: "network probe or dependency timed out".to_string(),
            confidence: 0.7,
            evidence,
        };
    }

    if signals.iter().any(|signal| {
        matches!(
            signal.kind,
            ErrorKind::Http500 | ErrorKind::InternalServerError
        )
    }) {
        return RootCause {
            category: RootCauseCategory::ApplicationException,
            summary: "application returned an internal server error".to_string(),
            confidence: 0.72,
            evidence,
        };
    }

    RootCause {
        category: RootCauseCategory::Unknown,
        summary: "root cause could not be determined from current signals".to_string(),
        confidence: 0.35,
        evidence,
    }
}

async fn collect_observation<R: ContainerRuntime>(
    runtime: &R,
    config: &RecoveryAgentConfig,
    container: &ContainerIdentity,
) -> Result<ContainerObservation> {
    let inspection = runtime.inspect_container(&container.id).await?;
    let logs = runtime
        .container_logs(&container.id, config.log_tail)
        .await
        .unwrap_or_else(|error| format!("failed to collect logs: {error}"));
    let resources = runtime
        .container_stats(&container.id)
        .await
        .unwrap_or_default();
    let endpoint_url = infer_endpoint_url(&container.ports, &config.health_path);
    let endpoint_probe = match endpoint_url {
        Some(url) => Some(
            runtime
                .probe_endpoint(&url, config.response_timeout)
                .await?,
        ),
        None => None,
    };

    Ok(ContainerObservation {
        container: container.clone(),
        inspection,
        logs,
        resources,
        endpoint_probe,
        observed_at: now_timestamp(),
    })
}

async fn attempt_recovery<R: ContainerRuntime>(
    runtime: &R,
    config: &RecoveryAgentConfig,
    policy: &RecoveryPolicy,
    container: &ContainerIdentity,
    observation: &ContainerObservation,
    errors: &[ErrorSignal],
) -> Result<RecoveryReport> {
    let root_cause = determine_root_cause(errors, observation);

    if config.dry_run {
        let attempts = policy
            .priority
            .iter()
            .filter(|action| **action != RecoveryAction::Escalate)
            .map(|action| RecoveryAttempt {
                action: *action,
                status: RecoveryAttemptStatus::Planned,
                detail: "dry run: action planned but not executed".to_string(),
                duration_ms: 0,
            })
            .collect::<Vec<_>>();

        return Ok(RecoveryReport {
            status: RecoveryStatus::Planned,
            root_cause: Some(root_cause),
            attempts,
            verification: "dry run mode; no recovery action was executed".to_string(),
            escalation: None,
        });
    }

    let mut attempts = Vec::new();

    for action in policy.priority.iter().copied() {
        if action == RecoveryAction::Escalate {
            break;
        }

        let attempt =
            execute_recovery_action(runtime, config, container, observation, action).await?;
        let action_succeeded = attempt.status == RecoveryAttemptStatus::Succeeded;
        attempts.push(attempt);

        if action_succeeded {
            let verified = verify_recovery(runtime, config, container)
                .await
                .unwrap_or(false);
            if verified {
                return Ok(RecoveryReport {
                    status: RecoveryStatus::Recovered,
                    root_cause: Some(root_cause),
                    attempts,
                    verification: "container returned to a healthy state after recovery"
                        .to_string(),
                    escalation: None,
                });
            }
        }
    }

    let escalation = build_escalation(
        container,
        &root_cause,
        attempts.clone(),
        &observation.logs,
        suggested_resolution(&root_cause),
    );

    Ok(RecoveryReport {
        status: RecoveryStatus::Escalated,
        root_cause: Some(root_cause),
        attempts,
        verification: "automatic recovery did not restore health".to_string(),
        escalation: Some(escalation),
    })
}

async fn execute_recovery_action<R: ContainerRuntime>(
    runtime: &R,
    config: &RecoveryAgentConfig,
    container: &ContainerIdentity,
    observation: &ContainerObservation,
    action: RecoveryAction,
) -> Result<RecoveryAttempt> {
    let start = Instant::now();
    let result = match action {
        RecoveryAction::RetryRequest => {
            if let Some(probe) = &observation.endpoint_probe {
                let retry = runtime
                    .probe_endpoint(&probe.url, config.response_timeout)
                    .await?;
                if retry.healthy() {
                    RecoveryAttempt {
                        action,
                        status: RecoveryAttemptStatus::Succeeded,
                        detail: format!("retry succeeded for {}", retry.url),
                        duration_ms: start.elapsed().as_millis() as u64,
                    }
                } else {
                    RecoveryAttempt {
                        action,
                        status: RecoveryAttemptStatus::Failed,
                        detail: format!(
                            "retry failed for {} with status {:?}: {:?}",
                            retry.url, retry.status_code, retry.error
                        ),
                        duration_ms: start.elapsed().as_millis() as u64,
                    }
                }
            } else {
                RecoveryAttempt {
                    action,
                    status: RecoveryAttemptStatus::Skipped,
                    detail: "no published endpoint was available to retry".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
        }
        RecoveryAction::RestartApplicationProcess => {
            execute_configured_container_command(
                runtime,
                container,
                action,
                restart_command(
                    observation,
                    config,
                    "optidock.agent.restart-command",
                    &config.app_restart_command,
                ),
                "no application restart command configured",
                start,
            )
            .await?
        }
        RecoveryAction::RestartWorker => {
            execute_configured_container_command(
                runtime,
                container,
                action,
                restart_command(
                    observation,
                    config,
                    "optidock.agent.worker-restart-command",
                    &config.worker_restart_command,
                ),
                "no worker restart command configured",
                start,
            )
            .await?
        }
        RecoveryAction::ReloadConfiguration => {
            execute_configured_container_command(
                runtime,
                container,
                action,
                restart_command(
                    observation,
                    config,
                    "optidock.agent.reload-command",
                    &config.reload_command,
                ),
                "no reload command configured",
                start,
            )
            .await?
        }
        RecoveryAction::RestartService => {
            execute_configured_container_command(
                runtime,
                container,
                action,
                restart_command(
                    observation,
                    config,
                    "optidock.agent.service-restart-command",
                    &config.service_restart_command,
                ),
                "no service restart command configured",
                start,
            )
            .await?
        }
        RecoveryAction::RestartContainer => {
            if !config.allow_container_restart {
                RecoveryAttempt {
                    action,
                    status: RecoveryAttemptStatus::Skipped,
                    detail: "container restart disabled; set --restart-containers or OPTIDOCK_AGENT_ALLOW_CONTAINER_RESTART=true".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            } else {
                let command_result = runtime.restart_container(&container.id).await?;
                RecoveryAttempt {
                    action,
                    status: if command_result.success {
                        RecoveryAttemptStatus::Succeeded
                    } else {
                        RecoveryAttemptStatus::Failed
                    },
                    detail: command_detail(command_result),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
        }
        RecoveryAction::Escalate => RecoveryAttempt {
            action,
            status: RecoveryAttemptStatus::Skipped,
            detail: "escalation is handled after recovery actions fail".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        },
    };

    Ok(result)
}

async fn execute_configured_container_command<R: ContainerRuntime>(
    runtime: &R,
    container: &ContainerIdentity,
    action: RecoveryAction,
    command: Option<String>,
    missing_detail: &str,
    start: Instant,
) -> Result<RecoveryAttempt> {
    let Some(command) = command else {
        return Ok(RecoveryAttempt {
            action,
            status: RecoveryAttemptStatus::Skipped,
            detail: missing_detail.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    };

    let args = ["sh", "-lc", command.as_str()];
    let command_result = runtime.exec_container(&container.id, &args).await?;
    Ok(RecoveryAttempt {
        action,
        status: if command_result.success {
            RecoveryAttemptStatus::Succeeded
        } else {
            RecoveryAttemptStatus::Failed
        },
        detail: command_detail(command_result),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn verify_recovery<R: ContainerRuntime>(
    runtime: &R,
    config: &RecoveryAgentConfig,
    container: &ContainerIdentity,
) -> Result<bool> {
    tokio::time::sleep(Duration::from_millis(500)).await;
    let observation = collect_observation(runtime, config, container).await?;
    let signals = inspect_observation(&observation, config, &default_health_checks());
    Ok(signals.is_empty())
}

fn restart_command(
    observation: &ContainerObservation,
    _config: &RecoveryAgentConfig,
    label: &str,
    fallback: &Option<String>,
) -> Option<String> {
    observation
        .inspection
        .labels
        .get(label)
        .cloned()
        .or_else(|| fallback.clone())
}

fn build_escalation(
    container: &ContainerIdentity,
    root_cause: &RootCause,
    attempts: Vec<RecoveryAttempt>,
    logs: &str,
    suggested_resolution: String,
) -> EscalationReport {
    EscalationReport {
        container_id: container.id.clone(),
        container_name: container.name.clone(),
        root_cause: root_cause.clone(),
        recovery_attempts: attempts,
        logs: logs
            .lines()
            .rev()
            .take(20)
            .map(str::to_string)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
        suggested_resolution,
        confidence_score: root_cause.confidence,
    }
}

fn build_audit_entry(
    container: &ContainerIdentity,
    errors: &[ErrorSignal],
    recovery: &RecoveryReport,
) -> AgentLogEntry {
    let started = Instant::now();
    let issue = errors
        .first()
        .map(|signal| signal.message.clone())
        .unwrap_or_else(|| "unknown issue".to_string());
    let root_cause = recovery
        .root_cause
        .as_ref()
        .map(|cause| cause.summary.clone())
        .unwrap_or_else(|| "not determined".to_string());
    let recovery_summary = recovery
        .attempts
        .iter()
        .map(|attempt| format!("{:?}:{:?}", attempt.action, attempt.status))
        .collect::<Vec<_>>()
        .join(", ");

    AgentLogEntry {
        level: if recovery.status == RecoveryStatus::Escalated {
            "WARN".to_string()
        } else {
            "INFO".to_string()
        },
        container: container.name.clone(),
        issue,
        root_cause,
        recovery: if recovery_summary.is_empty() {
            recovery.verification.clone()
        } else {
            recovery_summary
        },
        verification: recovery.verification.clone(),
        duration_ms: started.elapsed().as_millis() as u64,
        status: recovery.status,
        timestamp: now_timestamp(),
    }
}

fn classify_health_status(
    observation: &ContainerObservation,
    errors: &[ErrorSignal],
) -> AgentHealthStatus {
    if matches!(
        observation.inspection.runtime_state,
        ContainerRuntimeState::Exited | ContainerRuntimeState::Dead
    ) {
        return AgentHealthStatus::Crashed;
    }

    if errors
        .iter()
        .any(|signal| signal.severity == SignalSeverity::Critical)
    {
        AgentHealthStatus::Unhealthy
    } else if errors.is_empty() {
        AgentHealthStatus::Healthy
    } else {
        AgentHealthStatus::Degraded
    }
}

fn classify_infrastructure_status(
    heartbeats: &[Heartbeat],
    escalations: &[EscalationReport],
) -> AgentHealthStatus {
    if !escalations.is_empty() {
        return AgentHealthStatus::Escalated;
    }

    if heartbeats.iter().any(|heartbeat| {
        matches!(
            heartbeat.status,
            AgentHealthStatus::Crashed | AgentHealthStatus::Unhealthy
        )
    }) {
        AgentHealthStatus::Unhealthy
    } else if heartbeats
        .iter()
        .any(|heartbeat| heartbeat.status == AgentHealthStatus::Degraded)
    {
        AgentHealthStatus::Degraded
    } else {
        AgentHealthStatus::Healthy
    }
}

fn infer_running_services(observation: &ContainerObservation) -> Vec<String> {
    let mut services = vec![observation.container.image.clone()];
    if !observation.container.ports.trim().is_empty() {
        services.push(format!("ports: {}", observation.container.ports));
    }
    if let Some(health) = &observation.inspection.docker_health {
        services.push(format!("docker-health: {health}"));
    }
    services
}

fn suggested_resolution(root_cause: &RootCause) -> String {
    match root_cause.category {
        RootCauseCategory::DatabaseUnavailable => {
            "check database availability, credentials, connection pool limits, and network policy".to_string()
        }
        RootCauseCategory::RedisUnavailable => {
            "check Redis availability, credentials, DNS, and network reachability".to_string()
        }
        RootCauseCategory::MissingConfiguration => {
            "provide the missing environment variables or secrets, then restart the service".to_string()
        }
        RootCauseCategory::DependencyMissing => {
            "rebuild the image with the missing runtime dependency or restore the dependency volume".to_string()
        }
        RootCauseCategory::ResourceExhaustion => {
            "inspect memory growth, increase limits if appropriate, and capture a heap/profile dump".to_string()
        }
        RootCauseCategory::ProcessCrash => {
            "inspect the crash stack trace and validate startup commands and process manager configuration".to_string()
        }
        RootCauseCategory::NetworkFailure => {
            "verify DNS, upstream service health, firewall rules, and timeout budgets".to_string()
        }
        RootCauseCategory::PortFailure => {
            "verify published ports, application bind address, and health endpoint configuration".to_string()
        }
        RootCauseCategory::ApplicationException => {
            "inspect the stack trace, failing module, dependencies, environment variables, database, and external APIs".to_string()
        }
        RootCauseCategory::Unknown => {
            "review recent logs and container inspect data; automatic diagnosis did not find a confident cause".to_string()
        }
    }
}

fn infer_endpoint_url(ports: &str, health_path: &str) -> Option<String> {
    for part in ports.split(',') {
        let left = part.split("->").next()?.trim();
        let host_port = left.rsplit(':').next()?.trim();
        if host_port.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(format!("http://127.0.0.1:{host_port}{health_path}"));
        }
    }
    None
}

fn normalize_health_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn push_signal(
    signals: &mut Vec<ErrorSignal>,
    kind: ErrorKind,
    severity: SignalSeverity,
    message: &str,
    evidence: Vec<String>,
) {
    push_signal_owned(
        signals,
        ErrorSignal {
            kind,
            severity,
            message: message.to_string(),
            evidence,
        },
    );
}

fn push_signal_owned(signals: &mut Vec<ErrorSignal>, signal: ErrorSignal) {
    if let Some(existing) = signals
        .iter_mut()
        .find(|existing| existing.kind == signal.kind)
    {
        if signal.severity > existing.severity {
            existing.severity = signal.severity;
        }
        existing.evidence.extend(signal.evidence);
        existing.evidence.sort();
        existing.evidence.dedup();
        return;
    }

    signals.push(signal);
}

fn first_matching_line(logs: &str, lower_logs: &str, needles: &[&str]) -> Option<String> {
    if !needles.iter().any(|needle| lower_logs.contains(needle)) {
        return None;
    }

    logs.lines()
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            needles.iter().any(|needle| lower.contains(needle))
        })
        .map(str::to_string)
}

fn truncate_evidence(value: &str) -> String {
    const MAX: usize = 240;
    if value.chars().count() <= MAX {
        return value.to_string();
    }

    let mut truncated = value.chars().take(MAX).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn command_detail(result: CommandResult) -> String {
    if result.success {
        if result.stdout.trim().is_empty() {
            "command succeeded".to_string()
        } else {
            truncate_evidence(result.stdout.trim())
        }
    } else if !result.stderr.trim().is_empty() {
        truncate_evidence(result.stderr.trim())
    } else {
        "command failed".to_string()
    }
}

async fn docker_command<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let result = docker_command_result(args).await?;
    if result.success {
        Ok(result.stdout)
    } else {
        anyhow::bail!("docker command failed: {}", result.stderr)
    }
}

async fn docker_command_result<I, S>(args: I) -> Result<CommandResult>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .await
        .context("failed to execute docker")?;

    Ok(CommandResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn parse_inspection(value: &Value) -> ContainerInspection {
    let id = value_string(value, "/Id").unwrap_or_else(|| "unknown".to_string());
    let name = value_string(value, "/Name")
        .unwrap_or_else(|| "unknown".to_string())
        .trim_start_matches('/')
        .to_string();
    let image = value_string(value, "/Config/Image")
        .or_else(|| value_string(value, "/Image"))
        .unwrap_or_else(|| "unknown".to_string());
    let state = value_string(value, "/State/Status").unwrap_or_else(|| "unknown".to_string());
    let status = state.clone();
    let docker_health = value_string(value, "/State/Health/Status");
    let exit_code = value.pointer("/State/ExitCode").and_then(Value::as_i64);
    let error = value_string(value, "/State/Error").filter(|item| !item.trim().is_empty());
    let oom_killed = value
        .pointer("/State/OOMKilled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let restart_count = value
        .pointer("/RestartCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let env = value
        .pointer("/Config/Env")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let labels = value
        .pointer("/Config/Labels")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    ContainerInspection {
        identity: ContainerIdentity {
            id,
            name,
            image,
            status,
            ports: String::new(),
        },
        runtime_state: ContainerRuntimeState::from_docker_status(&state),
        docker_health,
        exit_code,
        error,
        oom_killed,
        restart_count,
        env,
        labels,
    }
}

fn value_string(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_percent(value: &Option<String>) -> Option<f64> {
    value
        .as_deref()
        .map(|value| value.trim().trim_end_matches('%'))
        .and_then(|value| value.parse::<f64>().ok())
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn now_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn detects_http_500_and_database_root_cause() {
        let logs = "GET /api/session HTTP 500\nInternal Server Error\nDatabase timeout while opening connection";
        let signals = detect_error_signals(logs);

        assert!(signals
            .iter()
            .any(|signal| signal.kind == ErrorKind::Http500));
        assert!(signals
            .iter()
            .any(|signal| signal.kind == ErrorKind::DatabaseConnectionFailure));

        let observation = observation_with_logs(logs);
        let cause = determine_root_cause(&signals, &observation);
        assert_eq!(cause.category, RootCauseCategory::DatabaseUnavailable);
        assert!(cause.confidence > 0.8);
    }

    #[test]
    fn recovery_policy_preserves_required_priority_order() {
        let policy = default_recovery_policy();

        assert_eq!(
            policy.priority,
            vec![
                RecoveryAction::RetryRequest,
                RecoveryAction::RestartApplicationProcess,
                RecoveryAction::RestartWorker,
                RecoveryAction::ReloadConfiguration,
                RecoveryAction::RestartService,
                RecoveryAction::RestartContainer,
                RecoveryAction::Escalate,
            ]
        );
    }

    #[test]
    fn registry_prevents_duplicate_local_agents() {
        let container = sample_container();
        let mut registry = AgentRegistry::default();

        assert!(registry.ensure_local_agent(&container));
        assert!(!registry.ensure_local_agent(&container));
        assert_eq!(registry.local_agent_count_for(&container.id), 1);
    }

    #[test]
    fn infers_endpoint_from_published_port() {
        let endpoint = infer_endpoint_url("0.0.0.0:8080->8080/tcp", "/health");
        assert_eq!(endpoint.as_deref(), Some("http://127.0.0.1:8080/health"));
    }

    #[tokio::test]
    async fn master_agent_discovers_container_and_creates_heartbeat() {
        let runtime = MemoryRuntime::healthy();
        let mut master = MasterAgent::new(runtime, RecoveryAgentConfig::default());

        let report = master.run_once().await.unwrap();

        assert_eq!(report.heartbeats.len(), 1);
        assert_eq!(report.registry.local_agent_count_for("abc123"), 1);
        assert_eq!(report.status, AgentHealthStatus::Healthy);
    }

    #[tokio::test]
    async fn local_agent_plans_recovery_for_internal_server_error_in_dry_run() {
        let runtime = MemoryRuntime::with_logs(
            "HTTP 500 Internal Server Error\nTraceback: database connection timeout",
        );
        let mut master = MasterAgent::new(runtime, RecoveryAgentConfig::default());

        let report = master.run_once().await.unwrap();

        assert_eq!(
            report.heartbeats[0].recovery_status,
            RecoveryStatus::Planned
        );
        assert!(report.audit_log[0]
            .recovery
            .contains("RetryRequest:Planned"));
    }

    fn sample_container() -> ContainerIdentity {
        ContainerIdentity {
            id: "abc123".to_string(),
            name: "api".to_string(),
            image: "example/api:latest".to_string(),
            status: "Up 2 minutes".to_string(),
            ports: String::new(),
        }
    }

    fn sample_inspection(logs_state: ContainerRuntimeState) -> ContainerInspection {
        ContainerInspection {
            identity: sample_container(),
            runtime_state: logs_state,
            docker_health: Some("healthy".to_string()),
            exit_code: Some(0),
            error: None,
            oom_killed: false,
            restart_count: 0,
            env: Vec::new(),
            labels: BTreeMap::new(),
        }
    }

    fn observation_with_logs(logs: &str) -> ContainerObservation {
        ContainerObservation {
            container: sample_container(),
            inspection: sample_inspection(ContainerRuntimeState::Running),
            logs: logs.to_string(),
            resources: ResourceSnapshot::default(),
            endpoint_probe: None,
            observed_at: now_timestamp(),
        }
    }

    #[derive(Clone)]
    struct MemoryRuntime {
        logs: Arc<Mutex<String>>,
        probe: Option<EndpointProbe>,
    }

    impl MemoryRuntime {
        fn healthy() -> Self {
            Self {
                logs: Arc::new(Mutex::new(String::new())),
                probe: None,
            }
        }

        fn with_logs(logs: &str) -> Self {
            Self {
                logs: Arc::new(Mutex::new(logs.to_string())),
                probe: None,
            }
        }
    }

    impl ContainerRuntime for MemoryRuntime {
        fn list_active_containers<'a>(&'a self) -> BoxFuture<'a, Result<Vec<ContainerIdentity>>> {
            Box::pin(async move { Ok(vec![sample_container()]) })
        }

        fn inspect_container<'a>(
            &'a self,
            _container_id: &'a str,
        ) -> BoxFuture<'a, Result<ContainerInspection>> {
            Box::pin(async move { Ok(sample_inspection(ContainerRuntimeState::Running)) })
        }

        fn container_logs<'a>(
            &'a self,
            _container_id: &'a str,
            _tail: usize,
        ) -> BoxFuture<'a, Result<String>> {
            Box::pin(async move { Ok(self.logs.lock().unwrap().clone()) })
        }

        fn container_stats<'a>(
            &'a self,
            _container_id: &'a str,
        ) -> BoxFuture<'a, Result<ResourceSnapshot>> {
            Box::pin(async move { Ok(ResourceSnapshot::default()) })
        }

        fn exec_container<'a>(
            &'a self,
            _container_id: &'a str,
            _command: &'a [&'a str],
        ) -> BoxFuture<'a, Result<CommandResult>> {
            Box::pin(async move {
                Ok(CommandResult {
                    success: true,
                    stdout: "ok".to_string(),
                    stderr: String::new(),
                })
            })
        }

        fn restart_container<'a>(
            &'a self,
            _container_id: &'a str,
        ) -> BoxFuture<'a, Result<CommandResult>> {
            Box::pin(async move {
                Ok(CommandResult {
                    success: true,
                    stdout: "restarted".to_string(),
                    stderr: String::new(),
                })
            })
        }

        fn probe_endpoint<'a>(
            &'a self,
            _url: &'a str,
            _timeout: Duration,
        ) -> BoxFuture<'a, Result<EndpointProbe>> {
            Box::pin(async move {
                Ok(self.probe.clone().unwrap_or(EndpointProbe {
                    url: "http://127.0.0.1:8080/".to_string(),
                    status_code: Some(200),
                    response_time_ms: 1,
                    error: None,
                }))
            })
        }
    }
}
