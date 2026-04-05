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
pub enum Severity {
    Info,
    Warning,
    Critical,
}
