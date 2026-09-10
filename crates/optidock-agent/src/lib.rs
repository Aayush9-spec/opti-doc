use anyhow::Result;
pub mod recovery;
use optidock_analyzer::{
    analyze_project, generate_optimized_dockerfile, security_audit as run_security_audit,
};
use optidock_brain::provider::openai::OpenAiDiagnosticProvider;
use optidock_core::{
    default_prompt_library, AiProviderConfig, AiProviderKind, AiRuntimeConfig, CiProvider,
    ContainerService, DeploymentPlan, DeploymentStrategy, DeploymentTarget, DockerfileAnalysis,
    OptimizationRequest, OptimizedDockerfile, PipelineContext, PipelineModerationReport,
    PipelineRecommendation, PipelineStatus, ProjectContext, PromptLibrary, SecurityAudit,
    ServiceRole, Severity, TrafficProfile,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub use recovery::{
    default_health_checks, default_recovery_policy, detect_error_signals,
    run_recovery_supervision_once, AgentHealthStatus, AgentLogEntry, AgentRegistry, BoxFuture,
    CommandResult, ContainerIdentity, ContainerInspection, ContainerObservation, ContainerRuntime,
    ContainerRuntimeState, DockerCliRuntime, EndpointProbe, ErrorKind, ErrorSignal,
    EscalationReport, HealthCheck, Heartbeat, LocalAgent, LocalAgentRecord, MasterAgent,
    MasterAgentReport, RecoveryAction, RecoveryAgentConfig, RecoveryAttempt, RecoveryAttemptStatus,
    RecoveryPolicy, RecoveryReport, RecoveryStatus, ResourceSnapshot, RootCause, RootCauseCategory,
    SignalSeverity,
};

pub fn run_analysis(path: &str) -> Result<DockerfileAnalysis> {
    analyze_project(path)
}

pub fn run_security_scan(path: &str) -> Result<SecurityAudit> {
    run_security_audit(path)
}

pub fn run_optimize(path: &str) -> Result<OptimizedDockerfile> {
    generate_optimized_dockerfile(path)
}

pub fn saved_prompt_library() -> PromptLibrary {
    default_prompt_library()
}

pub fn build_chat_prompt(
    user_input: &str,
    project_context: Option<&str>,
) -> optidock_core::PromptPack {
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

        if matches!(
            service.traffic_profile,
            TrafficProfile::High | TrafficProfile::Burst
        ) && !matches!(service.role, ServiceRole::Gateway | ServiceRole::Api)
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
// Transport: single OpenAI-compatible POST {base}/chat/completions
// Default:   OpenAI gpt-4.1-mini (no baked-in vendor defaults)
// Override:  OPTIDOCK_PROVIDER=openai|openrouter|groq|ollama|llamacpp|local|gemini|together|deepseek|vllm
//            OPTIDOCK_API_BASE=... OPTIDOCK_API_KEY=... OPTIDOCK_MODEL=...
// Config:    ~/.optidock/provider.json persists user choice

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
        AiProviderKind::LocalOpenAiCompatible => 2,
        AiProviderKind::Vllm => 3,
        AiProviderKind::Gemini => 4,
        AiProviderKind::Groq => 5,
        AiProviderKind::OpenRouter => 6,
        AiProviderKind::OpenAi => 7,
        AiProviderKind::Together => 8,
        AiProviderKind::DeepSeek => 9,
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
        openai_provider(),
        openrouter_provider(),
        groq_provider(),
        ollama_provider(),
        llamacpp_provider(),
        local_openai_provider(),
        vllm_provider(),
        together_provider(),
        deepseek_provider(),
        gemini_provider(),
    ]
}

