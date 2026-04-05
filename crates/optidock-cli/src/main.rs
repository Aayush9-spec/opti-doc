use anyhow::Result;
use clap::{Parser, Subcommand};
use optidock_agent::{
    default_ai_runtime_config, default_pipeline_context, moderate_pipeline, provider_summary,
    run_analysis,
};
use optidock_core::{
    DeploymentStrategy, DockerfileAnalysis, PipelineModerationReport, PipelineStatus, Severity,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "optidock",
    version,
    about = "Rust-first autonomous Docker optimization agent"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Analyze {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    Pipeline {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    Providers,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { path, json } => {
            let analysis = run_analysis(&path)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&analysis)?);
            } else {
                render_analysis(&analysis);
            }
        }
        Commands::Pipeline { path, json } => {
            let report = moderate_pipeline(default_pipeline_context(&path));

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                render_pipeline_report(&report);
            }
        }
        Commands::Providers => {
            render_provider_report();
        }
    }

    Ok(())
}

fn render_provider_report() {
    let config = default_ai_runtime_config();
    let active_summary = provider_summary(&config);

    print_header(
        "OptiDock AI",
        "Provider runtime",
        &[
            ("Active", active_summary.as_str()),
            ("Timeout", "90 seconds"),
        ],
    );

    print_section("Compatible Providers");
    println!("  {} OpenAI", paint_accent("•"));
    println!("  {} Anthropic / Claude", paint_accent("•"));
    println!("  {} Gemini", paint_accent("•"));
    println!("  {} OpenRouter", paint_accent("•"));
    println!("  {} Ollama", paint_accent("•"));
    println!("  {} Local OpenAI-compatible servers", paint_accent("•"));

    print_section("Expected Environment Keys");
    println!("  {} `OPENAI_API_KEY`", paint_muted("OpenAI"));
    println!("  {} `ANTHROPIC_API_KEY`", paint_muted("Anthropic"));
    println!("  {} `GEMINI_API_KEY`", paint_muted("Gemini"));
    println!("  {} `OPENROUTER_API_KEY`", paint_muted("OpenRouter"));
    println!("  {} none required by default", paint_muted("Ollama / local"));
}

fn render_analysis(analysis: &DockerfileAnalysis) {
    print_header(
        "OptiDock AI",
        "Docker analysis",
        &[("Project", &analysis.context.path), ("Dockerfile", &analysis.context.dockerfile_path)],
    );

    print_section("Findings");

    if analysis.findings.is_empty() {
        println!("  {} No issues detected.", paint_ok("PASS"));
        return;
    }

    for (index, finding) in analysis.findings.iter().enumerate() {
        if index > 0 {
            println!();
        }

        println!(
            "  {} {}",
            severity_badge(finding.severity),
            paint_bold(&finding.title)
        );
        println!("  {} {}", paint_muted("Why"), finding.explanation);
        println!("  {} {}", paint_muted("Fix"), finding.suggested_fix);
    }
}

fn render_pipeline_report(report: &PipelineModerationReport) {
    let service_count = report.pipeline.services.len().to_string();
    print_header(
        "OptiDock AI",
        "Pipeline moderation",
        &[
            ("Repository", &report.pipeline.repository),
            ("Branch", &report.pipeline.branch),
            ("Environment", &report.pipeline.environment),
            ("Services", &service_count),
        ],
    );

    print_section("Status");
    println!(
        "  {} {}",
        pipeline_status_badge(report.status),
        report.summary
    );

    print_section("Deployment Plan");
    println!(
        "  {} {}",
        paint_muted("Strategy"),
        strategy_label(report.deployment_plan.strategy)
    );
    for step in &report.deployment_plan.rollout_steps {
        println!("  {} {}", paint_accent("•"), step);
    }
    println!(
        "  {} {}",
        paint_muted("Rollback"),
        report.deployment_plan.rollback_trigger
    );

    if !report.recommendations.is_empty() {
        print_section("Recommendations");
        for (index, recommendation) in report.recommendations.iter().enumerate() {
            if index > 0 {
                println!();
            }

            println!(
                "  {} {}",
                severity_badge(recommendation.severity),
                paint_bold(&recommendation.title)
            );
            println!("  {} {}", paint_muted("Why"), recommendation.rationale);
            println!("  {} {}", paint_muted("Action"), recommendation.action);
        }
    } else {
        print_section("Recommendations");
        println!("  {} No moderation issues detected.", paint_ok("PASS"));
    }
}

