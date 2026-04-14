use anyhow::Result;
use optidock_analyzer::{analyze_project, security_audit as run_security_audit, generate_optimized_dockerfile};
use optidock_core::{
    AiProviderConfig, AiProviderKind, AiRuntimeConfig, CiProvider, ContainerService,
    DeploymentPlan, DeploymentStrategy, DeploymentTarget, DockerfileAnalysis, OptimizationProposal,
    OptimizationRequest, OptimizedDockerfile, PipelineContext, PipelineModerationReport,
    PipelineRecommendation, PipelineStatus, ProjectContext, PromptLibrary, SecurityAudit,
    ServiceRole, Severity, TrafficProfile, default_prompt_library,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn run_analysis(path: &str) -> Result<DockerfileAnalysis> {
    analyze_project(path)
}

pub fn run_security_scan(path: &str) -> Result<SecurityAudit> {
    run_security_audit(path)
}

pub fn run_optimize(path: &str) -> Result<OptimizedDockerfile> {
    generate_optimized_dockerfile(path)
}

pub trait OptimizationProvider {
    fn provider_kind(&self) -> AiProviderKind;
    fn generate_optimization(
        &self,
        request: &OptimizationRequest,
        config: &AiProviderConfig,
    ) -> Result<OptimizationProposal>;
}

pub fn saved_prompt_library() -> PromptLibrary {
    default_prompt_library()
}

pub fn build_chat_prompt(user_input: &str, project_context: Option<&str>) -> optidock_core::PromptPack {
    let mut values = BTreeMap::new();
    values.insert("user_input".to_string(), user_input.to_string());

    if let Some(project_context) = project_context {
        values.insert("project_context".to_string(), project_context.to_string());
    }

    saved_prompt_library()
        .build_pack("chat-default", &values)
        .expect("default chat prompt should exist")
}

pub fn build_architecture_prompt(
    user_input: &str,
    project_context: Option<&str>,
    module_context: Option<&str>,
) -> optidock_core::PromptPack {
    let mut values = BTreeMap::new();
    values.insert("user_input".to_string(), user_input.to_string());

    if let Some(project_context) = project_context {
        values.insert("project_context".to_string(), project_context.to_string());
    }

    if let Some(module_context) = module_context {
        values.insert("module_context".to_string(), module_context.to_string());
    }

    saved_prompt_library()
        .build_pack("systems-architecture", &values)
        .expect("systems architecture prompt should exist")
}

pub fn moderate_pipeline(pipeline: PipelineContext) -> PipelineModerationReport {
    let mut recommendations = Vec::new();

    for service in &pipeline.services {
        if service.dockerfile_path.is_none() && service.image.is_none() {
            recommendations.push(PipelineRecommendation {
                id: format!("service-image-missing-{}", service.name),
                severity: Severity::Critical,
                title: format!("Service '{}' has no image source", service.name),
                rationale: "The pipeline cannot build or deploy a container efficiently when the service has neither a Dockerfile path nor a pinned image reference.".to_string(),
                action: "Define a Dockerfile path or provide an immutable image reference before promotion.".to_string(),
            });
        }

        if matches!(service.traffic_profile, TrafficProfile::High | TrafficProfile::Burst)
            && !matches!(service.role, ServiceRole::Gateway | ServiceRole::Api)
        {
            recommendations.push(PipelineRecommendation {
                id: format!("traffic-profile-review-{}", service.name),
                severity: Severity::Warning,
                title: format!("Review scaling assumptions for '{}'", service.name),
                rationale: "A high-traffic or bursty workload should have an explicit rollout and recovery strategy to avoid noisy deployments.".to_string(),
                action: "Add smoke checks, readiness gates, and staged rollout controls for this service.".to_string(),
            });
        }

        if matches!(service.deployment, DeploymentTarget::Unknown) {
            recommendations.push(PipelineRecommendation {
                id: format!("deployment-target-unknown-{}", service.name),
                severity: Severity::Warning,
                title: format!("Deployment target missing for '{}'", service.name),
                rationale: "Without a known deployment target the agent cannot choose the safest rollout behavior.".to_string(),
                action: "Set a concrete deployment target so the agent can tailor deployment and rollback steps.".to_string(),
            });
        }
    }

    if !pipeline
        .services
        .iter()
        .any(|service| matches!(service.role, ServiceRole::Gateway | ServiceRole::Api))
    {
        recommendations.push(PipelineRecommendation {
            id: "no-public-entry-service".to_string(),
            severity: Severity::Info,
            title: "No public entry service detected".to_string(),
            rationale: "The pipeline may be fully internal, but lacking an entry service makes rollout validation harder for web-facing systems.".to_string(),
            action: "If this stack serves users, mark the ingress-facing service explicitly so benchmark and smoke checks target the right container.".to_string(),
        });
    }

    let critical_count = recommendations
        .iter()
        .filter(|item| matches!(item.severity, Severity::Critical))
        .count();
    let warning_count = recommendations
        .iter()
        .filter(|item| matches!(item.severity, Severity::Warning))
        .count();

    let status = if critical_count > 0 {
        PipelineStatus::Critical
    } else if warning_count > 0 {
        PipelineStatus::NeedsAttention
    } else {
        PipelineStatus::Healthy
    };

    let strategy = select_strategy(&pipeline);
    let deployment_plan = DeploymentPlan {
        strategy,
        rollout_steps: build_rollout_steps(&pipeline, strategy),
        rollback_trigger: "Rollback immediately if startup checks fail, error rate increases, or runtime performance regresses beyond the accepted threshold.".to_string(),
    };

    let summary = format!(
        "Pipeline moderation completed for '{}' on '{}' with {} service(s). Status: {:?}.",
        pipeline.repository,
        pipeline.branch,
        pipeline.services.len(),
        status
    );

    PipelineModerationReport {
        pipeline,
        status,
        summary,
        recommendations,
        deployment_plan,
    }
}

pub fn default_pipeline_context(path: &str) -> PipelineContext {
    PipelineContext {
        provider: CiProvider::GitHubActions,
        repository: path.to_string(),
        branch: "main".to_string(),
        environment: "staging".to_string(),
        services: vec![ContainerService {
            name: "app".to_string(),
            image: None,
            dockerfile_path: Some(format!("{path}/Dockerfile")),
            role: ServiceRole::Api,
            deployment: DeploymentTarget::LocalDocker,
            traffic_profile: TrafficProfile::Medium,
        }],
    }
}

// ── LLM Provider System ─────────────────────────────────────────────
//
// Default: Gemini 2.0 Flash (free tier, generous limits)
// Override: OPTIDOCK_PROVIDER=anthropic, OPTIDOCK_MODEL=claude-3-opus, etc.
// Config:  ~/.optidock/provider.json persists user choice

pub fn default_ai_runtime_config() -> AiRuntimeConfig {
    let active = resolve_active_provider();
    let active_kind = active.kind;

    let mut fallbacks: Vec<AiProviderConfig> = all_providers()
        .into_iter()
        .filter(|p| p.kind != active_kind)
        .collect();

    // Put free/local providers first in fallback order
    fallbacks.sort_by_key(|p| match p.kind {
        AiProviderKind::Ollama => 0,
        AiProviderKind::LlamaCpp => 1,
        AiProviderKind::Gemini => 2,
        AiProviderKind::Groq => 3,
        AiProviderKind::OpenRouter => 4,
        AiProviderKind::OpenAi => 5,
        AiProviderKind::Anthropic => 6,
        _ => 99,
    });

    AiRuntimeConfig {
        active_provider: active,
        fallback_providers: fallbacks,
        request_timeout_secs: 90,
    }
}

/// Returns all supported LLM providers
pub fn all_providers() -> Vec<AiProviderConfig> {
    vec![
        gemini_provider(),
        openai_provider(),
        anthropic_provider(),
        openrouter_provider(),
        groq_provider(),
        ollama_provider(),
        llamacpp_provider(),
        local_openai_provider(),
    ]
}

/// Resolve which provider to use based on:
/// 1. OPTIDOCK_PROVIDER env var
/// 2. Saved config file (~/.optidock/provider.json)
/// 3. Default: Gemini 2.0 Flash (free)
fn resolve_active_provider() -> AiProviderConfig {
    // Check env var first
    if let Ok(provider_name) = std::env::var("OPTIDOCK_PROVIDER") {
        let kind = detect_provider_from_name(&provider_name);
        let model_override = std::env::var("OPTIDOCK_MODEL").ok();
        let mut config = provider_by_kind(kind);
        if let Some(model) = model_override {
            config.model = model;
        }
        return config;
    }

    // Check saved config
    if let Some(saved) = load_saved_provider() {
        return saved;
    }

    // Default: Gemini 2.0 Flash (free tier)
    gemini_provider()
}

/// Get the config file path
fn provider_config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".optidock").join("provider.json")
}

