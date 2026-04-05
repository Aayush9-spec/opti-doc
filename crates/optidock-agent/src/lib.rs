use anyhow::Result;
use optidock_analyzer::analyze_project;
use optidock_core::{
    CiProvider, ContainerService, DeploymentPlan, DeploymentStrategy, DeploymentTarget,
    DockerfileAnalysis, PipelineContext, PipelineModerationReport, PipelineRecommendation,
    PipelineStatus, ServiceRole, Severity, TrafficProfile,
};

pub fn run_analysis(path: &str) -> Result<DockerfileAnalysis> {
    analyze_project(path)
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
