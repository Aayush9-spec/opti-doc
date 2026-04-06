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

pub fn run_shell_command(command: &str) -> Result<CommandExecution> {
    let output = Command::new("zsh").arg("-lc").arg(command).output()?;
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