/// Save provider choice to disk
pub fn save_provider_config(provider_name: &str, model: Option<&str>) -> Result<()> {
    let kind = detect_provider_from_name(provider_name);
    let mut config = provider_by_kind(kind);
    if let Some(m) = model {
        config.model = m.to_string();
    }

    let config_path = provider_config_path();
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, json)?;
    Ok(())
}

/// Load saved provider from disk
fn load_saved_provider() -> Option<AiProviderConfig> {
    let config_path = provider_config_path();
    let content = std::fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Maps a provider kind back to a default config
fn provider_by_kind(kind: AiProviderKind) -> AiProviderConfig {
    match kind {
        AiProviderKind::OpenAi => openai_provider(),
        AiProviderKind::Anthropic => anthropic_provider(),
        AiProviderKind::Gemini => gemini_provider(),
        AiProviderKind::OpenRouter => openrouter_provider(),
        AiProviderKind::Groq => groq_provider(),
        AiProviderKind::Ollama => ollama_provider(),
        AiProviderKind::LlamaCpp => llamacpp_provider(),
        AiProviderKind::LocalOpenAiCompatible => local_openai_provider(),
        AiProviderKind::Custom => openai_provider(),
    }
}

pub fn build_optimization_request(
    path: &str,
    dockerfile_contents: &str,
    findings: Vec<optidock_core::Finding>,
) -> OptimizationRequest {
    OptimizationRequest {
        project: ProjectContext {
            path: path.to_string(),
            dockerfile_path: format!("{path}/Dockerfile"),
        },
        instructions: "Optimize this Dockerfile for image size, build efficiency, runtime safety, and deployment readiness without changing application behavior unless necessary.".to_string(),
        dockerfile_contents: dockerfile_contents.to_string(),
        findings,
    }
}

pub fn provider_summary(config: &AiRuntimeConfig) -> String {
    let fallback_names = config
        .fallback_providers
        .iter()
        .map(|provider| provider_label(provider.kind))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "Active provider: {} ({}) | Fallbacks: {}",
        provider_label(config.active_provider.kind),
        config.active_provider.model,
        fallback_names
    )
}

