use anyhow::Result;
use optidock_agent::{
    AgentHealthStatus, BoxFuture, CommandResult, ContainerIdentity, ContainerInspection,
    ContainerRuntime, ContainerRuntimeState, EndpointProbe, MasterAgent, RecoveryAgentConfig,
    RecoveryStatus, ResourceSnapshot,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

#[tokio::test]
async fn master_agent_recovers_after_endpoint_retry() -> Result<()> {
    let runtime = FakeRuntime::new("", "0.0.0.0:8080->8080/tcp").with_probes(vec![
        probe(500),
        probe(200),
        probe(200),
    ]);
    let config = RecoveryAgentConfig {
        dry_run: false,
        ..RecoveryAgentConfig::default()
    };
    let mut master = MasterAgent::new(runtime, config);

    let report = master.run_once().await?;

    assert_eq!(report.status, AgentHealthStatus::Healthy);
    assert_eq!(
        report.heartbeats[0].recovery_status,
        RecoveryStatus::Recovered
    );
    assert!(report.escalations.is_empty());
    Ok(())
}

#[tokio::test]
async fn master_agent_escalates_after_retry_limit() -> Result<()> {
    let runtime = FakeRuntime::new("HTTP 500 Internal Server Error\nTraceback", "");
    let config = RecoveryAgentConfig {
        max_recovery_attempts: 1,
        ..RecoveryAgentConfig::default()
    };
    let mut master = MasterAgent::new(runtime, config);

    let first = master.run_once().await?;
    assert_eq!(first.heartbeats[0].recovery_status, RecoveryStatus::Planned);

    let second = master.run_once().await?;
    assert_eq!(second.status, AgentHealthStatus::Escalated);
    assert_eq!(
        second.heartbeats[0].recovery_status,
        RecoveryStatus::Escalated
    );
    assert_eq!(second.escalations.len(), 1);
    Ok(())
}

#[derive(Clone)]
struct FakeRuntime {
    logs: Arc<Mutex<String>>,
    ports: String,
    probes: Arc<Mutex<Vec<EndpointProbe>>>,
}

impl FakeRuntime {
    fn new(logs: &str, ports: &str) -> Self {
        Self {
            logs: Arc::new(Mutex::new(logs.to_string())),
            ports: ports.to_string(),
            probes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_probes(self, probes: Vec<EndpointProbe>) -> Self {
        *self.probes.lock().unwrap() = probes;
        self
    }
}

impl ContainerRuntime for FakeRuntime {
    fn list_active_containers<'a>(&'a self) -> BoxFuture<'a, Result<Vec<ContainerIdentity>>> {
        Box::pin(async move {
            Ok(vec![ContainerIdentity {
                id: "abc123".to_string(),
                name: "api".to_string(),
                image: "example/api:latest".to_string(),
                status: "Up 1 minute".to_string(),
                ports: self.ports.clone(),
            }])
        })
    }

    fn inspect_container<'a>(
        &'a self,
        _container_id: &'a str,
    ) -> BoxFuture<'a, Result<ContainerInspection>> {
        Box::pin(async move {
            Ok(ContainerInspection {
                identity: ContainerIdentity {
                    id: "abc123".to_string(),
                    name: "api".to_string(),
                    image: "example/api:latest".to_string(),
                    status: "running".to_string(),
                    ports: self.ports.clone(),
                },
                runtime_state: ContainerRuntimeState::Running,
                docker_health: Some("healthy".to_string()),
                exit_code: Some(0),
                error: None,
                oom_killed: false,
                restart_count: 0,
                env: Vec::new(),
                labels: BTreeMap::new(),
            })
        })
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
            let mut probes = self.probes.lock().unwrap();
            if probes.is_empty() {
                Ok(probe(200))
            } else {
                Ok(probes.remove(0))
            }
        })
    }
}

fn probe(status_code: u16) -> EndpointProbe {
    EndpointProbe {
        url: "http://127.0.0.1:8080/".to_string(),
        status_code: Some(status_code),
        response_time_ms: 1,
        error: None,
    }
}
