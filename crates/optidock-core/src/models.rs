use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub path: String,
    pub dockerfile_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerfileAnalysis {
    pub context: ProjectContext,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRequest {
    pub project: ProjectContext,
    pub instructions: String,
    pub dockerfile_contents: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationProposal {
    pub provider: AiProviderKind,
    pub model: String,
    pub summary: String,
    pub optimized_dockerfile: String,
    pub assumptions: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub explanation: String,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerService {
    pub name: String,
    pub image: Option<String>,
    pub dockerfile_path: Option<String>,
    pub role: ServiceRole,
    pub deployment: DeploymentTarget,
    pub traffic_profile: TrafficProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub provider: CiProvider,
    pub repository: String,
    pub branch: String,
    pub environment: String,
    pub services: Vec<ContainerService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineModerationReport {
    pub pipeline: PipelineContext,
    pub status: PipelineStatus,
    pub summary: String,
    pub recommendations: Vec<PipelineRecommendation>,
    pub deployment_plan: DeploymentPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRecommendation {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub rationale: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub strategy: DeploymentStrategy,
    pub rollout_steps: Vec<String>,
    pub rollback_trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub kind: AiProviderKind,
    pub model: String,
    pub api_base: String,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRuntimeConfig {
    pub active_provider: AiProviderConfig,
    pub fallback_providers: Vec<AiProviderConfig>,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContextRecord {
    pub context_id: Option<i64>,
    pub user_id: i64,
    pub session_key: Option<String>,
    pub context_label: Option<String>,
    pub context_payload: Option<String>,
    pub prompt_text: String,
    pub response_text: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewChatContextRecord {
    pub email: String,
    pub session_key: Option<String>,
    pub context_label: Option<String>,
    pub context_payload: Option<String>,
    pub prompt_text: String,
    pub response_text: Option<String>,
}

// ── Enums ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ServiceRole { Api, Worker, Frontend, Database, Cache, Gateway, Unknown }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeploymentTarget { LocalDocker, RemoteDocker, EdgeNode, Unknown }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TrafficProfile { Low, Medium, High, Burst }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CiProvider { GitHubActions, GitLabCi, Jenkins, CircleCi, Unknown }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PipelineStatus { Healthy, NeedsAttention, Critical }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeploymentStrategy { Rolling, BlueGreen, Canary, Recreate }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    OpenRouter,
    Groq,
    LlamaCpp,
    LocalOpenAiCompatible,
    Ollama,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Severity { Info, Warning, Critical }

// ── Security Audit Models ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAudit {
    pub context: ProjectContext,
    pub findings: Vec<SecurityFinding>,
    pub score: u8,
    pub grade: SecurityGrade,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub category: SecurityCategory,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SecurityCategory {
    Secrets,
    Privileges,
    NetworkExposure,
    BaseImage,
    SupplyChain,
    RuntimeSafety,
    ResourceLimits,
    Misconfiguration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SecurityGrade { A, B, C, D, F }

// ── Benchmark Models ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub baseline: ImageMetrics,
    pub optimized: Option<ImageMetrics>,
    pub improvement_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetrics {
    pub tag: String,
    pub size_bytes: u64,
    pub layer_count: u32,
    pub build_time_ms: u64,
    pub build_success: bool,
}

// ── Deployment Models ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub container_id: String,
    pub image: String,
    pub name: String,
    pub port_mapping: String,
    pub status: String,
    pub started_at: String,
}

// ── Monitor Models ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
    pub created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub containers: Vec<ContainerStatus>,
    pub images: Vec<ImageInfo>,
    pub system_disk_usage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub repository: String,
    pub tag: String,
    pub image_id: String,
    pub size: String,
    pub created: String,
}

// ── Optimization Output ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedDockerfile {
    pub original_path: String,
    pub output_path: String,
    pub changes_applied: Vec<String>,
    pub original_content: String,
    pub optimized_content: String,
}
