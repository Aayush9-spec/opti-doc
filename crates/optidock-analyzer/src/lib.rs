use anyhow::{Context, Result};
use optidock_core::{DockerfileAnalysis, Finding, ProjectContext, Severity};
use std::fs;
use std::path::Path;

pub fn analyze_project(path: &str) -> Result<DockerfileAnalysis> {
    let dockerfile_path = Path::new(path).join("Dockerfile");
    let context = ProjectContext {
        path: path.to_string(),
        dockerfile_path: dockerfile_path.display().to_string(),
    };

    let dockerfile_contents = fs::read_to_string(&dockerfile_path).with_context(|| {
        format!(
            "Could not read Dockerfile at {}",
            dockerfile_path.display()
        )
    })?;

    let upper = dockerfile_contents.to_ascii_uppercase();
    let mut findings = Vec::new();

    if !upper.contains("WORKDIR") {
        findings.push(Finding {
            id: "missing-workdir".to_string(),
            severity: Severity::Warning,
            title: "Missing WORKDIR".to_string(),
            explanation: "A missing WORKDIR makes Dockerfiles harder to reason about and often leads to fragile relative paths.".to_string(),
            suggested_fix: "Add an explicit WORKDIR before copy and run steps.".to_string(),
        });
    }

    if has_broad_copy_before_dependencies(&dockerfile_contents) {
        findings.push(Finding {
            id: "copy-too-early".to_string(),
            severity: Severity::Info,
            title: "Broad COPY may be too early".to_string(),
            explanation: "Copying the full project too early can invalidate Docker layer caching and increase rebuild times.".to_string(),
            suggested_fix: "Copy dependency manifests first, install dependencies, then copy the rest of the source.".to_string(),
        });
    }

    if upper.contains("FROM") && !upper.contains("AS ") && !upper.contains("COPY --FROM=") {
        findings.push(Finding {
            id: "single-stage-build".to_string(),
            severity: Severity::Info,
            title: "Single-stage image detected".to_string(),
            explanation: "Single-stage Dockerfiles often ship build tooling and caches into production images, increasing size and attack surface.".to_string(),
            suggested_fix: "Consider a multi-stage build so compilation happens in a builder image and only runtime artifacts are copied into the final stage.".to_string(),
        });
    }

    if upper.contains("USER ROOT") || !upper.contains("\nUSER ") {
        findings.push(Finding {
            id: "runtime-user-review".to_string(),
            severity: Severity::Warning,
            title: "Runtime user should be reviewed".to_string(),
            explanation: "Containers that run as root by default increase blast radius when compromised.".to_string(),
            suggested_fix: "Create and switch to a non-root user for the final runtime stage whenever the base image supports it.".to_string(),
        });
    }

    Ok(DockerfileAnalysis { context, findings })
}

fn has_broad_copy_before_dependencies(dockerfile_contents: &str) -> bool {
    let mut saw_broad_copy = false;

    for raw_line in dockerfile_contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let upper = line.to_ascii_uppercase();
        if upper.starts_with("COPY .") || upper.starts_with("ADD .") {
            saw_broad_copy = true;
        }

        if saw_broad_copy
            && (line.contains("package.json")
                || line.contains("package-lock.json")
                || line.contains("Cargo.toml")
                || line.contains("Cargo.lock")
                || line.contains("requirements.txt")
                || line.contains("poetry.lock")
                || line.contains("go.mod"))
        {
            return true;
        }
    }

    false
}
