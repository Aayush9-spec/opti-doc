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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ServiceRole {
    Api,
    Worker,
    Frontend,
    Database,
    Cache,
    Gateway,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeploymentTarget {
    LocalDocker,
    RemoteDocker,
    EdgeNode,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TrafficProfile {
    Low,
    Medium,
    High,
    Burst,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CiProvider {
    GitHubActions,
    GitLabCi,
    Jenkins,
    CircleCi,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PipelineStatus {
    Healthy,
    NeedsAttention,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    Rolling,
    BlueGreen,
    Canary,
    Recreate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AiProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    OpenRouter,
    LocalOpenAiCompatible,
    Ollama,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}