pub fn detect_provider_from_name(name: &str) -> AiProviderKind {
    match name.trim().to_ascii_lowercase().as_str() {
        "openai" | "chatgpt" | "gpt" => AiProviderKind::OpenAi,
        "anthropic" | "claude" => AiProviderKind::Anthropic,
        "gemini" | "google" => AiProviderKind::Gemini,
        "openrouter" => AiProviderKind::OpenRouter,
        "groq" => AiProviderKind::Groq,
        "ollama" => AiProviderKind::Ollama,
        "llamacpp" | "llama-cpp" | "llama.cpp" | "llama" => AiProviderKind::LlamaCpp,
        "local" | "local-openai" | "lm-studio" | "vllm" | "lmstudio" => AiProviderKind::LocalOpenAiCompatible,
        _ => AiProviderKind::Custom,
    }
}

pub fn provider_label(kind: AiProviderKind) -> &'static str {
    match kind {
        AiProviderKind::OpenAi => "OpenAI",
        AiProviderKind::Anthropic => "Anthropic",
        AiProviderKind::Gemini => "Gemini",
        AiProviderKind::OpenRouter => "OpenRouter",
        AiProviderKind::Groq => "Groq",
        AiProviderKind::LlamaCpp => "llama.cpp",
        AiProviderKind::LocalOpenAiCompatible => "Local OpenAI-Compatible",
        AiProviderKind::Ollama => "Ollama",
        AiProviderKind::Custom => "Custom",
    }
}