fn print_header(title: &str, subtitle: &str, fields: &[(&str, &str)]) {
    let width = 78;
    println!("{}", paint_panel_top(width));
    println!(
        "{} {}",
        paint_panel_side(),
        pad_line(&format!("{}  {}", paint_brand(title), paint_muted(subtitle)), width - 4)
    );
    println!("{}", paint_panel_divider(width));

    for (label, value) in fields {
        println!(
            "{} {}",
            paint_panel_side(),
            pad_line(&format!("{} {}", paint_muted(label), value), width - 4)
        );
    }

    println!("{}", paint_panel_bottom(width));
}

fn print_section(title: &str) {
    println!();
    println!("{} {}", paint_accent("●"), paint_bold(title));
}

fn severity_badge(severity: Severity) -> String {
    match severity {
        Severity::Info => paint_info(" INFO "),
        Severity::Warning => paint_warn(" WARN "),
        Severity::Critical => paint_critical(" CRIT "),
    }
}

fn pipeline_status_badge(status: PipelineStatus) -> String {
    match status {
        PipelineStatus::Healthy => paint_ok(" HEALTHY "),
        PipelineStatus::NeedsAttention => paint_warn(" ATTENTION "),
        PipelineStatus::Critical => paint_critical(" CRITICAL "),
    }
}

fn strategy_label(strategy: DeploymentStrategy) -> &'static str {
    match strategy {
        DeploymentStrategy::Rolling => "Rolling",
        DeploymentStrategy::BlueGreen => "Blue/Green",
        DeploymentStrategy::Canary => "Canary",
        DeploymentStrategy::Recreate => "Recreate",
    }
}

fn pad_line(content: &str, width: usize) -> String {
    let visible = strip_ansi(content).chars().count();
    let padding = width.saturating_sub(visible);
    format!("{content}{}", " ".repeat(padding))
}

fn strip_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            while let Some(next) = chars.next() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn paint_brand(value: &str) -> String {
    format!("\x1b[1;96m{value}\x1b[0m")
}

fn paint_bold(value: &str) -> String {
    format!("\x1b[1m{value}\x1b[0m")
}

fn paint_muted(value: &str) -> String {
    format!("\x1b[2;37m{value}\x1b[0m")
}

fn paint_accent(value: &str) -> String {
    format!("\x1b[1;94m{value}\x1b[0m")
}

fn paint_ok(value: &str) -> String {
    format!("\x1b[1;30;102m{value}\x1b[0m")
}

fn paint_info(value: &str) -> String {
    format!("\x1b[1;30;106m{value}\x1b[0m")
}

fn paint_warn(value: &str) -> String {
    format!("\x1b[1;30;103m{value}\x1b[0m")
}

fn paint_critical(value: &str) -> String {
    format!("\x1b[1;97;101m{value}\x1b[0m")
}

fn paint_panel_top(width: usize) -> String {
    format!(
        "{}{}{}",
        paint_accent("╭"),
        paint_accent(&"─".repeat(width.saturating_sub(2))),
        paint_accent("╮")
    )
}

fn paint_panel_divider(width: usize) -> String {
    format!(
        "{}{}{}",
        paint_accent("├"),
        paint_accent(&"─".repeat(width.saturating_sub(2))),
        paint_accent("┤")
    )
}

fn paint_panel_bottom(width: usize) -> String {
    format!(
        "{}{}{}",
        paint_accent("╰"),
        paint_accent(&"─".repeat(width.saturating_sub(2))),
        paint_accent("╯")
    )
}

fn paint_panel_side() -> String {
    paint_accent("│")
}
