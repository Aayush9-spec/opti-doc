use anyhow::{Context, Result};
use optidock_core::{
    DockerfileAnalysis, Finding, ProjectContext, SecurityAudit, SecurityCategory, SecurityFinding,
    SecurityGrade, Severity,
};
use std::fs;
use std::path::Path;

// ── Dockerfile Analysis ──────────────────────────────────────────────

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

// ── Security Audit Engine ────────────────────────────────────────────

pub fn security_audit(path: &str) -> Result<SecurityAudit> {
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

    let findings = run_security_rules(&dockerfile_contents);
    let score = compute_security_score(&findings);
    let grade = score_to_grade(score);
    let summary = format!(
        "Security audit completed. Score: {}/100 (Grade {}). {} finding(s) detected.",
        score,
        grade_label(grade),
        findings.len()
    );

    Ok(SecurityAudit {
        context,
        findings,
        score,
        grade,
        summary,
    })
}

fn run_security_rules(contents: &str) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let upper = contents.to_ascii_uppercase();
    let lines: Vec<&str> = contents.lines().collect();

    // ── Secrets Detection ────────────────────────────────────────────

    let secret_patterns = [
        "API_KEY", "SECRET_KEY", "PASSWORD", "TOKEN", "PRIVATE_KEY",
        "AWS_ACCESS", "AWS_SECRET", "DATABASE_URL", "MYSQL_ROOT_PASSWORD",
    ];
    for (line_num, line) in lines.iter().enumerate() {
        let line_upper = line.to_ascii_uppercase();
        if line_upper.starts_with("ENV ") || line_upper.starts_with("ARG ") {
            for pattern in &secret_patterns {
                if line_upper.contains(pattern) && line.contains('=') {
                    let value_part = line.split('=').nth(1).unwrap_or("");
                    if !value_part.trim().is_empty()
                        && !value_part.contains('$')
                        && !value_part.contains("changeme")
                    {
                        findings.push(SecurityFinding {
                            id: format!("secret-in-layer-L{}", line_num + 1),
                            category: SecurityCategory::Secrets,
                            severity: Severity::Critical,
                            title: format!("Potential hardcoded secret at line {}", line_num + 1),
                            detail: format!("The instruction `{}` may embed a secret into an image layer. Secrets baked into layers persist in all derived images and registries.", line.trim()),
                            remediation: "Use Docker build secrets (`--secret`), multi-stage builds, or runtime environment injection instead of ENV/ARG for sensitive values.".to_string(),
                        });
                    }
                }
            }
        }
    }

    // ── Privilege Escalation ─────────────────────────────────────────

    if upper.contains("--PRIVILEGED") {
        findings.push(SecurityFinding {
            id: "privileged-flag".to_string(),
            category: SecurityCategory::Privileges,
            severity: Severity::Critical,
            title: "Privileged mode reference detected".to_string(),
            detail: "Privileged containers bypass nearly all kernel-level security boundaries. A single escape gives full host access.".to_string(),
            remediation: "Remove --privileged. Use specific capabilities (--cap-add) only when absolutely required.".to_string(),
        });
    }

    if upper.contains("USER ROOT") {
        findings.push(SecurityFinding {
            id: "explicit-root-user".to_string(),
            category: SecurityCategory::Privileges,
            severity: Severity::Warning,
            title: "Explicit USER root instruction".to_string(),
            detail: "Setting USER root as the runtime user gives full filesystem and process access inside the container.".to_string(),
            remediation: "Switch to a non-root user for the final stage: `RUN adduser --system appuser && USER appuser`.".to_string(),
        });
    }

    if !upper.contains("\nUSER ") {
        findings.push(SecurityFinding {
            id: "no-user-instruction".to_string(),
            category: SecurityCategory::Privileges,
            severity: Severity::Warning,
            title: "No USER instruction found".to_string(),
            detail: "Without a USER instruction the container defaults to root, expanding the blast radius of any vulnerability.".to_string(),
            remediation: "Add a non-root USER instruction before the CMD/ENTRYPOINT.".to_string(),
        });
    }

    // ── Network Exposure ─────────────────────────────────────────────

    let sensitive_ports = ["22", "23", "3389", "5432", "3306", "6379", "27017", "2375"];
    for line in &lines {
        let line_upper = line.trim().to_ascii_uppercase();
        if line_upper.starts_with("EXPOSE ") {
            let port_str = line_upper.trim_start_matches("EXPOSE ").trim();
            for port in port_str.split_whitespace() {
                let port_num = port.split('/').next().unwrap_or(port);
                if sensitive_ports.contains(&port_num) {
                    findings.push(SecurityFinding {
                        id: format!("sensitive-port-{}", port_num),
                        category: SecurityCategory::NetworkExposure,
                        severity: Severity::Warning,
                        title: format!("Sensitive port {} exposed", port_num),
                        detail: format!("Port {} is commonly associated with administrative or database services that should not be directly reachable.", port_num),
                        remediation: "Remove the EXPOSE instruction or restrict access through network policies and firewall rules.".to_string(),
                    });
                }
            }
        }
    }

    // ── Base Image Quality ───────────────────────────────────────────

    for line in &lines {
        let trimmed = line.trim().to_ascii_uppercase();
        if trimmed.starts_with("FROM ") {
            let image_ref = line.trim().split_whitespace().nth(1).unwrap_or("");
            if image_ref.ends_with(":latest") || !image_ref.contains(':') {
                findings.push(SecurityFinding {
                    id: "unpinned-base-image".to_string(),
                    category: SecurityCategory::BaseImage,
                    severity: Severity::Warning,
                    title: "Base image tag is unpinned or uses :latest".to_string(),
                    detail: format!("The base image `{}` does not pin a specific version. Builds may silently pull breaking or vulnerable updates.", image_ref),
                    remediation: "Pin base images to a specific tag or SHA256 digest, e.g. `node:20.11-alpine` or `debian:bookworm-slim@sha256:...`.".to_string(),
                });
            }

            let large_images = ["ubuntu", "debian", "centos", "fedora", "node:", "python:"];
            let image_lower = image_ref.to_ascii_lowercase();
            if large_images.iter().any(|i| image_lower.starts_with(i))
                && !image_lower.contains("slim")
                && !image_lower.contains("alpine")
            {
                findings.push(SecurityFinding {
                    id: "large-base-image".to_string(),
                    category: SecurityCategory::BaseImage,
                    severity: Severity::Info,
                    title: "Full-size base image increases attack surface".to_string(),
                    detail: format!("The base image `{}` includes many packages not needed at runtime, expanding the CVE surface.", image_ref),
                    remediation: "Prefer slim or alpine variants, or use distroless images for production.".to_string(),
                });
            }
        }
    }

    // ── Supply Chain ─────────────────────────────────────────────────

    if upper.contains("CURL ") && upper.contains("| SH")
        || upper.contains("| BASH")
        || upper.contains("WGET ") && upper.contains("| SH")
    {
        findings.push(SecurityFinding {
            id: "pipe-to-shell".to_string(),
            category: SecurityCategory::SupplyChain,
            severity: Severity::Critical,
            title: "Piping remote script to shell".to_string(),
            detail: "Downloading and immediately executing a remote script bypasses integrity verification and introduces supply chain risk.".to_string(),
            remediation: "Download scripts first, verify checksums, then execute. Or use package managers with signature verification.".to_string(),
        });
    }

    // ── Runtime Safety ───────────────────────────────────────────────

    if !upper.contains("HEALTHCHECK") {
        findings.push(SecurityFinding {
            id: "missing-healthcheck".to_string(),
            category: SecurityCategory::RuntimeSafety,
            severity: Severity::Info,
            title: "No HEALTHCHECK instruction".to_string(),
            detail: "Without a HEALTHCHECK, orchestrators cannot detect if the application is responsive. Zombie containers may keep receiving traffic.".to_string(),
            remediation: "Add a HEALTHCHECK instruction: `HEALTHCHECK CMD curl -f http://localhost:PORT/ || exit 1`.".to_string(),
        });
    }

    if !upper.contains("LABEL ") {
        findings.push(SecurityFinding {
            id: "missing-labels".to_string(),
            category: SecurityCategory::Misconfiguration,
            severity: Severity::Info,
            title: "No metadata labels".to_string(),
            detail: "Labels provide provenance and traceability. Missing labels make auditing and image management harder.".to_string(),
            remediation: "Add LABEL maintainer, version, and description for traceability.".to_string(),
        });
    }

    // ── Package Cache ────────────────────────────────────────────────

    if (upper.contains("APT-GET INSTALL") || upper.contains("APT INSTALL"))
        && !upper.contains("RM -RF /VAR/LIB/APT")
    {
        findings.push(SecurityFinding {
            id: "apt-cache-not-cleaned".to_string(),
            category: SecurityCategory::ResourceLimits,
            severity: Severity::Warning,
            title: "Package cache not cleaned after install".to_string(),
            detail: "Leaving the apt cache inflates image size and retains metadata about installed packages.".to_string(),
            remediation: "Chain cleanup in the same RUN layer: `RUN apt-get update && apt-get install -y ... && rm -rf /var/lib/apt/lists/*`.".to_string(),
        });
    }

    if upper.contains("NPM INSTALL") && !upper.contains("NPM CI") {
        findings.push(SecurityFinding {
            id: "npm-install-over-ci".to_string(),
            category: SecurityCategory::SupplyChain,
            severity: Severity::Info,
            title: "Prefer `npm ci` over `npm install`".to_string(),
            detail: "`npm install` may update the lockfile, producing non-deterministic builds.".to_string(),
            remediation: "Use `npm ci` for reproducible builds that respect the lockfile.".to_string(),
        });
    }

    findings
}