/// Resolve which provider to use based on:
/// 1. OPTIDOCK_API_BASE (explicit) → Custom config with that base
/// 2. OPTIDOCK_PROVIDER env var → preset
/// 3. Saved config file (~/.optidock/provider.json)
/// 4. Default: OpenAI preset (documented behavior change from Gemini)
fn resolve_active_provider() -> AiProviderConfig {
    // 1. Explicit base URL → Custom config
    if let Ok(base) = std::env::var("OPTIDOCK_API_BASE") {
        let model = std::env::var("OPTIDOCK_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string());
        let key_env = std::env::var("OPTIDOCK_API_KEY_ENV").ok();
        let key = std::env::var("OPTIDOCK_API_KEY").ok();
        return AiProviderConfig {
            kind: AiProviderKind::Custom,
            model,
            api_base: base,
            api_key_env: key_env,
            api_key: if key.as_deref().unwrap_or("").trim().is_empty() { None } else { key },
            local: false,
        };
    }

    // 2. OPTIDOCK_PROVIDER env var
    if let Ok(provider_name) = std::env::var("OPTIDOCK_PROVIDER") {
        let kind = detect_provider_from_name(&provider_name);
        let model_override = std::env::var("OPTIDOCK_MODEL").ok();
        let mut config = provider_by_kind(kind);
        if let Some(model) = model_override {
            config.model = model;
        }
        // Apply generic key overrides if set
        if let Ok(key) = std::env::var("OPTIDOCK_API_KEY") {
            if !key.trim().is_empty() {
                config.api_key = Some(key);
            }
        }
        if let Ok(key_env) = std::env::var("OPTIDOCK_API_KEY_ENV") {
            config.api_key_env = Some(key_env);
        }
        return config;
    }

    // 3. Check saved config
    if let Some(saved) = load_saved_provider() {
        return saved;
    }

    // 4. Default: OpenAI preset
    openai_provider()
}

/// Get the config file path
fn provider_config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".optidock").join("provider.json")
}

/// Save provider choice to disk.
///
/// When `api_base` is provided, saves a Custom config pointing at that endpoint.
/// Otherwise looks up the preset by `provider_name` and applies optional model/key overrides.
pub fn save_provider_config(
    provider_name: &str,
    model: Option<&str>,
    api_base: Option<&str>,
    api_key: Option<&str>,
) -> Result<()> {
    let mut config = if let Some(base) = api_base {
        // Custom endpoint — user provided their own base URL
        AiProviderConfig {
            kind: AiProviderKind::Custom,
            model: model.unwrap_or("gpt-4.1-mini").to_string(),
            api_base: base.to_string(),
            api_key_env: None,
            api_key: api_key.map(|s| s.to_string()),
            local: false,
        }
    } else {
        let kind = detect_provider_from_name(provider_name);
        provider_by_kind(kind)
    };
    if let Some(m) = model {
        config.model = m.to_string();
    }
    if let Some(k) = api_key {
        if !k.trim().is_empty() {
            config.api_key = Some(k.to_string());
        }
    }

    let config_path = provider_config_path();
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, json)?;
    Ok(())
}

