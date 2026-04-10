use anyhow::Result;
use std::process::Command;

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