fn compute_security_score(findings: &[SecurityFinding]) -> u8 {
    let mut score: i32 = 100;
    for finding in findings {
        match finding.severity {
            Severity::Critical => score -= 20,
            Severity::Warning => score -= 10,
            Severity::Info => score -= 3,
        }
    }
    score.max(0).min(100) as u8
}

fn score_to_grade(score: u8) -> SecurityGrade {
    match score {
        90..=100 => SecurityGrade::A,
        75..=89 => SecurityGrade::B,
        60..=74 => SecurityGrade::C,
        40..=59 => SecurityGrade::D,
        _ => SecurityGrade::F,
    }
}

pub fn grade_label(grade: SecurityGrade) -> &'static str {
    match grade {
        SecurityGrade::A => "A",
        SecurityGrade::B => "B",
        SecurityGrade::C => "C",
        SecurityGrade::D => "D",
        SecurityGrade::F => "F",
    }
}

// ── Dockerfile Optimization Engine ───────────────────────────────────

pub fn generate_optimized_dockerfile(path: &str) -> Result<optidock_core::OptimizedDockerfile> {
    let dockerfile_path = Path::new(path).join("Dockerfile");
    let original = fs::read_to_string(&dockerfile_path).with_context(|| {
        format!("Could not read Dockerfile at {}", dockerfile_path.display())
    })?;

    let mut optimized = original.clone();
    let mut changes = Vec::new();

    // Multi-stage: wrap single-stage builds
    let upper = original.to_ascii_uppercase();
    if upper.contains("FROM") && !upper.contains("AS ") {
        let first_from = original.lines().find(|l| l.trim().to_ascii_uppercase().starts_with("FROM ")).unwrap_or("");
        let new_from = format!("{} AS builder", first_from.trim());
        optimized = optimized.replacen(first_from.trim(), &new_from, 1);
        changes.push("Converted to multi-stage build (added AS builder alias)".to_string());
    }

    // Add WORKDIR if missing
    if !upper.contains("WORKDIR") {
        if let Some(pos) = optimized.find('\n') {
            optimized.insert_str(pos + 1, "\nWORKDIR /app\n");
            changes.push("Added explicit WORKDIR /app".to_string());
        }
    }

    // Add non-root USER if missing
    if !upper.contains("\nUSER ") {
        optimized.push_str("\n# Run as non-root for security\nRUN adduser --system --no-create-home appuser 2>/dev/null || true\nUSER appuser\n");
        changes.push("Added non-root USER (appuser) for runtime safety".to_string());
    }

    // Add HEALTHCHECK if missing
    if !upper.contains("HEALTHCHECK") {
        optimized.push_str("\nHEALTHCHECK --interval=30s --timeout=5s --retries=3 \\\n  CMD curl -f http://localhost:${PORT:-8080}/ || exit 1\n");
        changes.push("Added HEALTHCHECK for container liveness monitoring".to_string());
    }

    // Add labels if missing
    if !upper.contains("LABEL ") {
        if let Some(pos) = optimized.find('\n') {
            optimized.insert_str(pos + 1, "\nLABEL maintainer=\"optidock-ai\" \\\n      version=\"1.0\" \\\n      description=\"Optimized by OptiDock AI\"\n");
            changes.push("Added metadata LABELs for image traceability".to_string());
        }
    }

    // Clean apt cache
    if (upper.contains("APT-GET INSTALL") || upper.contains("APT INSTALL"))
        && !upper.contains("RM -RF /VAR/LIB/APT")
    {
        optimized = optimized.replace(
            "apt-get install",
            "apt-get install -y --no-install-recommends",
        );
        // Append cleanup to RUN lines containing apt-get install
        if !optimized.contains("rm -rf /var/lib/apt") {
            optimized = optimized.replace(
                "apt-get install -y --no-install-recommends",
                "apt-get install -y --no-install-recommends",
            );
            changes.push("Recommended: chain `&& rm -rf /var/lib/apt/lists/*` after apt-get install".to_string());
        }
    }

    let output_path = Path::new(path).join("Dockerfile.optimized");
    fs::write(&output_path, &optimized)?;

    Ok(optidock_core::OptimizedDockerfile {
        original_path: dockerfile_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changes_applied: changes,
        original_content: original,
        optimized_content: optimized,
    })
}

// ── Helpers ──────────────────────────────────────────────────────────

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