/// Load saved provider from disk; migrate legacy native-Gemini/Anthropic bases
fn load_saved_provider() -> Option<AiProviderConfig> {
    let config_path = provider_config_path();
    let content = std::fs::read_to_string(config_path).ok()?;
    let mut config: AiProviderConfig = serde_json::from_str(&content).ok()?;

    // Migration: old native Gemini base → OpenAI-compat endpoint
    if config.kind == AiProviderKind::Gemini
        && config.api_base == "https://generativelanguage.googleapis.com"
    {
        config.api_base = "https://generativelanguage.googleapis.com/v1beta/openai".to_string();
    }
    // Migration: native Anthropic base is not /chat/completions compatible → empty base
    // requires a compat gateway (OpenRouter, LiteLLM, etc.) via OPTIDOCK_API_BASE
    if config.kind == AiProviderKind::Anthropic
        && config.api_base == "https://api.anthropic.com"
    {
        config.api_base = String::new();
    }
    Some(config)
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
        AiProviderKind::Vllm => vllm_provider(),
        AiProviderKind::Together => together_provider(),
        AiProviderKind::DeepSeek => deepseek_provider(),
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

/// Detect provider kind from a user-friendly string
pub fn detect_provider_from_name(name: &str) -> AiProviderKind {
    match name.trim().to_ascii_lowercase().as_str() {
        "openai" | "chatgpt" | "gpt" | "oai" => AiProviderKind::OpenAi,
        "anthropic" | "claude" => AiProviderKind::Anthropic,
        "gemini" | "google" => AiProviderKind::Gemini,
        "openrouter" | "or" => AiProviderKind::OpenRouter,
        "groq" => AiProviderKind::Groq,
        "ollama" => AiProviderKind::Ollama,
        "llamacpp" | "llama-cpp" | "llama.cpp" | "llama" => AiProviderKind::LlamaCpp,
        "local" | "local-openai" | "lm-studio" | "lmstudio" => {
            AiProviderKind::LocalOpenAiCompatible
        }
        "together" => AiProviderKind::Together,
        "deepseek" => AiProviderKind::DeepSeek,
        "vllm" => AiProviderKind::Vllm,
        _ => AiProviderKind::Custom,
    }
}

/// Human-readable label for a provider kind
pub fn provider_label(kind: AiProviderKind) -> &'static str {
    match kind {
        AiProviderKind::OpenAi => "OpenAI",
        AiProviderKind::Anthropic => "Anthropic",
        AiProviderKind::Gemini => "Gemini (OpenAI-compat)",
        AiProviderKind::OpenRouter => "OpenRouter",
        AiProviderKind::Groq => "Groq",
        AiProviderKind::LlamaCpp => "llama.cpp",
        AiProviderKind::LocalOpenAiCompatible => "Local OpenAI-Compatible",
        AiProviderKind::Ollama => "Ollama",
        AiProviderKind::Vllm => "vLLM",
        AiProviderKind::Together => "Together",
        AiProviderKind::DeepSeek => "DeepSeek",
        AiProviderKind::Custom => "Custom",
    }
}

// ── Provider Definitions (all OpenAI-compatible /chat/completions) ──

fn openai_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::OpenAi,
        model: "gpt-4.1-mini".to_string(),
        api_base: "https://api.openai.com/v1".to_string(),
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        api_key: None,
        local: false,
    }
}

fn openrouter_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::OpenRouter,
        model: "openai/gpt-4.1-mini".to_string(),
        api_base: "https://openrouter.ai/api/v1".to_string(),
        api_key_env: Some("OPENROUTER_API_KEY".to_string()),
        api_key: None,
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
        local: false,
    }
}

fn ollama_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Ollama,
        model: "llama3.1".to_string(),
        api_base: "http://127.0.0.1:11434/v1".to_string(),
        api_key_env: None,
        api_key: None,
        local: true,
    }
}

fn llamacpp_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::LlamaCpp,
        model: "local-model".to_string(),
        api_base: "http://127.0.0.1:8080/v1".to_string(),
        api_key_env: None,
        api_key: None,
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
        local: true,
    }
}

fn vllm_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Vllm,
        model: "local-model".to_string(),
        api_base: "http://127.0.0.1:8000/v1".to_string(),
        api_key_env: None,
        api_key: None,
        local: true,
    }
}

fn together_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Together,
        model: "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo".to_string(),
        api_base: "https://api.together.xyz/v1".to_string(),
        api_key_env: Some("TOGETHER_API_KEY".to_string()),
        api_key: None,
        local: false,
    }
}

fn deepseek_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::DeepSeek,
        model: "deepseek-chat".to_string(),
        api_base: "https://api.deepseek.com/v1".to_string(),
        api_key_env: Some("DEEPSEEK_API_KEY".to_string()),
        api_key: None,
        local: false,
    }
}

