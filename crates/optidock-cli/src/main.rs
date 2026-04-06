use anyhow::Result;
use clap::{Parser, Subcommand};
use optidock_agent::{
    default_ai_runtime_config, default_pipeline_context, moderate_pipeline, provider_summary,
    run_analysis,
};
use optidock_core::{
    AiRuntimeConfig, DeploymentStrategy, DockerfileAnalysis, PipelineModerationReport,
    PipelineStatus, Severity,
};
use optidock_runner::command_check;
use std::{env, fs, path::Path};
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
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
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
    Doctor,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            init_project(&path)?;
        }
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
        Commands::Doctor => {
            render_doctor_report();
        }
    }

    Ok(())
}

fn init_project(path: &str) -> Result<()> {
    let root = Path::new(path);
    let dockerfile_path = root.join("Dockerfile");
    let env_path = root.join(".optidock.env");

    if !dockerfile_path.exists() {
        let starter = r#"FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

EXPOSE 3000
CMD ["npm", "start"]
"#;
        fs::write(&dockerfile_path, starter)?;
    }

    if !env_path.exists() {
        let starter = r#"OPTIDOCK_PROVIDER=openai
OPTIDOCK_FALLBACKS=openrouter,anthropic,gemini,ollama
OPENAI_API_KEY=
"#;
        fs::write(&env_path, starter)?;
    }

    print_header(
        "OptiDock AI",
        "Project bootstrap",
        &[
            ("Path", path),
            ("Dockerfile", path_or_exists(&dockerfile_path)),
            ("Config", path_or_exists(&env_path)),
        ],
    );

    print_section("Next Steps");
    println!("  {} Run `optidock doctor` to validate local tooling.", paint_accent("•"));
    println!("  {} Run `optidock analyze {path}` to inspect the Dockerfile.", paint_accent("•"));
    println!("  {} Edit `.optidock.env` to connect your preferred AI provider.", paint_accent("•"));

    Ok(())
}

fn render_doctor_report() {
    let cargo = command_check("cargo");
    let rustc = command_check("rustc");
    let docker = command_check("docker");
    let config = default_ai_runtime_config();
    let provider_env = config
        .active_provider
        .api_key_env
        .as_deref()
        .unwrap_or("no key required");

    print_header(
        "OptiDock AI",
        "Environment doctor",
        &[
            ("Active provider", provider_label_line(&config)),
            ("Expected key", provider_env),
        ],
    );

    print_section("Tooling");
    render_command_check(&cargo);
    render_command_check(&rustc);
    render_command_check(&docker);

    print_section("Provider Configuration");
    match env::var(provider_env) {
        Ok(value) if !value.trim().is_empty() => {
            println!("  {} {} is set", paint_ok(" READY "), provider_env);
        }
        _ => {
            println!(
                "  {} {} is missing or empty",
                paint_warn(" CONFIG "),
                provider_env
            );
        }
    }

    print_section("Recommended Run");
    println!("  {} `optidock analyze .`", paint_accent("•"));
    println!("  {} `optidock pipeline .`", paint_accent("•"));
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

fn render_command_check(check: &optidock_runner::CommandCheck) {
    if check.available {
        println!(
            "  {} {}",
            paint_ok(" READY "),
            pad_line(&format!("{} {}", paint_bold(&check.name), check.detail), 64)
        );
    } else {
        println!(
            "  {} {}",
            paint_warn(" MISSING "),
            pad_line(
                &format!("{} {}", paint_bold(&check.name), check.detail),
                64
            )
        );
    }
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
    let banner = ascii_banner();

    println!("{}", paint_panel_top(width));
    for line in banner {
        println!(
            "{} {}",
            paint_panel_side(),
            pad_line(&paint_brand(line), width - 4)
        );
    }
    println!("{}", paint_panel_divider(width));
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

fn ascii_banner() -> [&'static str; 5] {
    [
        "  ____        __  _ ____             _    ",
        " / __ \\____  / /_(_) __ \\____   _____| | __",
        "/ / / / __ \\/ __/ / / / / __ \\ / ___/ |/_/",
        "/ /_/ / /_/ / /_/ / /_/ / /_/ // /__/   <  ",
        "\\____/ .___/\\__/_/_____/\\____/ \\___/_/|_| ",
    ]
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

fn provider_label_line(config: &AiRuntimeConfig) -> &str {
    &config.active_provider.model
}

fn path_or_exists(path: &Path) -> &str {
    if path.exists() {
        "created"
    } else {
        "pending"
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
