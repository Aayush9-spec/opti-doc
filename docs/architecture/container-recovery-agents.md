# Autonomous Container Recovery Agents

OptiDock now includes a hierarchical recovery-agent system for Docker workloads.

The implementation lives in `crates/optidock-agent/src/recovery.rs` and is exposed through:

```bash
optidock agents
optidock agents --watch
optidock agents --apply
optidock agents --apply --restart-containers
optidock agents --json
```

## Architecture

```text
MasterAgent
  -> discovers Docker containers, including exited containers for crash reporting
  -> ensures one LocalAgent record per container
  -> collects heartbeats and audit entries
  -> escalates unresolved failures

LocalAgent
  -> owns one container assignment
  -> gathers inspect/log/stats/endpoint observations
  -> runs health-check plugins
  -> plans or executes recovery in policy order
  -> emits heartbeat and recovery report
```

The local agent is represented by an OptiDock supervisor process on the host and is assigned exactly one container in the `AgentRegistry`. The master does not modify application source code inside containers.

## Communication Flow

1. `MasterAgent` calls the container runtime to list Docker containers.
2. The registry creates a `LocalAgentRecord` for each new container and reuses existing records to prevent duplicates.
3. Each `LocalAgent` collects:
   - Docker inspect state and health status
   - recent logs
   - CPU and RAM stats
   - endpoint latency/status when a published port exists
4. Health checks emit `ErrorSignal` values.
5. The local agent sends a `Heartbeat` with container name, status, CPU, RAM, response time, running services, current errors, recovery status, and timestamp.
6. Failed recovery creates an `EscalationReport` with root cause, attempts, logs, suggested resolution, and confidence score.

## Recovery Lifecycle

Recovery always follows this priority order:

1. Retry request
2. Restart application process
3. Restart worker
4. Reload configuration
5. Restart service
6. Restart container
7. Escalate

`optidock agents` defaults to dry-run planning. This prevents surprise process or container changes while still showing the exact recovery path that would be attempted.

Use `--apply` to execute configured in-container commands. Use `--restart-containers` only when container restarts are acceptable for the environment.

## Health Checks

The default health-check plugins detect:

- Docker unhealthy/exited/dead/restarting states
- OOM-killed containers
- high or critical memory pressure
- endpoint HTTP 500 and other 5xx responses
- endpoint network and timeout failures
- log patterns for tracebacks, runtime exceptions, Node crashes, Java exceptions, missing modules, missing environment variables, database failures, Redis failures, API failures, timeouts, and segmentation faults

New checks can be added by implementing the `HealthCheck` trait and registering them with `MasterAgent::with_health_check`.

## Configuration

Environment variables:

| Variable | Default | Purpose |
|---|---:|---|
| `OPTIDOCK_AGENT_HEARTBEAT_SECS` | `30` | Watch-loop heartbeat interval |
| `OPTIDOCK_AGENT_HEALTH_CHECK_SECS` | `10` | Intended health-check frequency for future schedulers |
| `OPTIDOCK_AGENT_MAX_RECOVERY_ATTEMPTS` | `3` | Escalate after repeated failed cycles |
| `OPTIDOCK_AGENT_LOG_TAIL` | `200` | Log lines collected per container |
| `OPTIDOCK_AGENT_RESPONSE_TIMEOUT_MS` | `2000` | Endpoint probe timeout |
| `OPTIDOCK_AGENT_MEMORY_WARN_PERCENT` | `80` | Memory warning threshold |
| `OPTIDOCK_AGENT_MEMORY_CRITICAL_PERCENT` | `95` | Memory critical threshold |
| `OPTIDOCK_AGENT_HEALTH_PATH` | `/` | Path probed on published ports |
| `OPTIDOCK_AGENT_APPLY` | `false` | Execute configured recovery commands |
| `OPTIDOCK_AGENT_ALLOW_CONTAINER_RESTART` | `false` | Permit `docker restart` |
| `OPTIDOCK_AGENT_APP_RESTART_COMMAND` | unset | Shell command executed inside the container for app process restart |
| `OPTIDOCK_AGENT_WORKER_RESTART_COMMAND` | unset | Shell command executed inside the container for worker restart |
| `OPTIDOCK_AGENT_RELOAD_COMMAND` | unset | Shell command executed inside the container for config reload |
| `OPTIDOCK_AGENT_SERVICE_RESTART_COMMAND` | unset | Shell command executed inside the container for service restart |

Container labels can override global commands per container:

| Label | Purpose |
|---|---|
| `optidock.agent.restart-command` | Restart application process |
| `optidock.agent.worker-restart-command` | Restart worker |
| `optidock.agent.reload-command` | Reload configuration |
| `optidock.agent.service-restart-command` | Restart service |

## Safety Rules

The recovery executor never deletes source code, databases, images, or volumes. It does not stop unrelated containers. The only Docker mutation implemented by default is `docker restart`, and it requires `--restart-containers` or `OPTIDOCK_AGENT_ALLOW_CONTAINER_RESTART=true`.

All recovery activity is recorded in the report audit log.

## Tests

Coverage includes:

- HTTP 500 and database-error detection
- recovery priority ordering
- duplicate local-agent prevention
- endpoint inference
- master-agent discovery and heartbeat generation
- dry-run recovery planning
- public API workflow tests for endpoint retry recovery and retry-limit escalation