fn gemini_provider() -> AiProviderConfig {
    AiProviderConfig {
        kind: AiProviderKind::Gemini,
        model: "gemini-2.0-flash".to_string(),
        // Google's OpenAI-compatible endpoint (NOT the native Gemini API)
        api_base: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
        api_key_env: Some("GEMINI_API_KEY".to_string()),
        api_key: None,
        local: false,
    }
}

fn anthropic_provider() -> AiProviderConfig {
    // Native Anthropic is NOT /chat/completions compatible.
    // Returns an empty-base config — caller MUST set OPTIDOCK_API_BASE to a
    // compatible gateway (OpenRouter, LiteLLM, etc.) to actually use it.
    AiProviderConfig {
        kind: AiProviderKind::Anthropic,
        model: "claude-sonnet-4-20250514".to_string(),
        api_base: String::new(),
        api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
        api_key: None,
        local: false,
    }
}

// ── Live AI Chat ────────────────────────────────────────────────────

/// Send a user message through the configured LLM provider and return the reply.
///
/// Resolves provider settings from the standard env/saved-config chain, then uses
/// the generic OpenAI-compatible transport (`optidock_brain::provider::openai`).
/// Returns `Err` if no base URL or key is resolvable for a non-local provider.
pub async fn live_ai_chat(user_input: &str, project_path: Option<&str>) -> Result<String> {
    let config = resolve_active_provider();

    // Build the prompt (system + user) via the shared prompt library
    let pack = build_chat_prompt(user_input, project_path);
    let system = pack.system_prompt;
    let user = pack.user_prompt;

    // Resolve the effective API key: explicit api_key field → env(api_key_env) → OPENAI_API_KEY fallback
    let api_key = resolve_api_key(&config);

    let provider = OpenAiDiagnosticProvider::new(
        &config.api_base,
        &api_key,
        &config.model,
    );

    provider.chat(&system, &user).await
}

/// Resolve the effective API key for a provider config.
///
/// Priority: explicit `api_key` → `env(api_key_env)` → `env("OPENAI_API_KEY")` fallback → empty (local).
fn resolve_api_key(config: &AiProviderConfig) -> String {
    if let Some(key) = &config.api_key {
        if !key.trim().is_empty() {
            return key.clone();
        }
    }
    if let Some(env_name) = &config.api_key_env {
        if let Ok(val) = std::env::var(env_name) {
            if !val.trim().is_empty() {
                return val;
            }
        }
    }
    // Fallback: OPENAI_API_KEY
    if let Ok(val) = std::env::var("OPENAI_API_KEY") {
        if !val.trim().is_empty() {
            return val;
        }
    }
    String::new()
}

// ── Strategy Selection ───────────────────────────────────────────────

fn select_strategy(pipeline: &PipelineContext) -> DeploymentStrategy {
    if pipeline.services.iter().any(|service| {
        matches!(
            service.traffic_profile,
            TrafficProfile::Burst | TrafficProfile::High
        )
    }) {
        DeploymentStrategy::Canary
    } else if pipeline.services.len() > 1 {
        DeploymentStrategy::Rolling
    } else {
        DeploymentStrategy::BlueGreen
    }
}

fn build_rollout_steps(pipeline: &PipelineContext, strategy: DeploymentStrategy) -> Vec<String> {
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
            steps.push(
                "Compare latency, startup stability, and error rate before full rollout."
                    .to_string(),
            );
        }
        DeploymentStrategy::Rolling => {
            steps.push(
                "Replace services incrementally to avoid full-environment interruption."
                    .to_string(),
            );
        }
        DeploymentStrategy::BlueGreen => {
            steps.push("Stand up the optimized release beside the active environment.".to_string());
            steps.push("Switch traffic only after validation passes.".to_string());
        }
        DeploymentStrategy::Recreate => {
            steps
                .push("Stop the old workload and replace it in one controlled action.".to_string());
        }
    }

    steps.push(
        "Persist deployment outcome and benchmark results for future agent decisions.".to_string(),
    );
    steps
}