// ── Provider Definitions ─────────────────────────────────────────────

fn gemini_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Gemini,
        model: "gemini-2.0-flash".to_string(),
        api_base: "https://generativelanguage.googleapis.com".to_string(),
        api_key_env: Some("GEMINI_API_KEY".to_string()),
        api_key: None,
        organization: None,
        project: None,
        local: false,
    }
}

fn openai_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::OpenAi,
        model: "gpt-4.1-mini".to_string(),
        api_base: "https://api.openai.com/v1".to_string(),
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        api_key: None,
        organization: None,
        project: None,
        local: false,
    }
}

fn anthropic_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Anthropic,
        model: "claude-sonnet-4-20250514".to_string(),
        api_base: "https://api.anthropic.com".to_string(),
        api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
        api_key: None,
        organization: None,
        project: None,
        local: false,
    }
}

fn openrouter_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::OpenRouter,
        model: "google/gemini-2.0-flash-exp:free".to_string(),
        api_base: "https://openrouter.ai/api/v1".to_string(),
        api_key_env: Some("OPENROUTER_API_KEY".to_string()),
        api_key: None,
        organization: None,
        project: None,
        local: false,
    }
}

fn groq_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Groq,
        model: "llama-3.3-70b-versatile".to_string(),
        api_base: "https://api.groq.com/openai/v1".to_string(),
        api_key_env: Some("GROQ_API_KEY".to_string()),
        api_key: None,
        organization: None,
        project: None,
        local: false,
    }
}

fn ollama_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Ollama,
        model: "llama3.1".to_string(),
        api_base: "http://127.0.0.1:11434".to_string(),
        api_key_env: None,
        api_key: None,
        organization: None,
        project: None,
        local: true,
    }
}

fn llamacpp_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::LlamaCpp,
        model: "local-model".to_string(),
        api_base: "http://127.0.0.1:8080".to_string(),
        api_key_env: None,
        api_key: None,
        organization: None,
        project: None,
        local: true,
    }
}

fn local_openai_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::LocalOpenAiCompatible,
        model: "local-model".to_string(),
        api_base: "http://127.0.0.1:1234/v1".to_string(),
        api_key_env: None,
        api_key: None,
        organization: None,
        project: None,
        local: true,
    }
}

// ── Strategy Selection ───────────────────────────────────────────────

fn select_strategy(pipeline: &PipelineContext) -> DeploymentStrategy {
    if pipeline
        .services
        .iter()
        .any(|service| matches!(service.traffic_profile, TrafficProfile::Burst | TrafficProfile::High))
    {
        DeploymentStrategy::Canary
    } else if pipeline.services.len() > 1 {
        DeploymentStrategy::Rolling
    } else {
        DeploymentStrategy::BlueGreen
    }
}

fn build_rollout_steps(
    pipeline: &PipelineContext,
    strategy: DeploymentStrategy,
) -> Vec<String> {
    let mut steps = vec![
        format!(
            "Validate build artifacts and container metadata for {} service(s).",
            pipeline.services.len()
        ),
        "Run smoke tests before promotion.".to_string(),
    ];

    match strategy {
        DeploymentStrategy::Canary => {
            steps.push("Ship optimized containers to a small traffic slice first.".to_string());
            steps.push("Compare latency, startup stability, and error rate before full rollout.".to_string());
        }
        DeploymentStrategy::Rolling => {
            steps.push("Replace services incrementally to avoid full-environment interruption.".to_string());
        }
        DeploymentStrategy::BlueGreen => {
            steps.push("Stand up the optimized release beside the active environment.".to_string());
            steps.push("Switch traffic only after validation passes.".to_string());
        }
        DeploymentStrategy::Recreate => {
            steps.push("Stop the old workload and replace it in one controlled action.".to_string());
        }
    }

    steps.push("Persist deployment outcome and benchmark results for future agent decisions.".to_string());
    steps
}
