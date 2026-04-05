use anyhow::Result;
use optidock_core::{DockerfileAnalysis, Finding, ProjectContext, Severity};

pub fn analyze_project(path: &str) -> Result<DockerfileAnalysis> {
    let context = ProjectContext {
        path: path.to_string(),
        dockerfile_path: format!("{path}/Dockerfile"),
    };

    let findings = vec![
        Finding {
            id: "missing-workdir".to_string(),
            severity: Severity::Warning,
            title: "Missing WORKDIR".to_string(),
            explanation: "A missing WORKDIR makes Dockerfiles harder to reason about and often leads to fragile relative paths.".to_string(),
            suggested_fix: "Add an explicit WORKDIR before copy and run steps.".to_string(),
        },
        Finding {
            id: "copy-too-early".to_string(),
            severity: Severity::Info,
            title: "Broad COPY may be too early".to_string(),
            explanation: "Copying the full project too early can invalidate Docker layer caching and increase rebuild times.".to_string(),
            suggested_fix: "Copy dependency manifests first, install dependencies, then copy the rest of the source.".to_string(),
        },
    ];

    Ok(DockerfileAnalysis { context, findings })
}
