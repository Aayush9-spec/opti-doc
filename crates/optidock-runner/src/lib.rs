use anyhow::{Context, Result};
use optidock_core::{
    BenchmarkResult, ContainerStatus, DeploymentRecord, ImageInfo, ImageMetrics, MonitorSnapshot,
};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct CommandCheck {
    pub name: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct CommandExecution {
    pub command: String,
    pub success: bool,
    pub status: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRisk {
    Safe,
    NeedsApproval,
}

#[derive(Debug, Clone)]
pub struct CommandPolicy {
    pub risk: CommandRisk,
    pub reason: Option<String>,
}

pub fn verify_docker_available() -> Result<()> {
    let _ = command_version("docker")?;
    Ok(())
}

pub fn command_check(name: &str) -> CommandCheck {
    match command_version(name) {
        Ok(detail) => CommandCheck {
            name: name.to_string(),
            available: true,
            detail,
        },
        Err(error) => CommandCheck {
            name: name.to_string(),
            available: false,
            detail: error.to_string(),
        },
    }
}

pub fn evaluate_command_policy(command: &str) -> CommandPolicy {
    let normalized = command.trim().to_ascii_lowercase();

    if normalized.is_empty() {
        return CommandPolicy {
            risk: CommandRisk::NeedsApproval,
            reason: Some("empty command cannot be evaluated safely".to_string()),
        };
    }

    let dangerous_tokens = [
        (" rm ", "destructive file deletion"),
        ("rm -", "destructive file deletion"),
        ("remove-item", "destructive file deletion"),
        ("del ", "destructive file deletion"),
        ("rmdir ", "destructive directory removal"),
        ("format ", "disk formatting or destructive command"),
        ("mkfs", "filesystem destruction"),
        ("shutdown", "machine control"),
        ("reboot", "machine control"),
        ("restart-computer", "machine control"),
        ("stop-computer", "machine control"),
        ("sudo ", "privilege escalation"),
        ("runas ", "privilege escalation"),
        ("set-executionpolicy", "shell policy modification"),
        ("net user", "system account modification"),
        ("reg add", "registry modification"),
        ("reg delete", "registry modification"),
        ("sc delete", "service deletion"),
        ("diskpart", "disk management"),
        ("mount ", "system mount operation"),
        ("umount", "system mount operation"),
        ("chmod 777", "broad permission escalation"),
        ("chown ", "ownership modification"),
        ("git reset --hard", "destructive repository rewrite"),
        ("git clean -", "destructive repository cleanup"),
        (">", "shell redirection can overwrite files"),
        (">>", "shell redirection modifies files"),
        ("curl ", "network command requires operator intent"),
        ("wget ", "network command requires operator intent"),
        ("invoke-webrequest", "network command requires operator intent"),
        ("invoke-restmethod", "network command requires operator intent"),
        ("npm publish", "publishing operation"),
        ("cargo publish", "publishing operation"),
        ("docker system prune", "destructive docker cleanup"),
        ("docker rm", "container deletion"),
        ("docker rmi", "image deletion"),
        ("docker volume rm", "volume deletion"),
    ];

    for (token, reason) in dangerous_tokens {
        if normalized.contains(token) {
            return CommandPolicy {
                risk: CommandRisk::NeedsApproval,
                reason: Some(reason.to_string()),
            };
        }
    }

    if contains_pipe_or_chain(&normalized) {
        return CommandPolicy {
            risk: CommandRisk::NeedsApproval,
            reason: Some("chained or piped commands need explicit approval".to_string()),
        };
    }

    CommandPolicy {
        risk: CommandRisk::Safe,
        reason: None,
    }
}

pub fn run_shell_command(command: &str) -> Result<CommandExecution> {
    let output = if cfg!(windows) {
        Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(command)
            .output()?
    } else {
        Command::new("sh").arg("-lc").arg(command).output()?
    };

    let status = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated by signal".to_string());

    Ok(CommandExecution {
        command: command.to_string(),
        success: output.status.success(),
        status,
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

// ── Docker Build & Benchmark ─────────────────────────────────────────

pub fn docker_build(path: &str, tag: &str) -> Result<ImageMetrics> {
    verify_docker_available()?;

    let start = Instant::now();
    let output = Command::new("docker")
        .args(["build", "-t", tag, path])
        .output()
        .context("failed to execute docker build")?;

    let build_time_ms = start.elapsed().as_millis() as u64;
    let build_success = output.status.success();

    let (size_bytes, layer_count) = if build_success {
        (docker_image_size(tag), docker_layer_count(tag))
    } else {
        (0, 0)
    };

    Ok(ImageMetrics {
        tag: tag.to_string(),
        size_bytes,
        layer_count,
        build_time_ms,
        build_success,
    })
}

pub fn docker_benchmark(path: &str) -> Result<BenchmarkResult> {
    let baseline_tag = "optidock-baseline:latest";
    let baseline = docker_build(path, baseline_tag)?;

    // Check for optimized Dockerfile
    let optimized_dockerfile = std::path::Path::new(path).join("Dockerfile.optimized");
    let optimized = if optimized_dockerfile.exists() {
        let opt_tag = "optidock-optimized:latest";
        // Temporarily swap Dockerfile
        let original_df = std::path::Path::new(path).join("Dockerfile");
        let backup = std::path::Path::new(path).join("Dockerfile.backup");
        std::fs::copy(&original_df, &backup).ok();
        std::fs::copy(&optimized_dockerfile, &original_df).ok();

        let result = docker_build(path, opt_tag);

        // Restore original
        if backup.exists() {
            std::fs::copy(&backup, &original_df).ok();
            std::fs::remove_file(&backup).ok();
        }

        result.ok()
    } else {
        None
    };

    let summary = if let Some(ref opt) = optimized {
        if opt.build_success && baseline.build_success && baseline.size_bytes > 0 {
            let saved = baseline.size_bytes.saturating_sub(opt.size_bytes);
            let pct = (saved as f64 / baseline.size_bytes as f64 * 100.0) as u64;
            format!(
                "Optimized image saved {} bytes ({}% reduction). Build time: {}ms vs {}ms.",
                saved, pct, opt.build_time_ms, baseline.build_time_ms
            )
        } else {
            "Benchmark completed. Check build results for details.".to_string()
        }
    } else {
        "Baseline built successfully. Run `optidock optimize` first to create a Dockerfile.optimized for comparison.".to_string()
    };

    Ok(BenchmarkResult {
        baseline,
        optimized,
        improvement_summary: summary,
    })
}

fn docker_image_size(tag: &str) -> u64 {
    Command::new("docker")
        .args(["image", "inspect", tag, "--format", "{{.Size}}"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0)
}

fn docker_layer_count(tag: &str) -> u32 {
    Command::new("docker")
        .args(["image", "inspect", tag, "--format", "{{len .RootFS.Layers}}"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u32>()
                .ok()
        })
        .unwrap_or(0)
}

// ── Docker Deploy ────────────────────────────────────────────────────

pub fn docker_deploy(image: &str, name: &str, port: u16) -> Result<DeploymentRecord> {
    verify_docker_available()?;

    // Stop and remove existing container with same name
    let _ = Command::new("docker")
        .args(["stop", name])
        .output();
    let _ = Command::new("docker")
        .args(["rm", name])
        .output();

    let port_mapping = format!("{}:{}", port, port);
    let output = Command::new("docker")
        .args([
            "run", "-d",
            "--name", name,
            "-p", &port_mapping,
            "--restart", "unless-stopped",
            "--memory", "512m",
            "--cpus", "1.0",
            image,
        ])
        .output()
        .context("failed to run docker container")?;

    if !output.status.success() {
        anyhow::bail!(
            "docker run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let short_id = container_id.chars().take(12).collect();

    Ok(DeploymentRecord {
        container_id: short_id,
        image: image.to_string(),
        name: name.to_string(),
        port_mapping,
        status: "running".to_string(),
        started_at: chrono_now(),
    })
}

pub fn docker_rollback(name: &str) -> Result<String> {
    verify_docker_available()?;

    let stop = Command::new("docker")
        .args(["stop", name])
        .output()
        .context("failed to stop container")?;

    if !stop.status.success() {
        anyhow::bail!("Could not stop container '{}'. It may not be running.", name);
    }

    let _ = Command::new("docker").args(["rm", name]).output();

    Ok(format!("Container '{}' stopped and removed.", name))
}

// ── Docker Monitor ───────────────────────────────────────────────────

pub fn docker_monitor() -> Result<MonitorSnapshot> {
    verify_docker_available()?;

    let containers = list_containers()?;
    let images = list_images()?;
    let disk = docker_disk_usage();

    Ok(MonitorSnapshot {
        containers,
        images,
        system_disk_usage: disk,
    })
}

fn list_containers() -> Result<Vec<ContainerStatus>> {
    let output = Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}|{{.Ports}}|{{.CreatedAt}}"])
        .output()
        .context("failed to list containers")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut containers = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 6 {
            containers.push(ContainerStatus {
                container_id: parts[0].to_string(),
                name: parts[1].to_string(),
                image: parts[2].to_string(),
                status: parts[3].to_string(),
                ports: parts[4].to_string(),
                created: parts[5].to_string(),
            });
        }
    }

    Ok(containers)
}

fn list_images() -> Result<Vec<ImageInfo>> {
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}|{{.Tag}}|{{.ID}}|{{.Size}}|{{.CreatedAt}}"])
        .output()
        .context("failed to list images")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut images = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            images.push(ImageInfo {
                repository: parts[0].to_string(),
                tag: parts[1].to_string(),
                image_id: parts[2].to_string(),
                size: parts[3].to_string(),
                created: parts[4].to_string(),
            });
        }
    }

    Ok(images)
}

fn docker_disk_usage() -> Option<String> {
    Command::new("docker")
        .args(["system", "df", "--format", "{{.Type}}: {{.Size}} ({{.Reclaimable}} reclaimable)"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn chrono_now() -> String {
    let output = if cfg!(windows) {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"])
            .output()
            .ok()
    } else {
        Command::new("date").arg("+%Y-%m-%d %H:%M:%S").output().ok()
    };

    output
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn command_version(name: &str) -> Result<String> {
    let output = Command::new(name).arg("--version").output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Ok(stdout);
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Ok(stderr);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let detail = if stderr.is_empty() {
        format!("`{name} --version` exited with {}", output.status)
    } else {
        stderr
    };

    anyhow::bail!("{detail}");
}

fn contains_pipe_or_chain(command: &str) -> bool {
    ["&&", "||", "|", ";"].iter().any(|token| command.contains(token))
}

#[cfg(test)]
mod tests {
    use super::{evaluate_command_policy, CommandRisk};

    #[test]
    fn marks_safe_commands_as_safe() {
        let policy = evaluate_command_policy("Get-ChildItem");
        assert_eq!(policy.risk, CommandRisk::Safe);
    }

    #[test]
    fn marks_destructive_commands_for_approval() {
        let policy = evaluate_command_policy("Remove-Item -Recurse .\\target");
        assert_eq!(policy.risk, CommandRisk::NeedsApproval);
    }

    #[test]
    fn marks_network_commands_for_approval() {
        let policy = evaluate_command_policy("curl https://example.com");
        assert_eq!(policy.risk, CommandRisk::NeedsApproval);
    }
}
