mod auth;

use anyhow::Result;
use auth::{
    doctor_report as db_auth_doctor_report, login_operator, recent_chat_context,
    signup_operator, store_chat_context,
};
use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use clap::{Parser, Subcommand};
use optidock_agent::{
    all_providers, build_architecture_prompt, build_chat_prompt, default_ai_runtime_config,
    default_pipeline_context, moderate_pipeline, provider_label, provider_summary, run_analysis,
    run_optimize, run_security_scan, save_provider_config,
};
use optidock_core::{
    AiProviderConfig, AiProviderKind, AiRuntimeConfig, BenchmarkResult, DeploymentRecord,
    DeploymentStrategy, DockerfileAnalysis, MonitorSnapshot, NewChatContextRecord,
    OptimizedDockerfile, PipelineModerationReport, PipelineStatus, SecurityAudit, SecurityCategory,
    SecurityGrade, Severity,
};
use optidock_runner::{
    command_check, docker_benchmark, docker_deploy, docker_monitor, docker_rollback,
    evaluate_command_policy, run_shell_command, CommandExecution, CommandRisk,
};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
};
use tower_http::cors::CorsLayer;
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
    Signup,
    Login,
    Logout,
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
    Live {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Start a lightweight HTTP API server (used by Cloud Run)
    Serve {
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Generate an optimized Dockerfile from the current one
    Optimize {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Run a security audit on the Dockerfile and container configuration
    Security {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Build baseline and optimized images, compare metrics
    Benchmark {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Deploy a container locally with resource limits and monitoring
    Deploy {
        image: String,
        #[arg(long, default_value = "optidock-app")]
        name: String,
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Show running containers, images, and Docker system status
    Monitor {
        #[arg(long)]
        json: bool,
    },
    /// Stop and remove a deployed container
    Rollback {
        name: String,
    },
    /// Switch the active LLM provider
    Config {
        /// Provider name: gemini, openai, anthropic, openrouter, groq, ollama, llamacpp, local
        #[arg(long)]
        provider: Option<String>,
        /// Model override (e.g. gpt-4o, claude-sonnet-4-20250514)
        #[arg(long)]
        model: Option<String>,
        /// List all supported providers
        #[arg(long)]
        list: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if std::env::var("K_SERVICE").is_ok() {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
        let formatting_layer = tracing_bunyan_formatter::BunyanFormattingLayer::new(
            "optidock-cli".into(),
            std::io::stdout,
        );
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_bunyan_formatter::JsonStorageLayer)
            .with(formatting_layer)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .without_time()
            .init();
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            init_project(&path)?;
        }
        Commands::Signup => {
            signup_user().await?;
        }
        Commands::Login => {
            login_user().await?;
        }
        Commands::Logout => {
            logout_user()?;
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
            render_doctor_report().await;
        }
        Commands::Live { path } => {
            run_live_session(&path).await?;
        }
        Commands::Serve { port } => {
            run_http_server(port).await?;
        }
        Commands::Optimize { path, json } => {
            let result = run_optimize(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                render_optimize_report(&result);
            }
        }
        Commands::Security { path, json } => {
            let audit = run_security_scan(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&audit)?);
            } else {
                render_security_report(&audit);
            }
        }
        Commands::Benchmark { path, json } => {
            let result = docker_benchmark(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                render_benchmark_report(&result);
            }
        }
        Commands::Deploy { image, name, port } => {
            let record = docker_deploy(&image, &name, port)?;
            render_deploy_report(&record);
        }
        Commands::Monitor { json } => {
            let snapshot = docker_monitor()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                render_monitor_report(&snapshot);
            }
        }
        Commands::Rollback { name } => {
            let msg = docker_rollback(&name)?;
            print_section("Rollback");
            println!("  {} {}", paint_ok(" DONE "), msg);
        }
        Commands::Config { provider, model, list } => {
            if list {
                render_provider_list();
            } else if let Some(ref p) = provider {
                save_provider_config(p, model.as_deref())?;
                let config = default_ai_runtime_config();
                print_section("Provider Updated");
                println!(
                    "  {} Active: {} ({})",
                    paint_ok(" SET "),
                    provider_label(config.active_provider.kind),
                    config.active_provider.model
                );
            } else {
                render_provider_report();
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthSession {
    email: String,
    workspace: String,
}

#[derive(Debug, Clone)]
struct LiveUiState {
    workspace: String,
    auth_status: String,
    api_status: String,
    last_command: Option<CommandExecution>,
}

async fn run_live_session(path: &str) -> Result<()> {
    print_live_header(path);
    print_section("Live Mode");
    println!(
        "  {} Chat-style local agent loop is active.",
        paint_accent("•")
    );
    println!("  {} Use `/help` to see commands.", paint_accent("•"));
    println!("  {} Use `/exit` to leave the session.", paint_accent("•"));

    let stdin = io::stdin();
    loop {
        print!("{}", paint_prompt("optidock>"));
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes_read = stdin.read_line(&mut input)?;
        if bytes_read == 0 {
            println!();
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" | "exit" | "quit" => {
                println!("{} Session closed.", paint_muted("optidock"));
                break;
            }
            "/help" | "help" => render_live_help(),
            "/doctor" => render_doctor_report().await,
            "/providers" => render_provider_report(),
            "/analyze" => {
                let analysis = run_analysis(path)?;
                render_analysis(&analysis);
            }
            "/pipeline" => {
                let report = moderate_pipeline(default_pipeline_context(path));
                render_pipeline_report(&report);
            }
            _ if input.starts_with("/run ") => {
                let command = input.trim_start_matches("/run ").trim();
                run_and_render_command(command)?;
            }
            _ if input.starts_with("/analyze ") => {
                let target = input.trim_start_matches("/analyze ").trim();
                let analysis = run_analysis(target)?;
                render_analysis(&analysis);
            }
            _ if input.starts_with("/pipeline ") => {
                let target = input.trim_start_matches("/pipeline ").trim();
                let report = moderate_pipeline(default_pipeline_context(target));
                render_pipeline_report(&report);
            }
            _ => {
                render_live_agent_response(input, path).await?;
            }
        }
    }

    Ok(())
}

// ── HTTP Serve Mode (Cloud Run) ──────────────────────────────────────

async fn run_http_server(port: u16) -> Result<()> {
    print_header(
        "OptiDock AI",
        "HTTP serve mode",
        &[
            ("Port", &port.to_string()),
            ("Mode", "Cloud Run API"),
        ],
    );

    let app = Router::new()
        .route("/", get(handle_root))
        .route("/health", get(handle_health))
        .route("/analyze", get(handle_analyze))
        .route("/pipeline", get(handle_pipeline))
        .route("/providers", get(handle_providers))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("  {} Listening on http://{}", paint_ok(" READY "), addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Serialize)]
struct ApiStatus {
    service: &'static str,
    version: &'static str,
    status: &'static str,
    endpoints: Vec<&'static str>,
}

async fn handle_root() -> impl IntoResponse {
    Json(ApiStatus {
        service: "OptiDock AI",
        version: env!("CARGO_PKG_VERSION"),
        status: "running",
        endpoints: vec!["/", "/health", "/analyze", "/pipeline", "/providers"],
    })
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

#[derive(Deserialize)]
struct AnalyzeParams {
    path: Option<String>,
}

async fn handle_analyze(Query(params): Query<AnalyzeParams>) -> impl IntoResponse {
    let path = params.path.unwrap_or_else(|| "./sample".to_string());
    match run_analysis(&path) {
        Ok(analysis) => (StatusCode::OK, Json(serde_json::to_value(analysis).unwrap())),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}

async fn handle_pipeline() -> impl IntoResponse {
    let ctx = default_pipeline_context("./sample");
    let report = moderate_pipeline(ctx);
    Json(serde_json::to_value(report).unwrap())
}

async fn handle_providers() -> impl IntoResponse {
    let config = default_ai_runtime_config();
    let summary = provider_summary(&config);
    Json(serde_json::json!({
        "active_provider": config.active_provider.model,
        "summary": summary,
        "compatible": [
            "OpenAI", "Anthropic / Claude", "Gemini",
            "OpenRouter", "Ollama", "Local OpenAI-compatible"
        ]
    }))
}


fn render_live_help() {
    print_section("Available Commands");
    println!(
        "  {} `optidock signup` create a local operator account",
        paint_accent("•")
    );
    println!(
        "  {} `optidock login` sign in from the terminal",
        paint_accent("•")
    );
    println!(
        "  {} `optidock logout` clear the local session",
        paint_accent("•")
    );
    println!(
        "  {} `/help` show available live-mode commands",
        paint_accent("•")
    );
    println!(
        "  {} `/doctor` inspect local runtime readiness",
        paint_accent("•")
    );
    println!(
        "  {} `/providers` list provider runtime information",
        paint_accent("•")
    );
    println!(
        "  {} `/analyze [path]` analyze a Docker project",
        paint_accent("•")
    );
    println!(
        "  {} `/pipeline [path]` moderate rollout strategy",
        paint_accent("•")
    );
    println!(
        "  {} `/run <command>` execute a shell command live",
        paint_accent("•")
    );
    println!(
        "  {} `/exit` close the live agent session",
        paint_accent("•")
    );
}

async fn render_live_agent_response(input: &str, path: &str) -> Result<()> {
    print_section("Agent");

    if looks_like_shell_command(input) {
        println!(
            "  {} That looks like a shell command. Running it directly.",
            paint_muted("Decision")
        );
        run_and_render_command(input)?;
        return Ok(());
    }

    if input.contains("analy") || input.contains("dockerfile") {
        println!(
            "  {} Interpreting your request as a project analysis for `{}`.",
            paint_muted("Decision"),
            path
        );
        let analysis = run_analysis(path)?;
        render_analysis(&analysis);
        return Ok(());
    }

    if input.contains("pipeline") || input.contains("deploy") || input.contains("rollout") {
        println!(
            "  {} Interpreting your request as a pipeline moderation pass for `{}`.",
            paint_muted("Decision"),
            path
        );
        let report = moderate_pipeline(default_pipeline_context(path));
        render_pipeline_report(&report);
        return Ok(());
    }

    println!(
        "  {} I can help with project inspection, pipeline moderation, and live shell commands.",
        paint_muted("Guide")
    );
    let prompt_pack = build_chat_prompt(input, Some(path));
    let architecture_pack = build_architecture_prompt(input, Some(path), Some("live-agent-response"));
    println!(
        "  {} Using saved prompt preset `{}`.",
        paint_muted("Prompt"),
        prompt_pack.prompt_id
    );
    println!(
        "  {} Architecture preset available as `{}`.",
        paint_muted("Prompt"),
        architecture_pack.prompt_id
    );
    println!(
        "  {} Try `/analyze`, `/pipeline`, or `/run cargo test`.",
        paint_muted("Next")
    );

    persist_live_chat_context(input, path, &prompt_pack.prompt_id).await;

    Ok(())
}

fn run_and_render_command(command: &str) -> Result<()> {
    print_section("Command Execution");
    println!("  {} {}", paint_muted("Command"), command);

    let policy = evaluate_command_policy(command);
    if matches!(policy.risk, CommandRisk::NeedsApproval) && !confirm_unsafe_command(command, policy.reason.as_deref())? {
        println!(
            "  {} Command blocked until the operator grants permission.",
            paint_warn(" BLOCKED ")
        );
        return Ok(());
    }

    let result = run_shell_command(command)?;
    render_command_result(&result);

    Ok(())
}

fn render_command_result(result: &CommandExecution) {
    let status_badge = if result.success {
        paint_ok(" SUCCESS ")
    } else {
        paint_warn(" FAILED ")
    };

    println!("  {} exit status {}", status_badge, result.status);

    if !result.stdout.is_empty() {
        println!("  {}", paint_bold("stdout"));
        for line in result.stdout.lines() {
            println!("    {}", line);
        }
    }

    if !result.stderr.is_empty() {
        println!("  {}", paint_bold("stderr"));
        for line in result.stderr.lines() {
            println!("    {}", line);
        }
    }
}

fn print_live_header(path: &str) {
    let auth_status = load_session()
        .map(|session| format!("Signed in as {}", session.email))
        .unwrap_or_else(|_| "Not signed in".to_string());

    print_header(
        "OptiDock Live",
        "Interactive agent terminal",
        &[
            ("Workspace", path),
            ("Mode", "Chat + terminal execution"),
            ("Auth", &auth_status),
        ],
    );
}

fn looks_like_shell_command(input: &str) -> bool {
    [
        "cargo ", "git ", "docker ", "npm ", "pnpm ", "ls", "pwd", "cat ", "sed ", "find ", "rg ",
        "./",
    ]
    .iter()
    .any(|prefix| input.starts_with(prefix))
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
NEXT_PUBLIC_SUPABASE_URL=
NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY=
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
    println!(
        "  {} Run `optidock doctor` to validate local tooling.",
        paint_accent("•")
    );
    println!(
        "  {} Run `optidock analyze {path}` to inspect the Dockerfile.",
        paint_accent("•")
    );
    println!(
        "  {} Edit `.optidock.env` to connect your preferred AI provider.",
        paint_accent("•")
    );

    Ok(())
}

async fn signup_user() -> Result<()> {
    print_header(
        "OptiDock Auth",
        "Create operator account",
        &[
            ("Mode", "Supabase-backed terminal signup"),
            ("Store", "Supabase + local session"),
        ],
    );

    let full_name = prompt_for("Full name")?;
    let email = prompt_for("Email")?;
    let workspace = prompt_for("Workspace name")?;
    let password = prompt_for("Password")?;

    let account = signup_operator(full_name, email.clone(), workspace.clone(), password).await?;
    save_session(&AuthSession { email, workspace })?;

    print_section("Signup Complete");
    println!(
        "  {} Supabase account provisioned for {}.",
        paint_ok(" READY "),
        account.email
    );
    println!("  {} You are now signed in to OptiDock.", paint_accent("•"));

    Ok(())
}

async fn login_user() -> Result<()> {
    print_header(
        "OptiDock Auth",
        "Operator login",
        &[
            ("Mode", "Supabase-backed terminal login"),
            ("Store", "Supabase + local session"),
        ],
    );

    let email = prompt_for("Email")?;
    let password = prompt_for("Password")?;
    let account = match login_operator(&email, &password).await {
        Ok(account) => account,
        Err(error) => {
            println!(
                "  {} Invalid Supabase credentials.",
                paint_critical(" DENIED ")
            );
            return Err(error);
        }
    };

    save_session(&AuthSession {
        email: account.email.clone(),
        workspace: account.workspace.clone(),
    })?;

    print_section("Login Complete");
    println!("  {} Signed in as {}", paint_ok(" READY "), account.email);
    println!(
        "  {} Workspace {}",
        paint_muted("Workspace"),
        account.workspace
    );

    Ok(())
}

fn logout_user() -> Result<()> {
    let session_path = auth_session_path()?;
    if session_path.exists() {
        fs::remove_file(&session_path)?;
    }

    print_header(
        "OptiDock Auth",
        "Operator logout",
        &[
            ("Mode", "Local terminal logout"),
            ("Status", "Session cleared"),
        ],
    );
    print_section("Logout Complete");
    println!("  {} Local session removed.", paint_ok(" DONE "));

    Ok(())
}

async fn render_doctor_report() {
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

    print_section("Authentication");
    match load_session() {
        Ok(session) => {
            println!(
                "  {} {} on {}",
                paint_ok(" SIGNED IN "),
                session.email,
                session.workspace
            );
        }
        Err(_) => {
            println!("  {} no local OptiDock session", paint_warn(" AUTH "));
        }
    }

    let db_report = db_auth_doctor_report().await;
    print_section("Supabase Auth");
    if !db_report.env_ready {
        println!("  {} {}", paint_warn(" CONFIG "), db_report.detail);
    } else if db_report.connection_ready {
        println!("  {} {}", paint_ok(" READY "), db_report.detail);
    } else {
        println!("  {} {}", paint_warn(" DB "), db_report.detail);
    }

    print_section("Recommended Run");
    println!("  {} `optidock signup`", paint_accent("•"));
    println!("  {} `optidock login`", paint_accent("•"));
    println!("  {} `optidock analyze .`", paint_accent("•"));
    println!("  {} `optidock pipeline .`", paint_accent("•"));
    if let Ok(session) = load_session() {
        match recent_chat_context(&session.email, 3).await {
            Ok(records) if !records.is_empty() => {
                print_section("Recent Supabase Chat Context");
                for record in records {
                    let label = record
                        .context_label
                        .unwrap_or_else(|| "chat-entry".to_string());
                    println!(
                        "  {} {}",
                        paint_accent("â€¢"),
                        format!("{label} -> {}", truncate_line(&record.prompt_text, 56))
                    );
                }
            }
            Ok(_) => {}
            Err(error) => {
                print_section("Recent Supabase Chat Context");
                println!("  {} {}", paint_warn(" DB "), error);
            }
        }
    }
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
            pad_line(&format!("{} {}", paint_bold(&check.name), check.detail), 64)
        );
    }
}

fn render_analysis(analysis: &DockerfileAnalysis) {
    print_header(
        "OptiDock AI",
        "Docker analysis",
        &[
            ("Project", &analysis.context.path),
            ("Dockerfile", &analysis.context.dockerfile_path),
        ],
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

// ── Provider Reports ─────────────────────────────────────────────────

fn render_provider_report() {
    let config = default_ai_runtime_config();
    print_header(
        "OptiDock AI",
        "LLM provider configuration",
        &[
            ("Active", provider_label(config.active_provider.kind)),
            ("Model", &config.active_provider.model),
        ],
    );

    print_section("Active Provider");
    render_single_provider(&config.active_provider, true);

    print_section("Fallback Chain");
    for provider in &config.fallback_providers {
        render_single_provider(provider, false);
    }

    print_section("Switch Provider");
    println!("  {} `optidock config --provider gemini`", paint_accent("•"));
    println!("  {} `optidock config --provider openai --model gpt-4o`", paint_accent("•"));
    println!("  {} `optidock config --list` to see all options", paint_accent("•"));
    println!("  {} Or set OPTIDOCK_PROVIDER=anthropic in your env", paint_accent("•"));
}

fn render_provider_list() {
    let providers = all_providers();

    print_header(
        "OptiDock AI",
        "Supported LLM providers",
        &[
            ("Total", &providers.len().to_string()),
            ("Default", "Gemini 2.0 Flash (free)"),
        ],
    );

    let config = default_ai_runtime_config();

    print_section("All Providers");
    for p in &providers {
        let active_marker = if p.kind == config.active_provider.kind {
            paint_ok(" ACTIVE ")
        } else {
            "        ".to_string()
        };

        let tier = match p.kind {
            AiProviderKind::Gemini => paint_ok(" FREE "),
            AiProviderKind::Groq => paint_ok(" FREE "),
            AiProviderKind::Ollama | AiProviderKind::LlamaCpp | AiProviderKind::LocalOpenAiCompatible => paint_accent(" LOCAL "),
            _ => paint_warn(" PAID "),
        };

        println!(
            "  {} {} {} ({})",
            active_marker,
            tier,
            paint_bold(provider_label(p.kind)),
            p.model
        );
        println!(
            "          {} {}",
            paint_muted("API"),
            p.api_base
        );
        if let Some(ref key_env) = p.api_key_env {
            println!("          {} {}", paint_muted("Key"), key_env);
        } else {
            println!("          {} No API key needed", paint_muted("Key"));
        }
        println!();
    }

    print_section("Usage");
    println!("  {} Set provider: `optidock config --provider gemini`", paint_accent("•"));
    println!("  {} Override model: `optidock config --provider openai --model gpt-4o`", paint_accent("•"));
    println!("  {} Env override: `OPTIDOCK_PROVIDER=anthropic optidock live .`", paint_accent("•"));
}

fn render_single_provider(p: &AiProviderConfig, active: bool) {
    let badge = if active { paint_ok(" ● ") } else { "   ".to_string() };
    println!(
        "  {} {} — {} ({})",
        badge,
        paint_bold(provider_label(p.kind)),
        p.model,
        if p.local { "local" } else { "cloud" }
    );
    println!("  {}   API: {}", "", p.api_base);
    if let Some(ref key) = p.api_key_env {
        let has_key = std::env::var(key).is_ok();
        let status = if has_key { paint_ok(" SET ") } else { paint_warn(" MISSING ") };
        println!("  {}   Key: {} {}", "", key, status);
    }
}

// ── Security Report ──────────────────────────────────────────────────

fn render_security_report(audit: &SecurityAudit) {
    let score_str = audit.score.to_string();
    let grade_str = security_grade_label(audit.grade);
    let count = audit.findings.len().to_string();

    print_header(
        "OptiDock AI",
        "Security audit",
        &[
            ("Project", &audit.context.path),
            ("Score", &format!("{}/100", score_str)),
            ("Grade", grade_str),
            ("Findings", &count),
        ],
    );

    print_section("Summary");
    println!(
        "  {} {}",
        security_grade_badge(audit.grade),
        audit.summary
    );

    if audit.findings.is_empty() {
        print_section("Results");
        println!("  {} No security issues detected.", paint_ok("PASS"));
        return;
    }

    print_section("Findings");
    for (i, f) in audit.findings.iter().enumerate() {
        if i > 0 { println!(); }
        println!(
            "  {} [{}] {}",
            severity_badge(f.severity),
            security_cat_label(f.category),
            paint_bold(&f.title)
        );
        println!("  {} {}", paint_muted("Detail"), f.detail);
        println!("  {} {}", paint_muted("Fix"), f.remediation);
    }
}

fn security_grade_badge(grade: SecurityGrade) -> String {
    match grade {
        SecurityGrade::A => paint_ok(" A "),
        SecurityGrade::B => paint_ok(" B "),
        SecurityGrade::C => paint_warn(" C "),
        SecurityGrade::D => paint_warn(" D "),
        SecurityGrade::F => paint_critical(" F "),
    }
}

fn security_grade_label(grade: SecurityGrade) -> &'static str {
    match grade {
        SecurityGrade::A => "A — Excellent",
        SecurityGrade::B => "B — Good",
        SecurityGrade::C => "C — Fair",
        SecurityGrade::D => "D — Poor",
        SecurityGrade::F => "F — Critical",
    }
}

fn security_cat_label(cat: SecurityCategory) -> &'static str {
    match cat {
        SecurityCategory::Secrets => "SECRETS",
        SecurityCategory::Privileges => "PRIVESC",
        SecurityCategory::NetworkExposure => "NETWORK",
        SecurityCategory::BaseImage => "IMAGE",
        SecurityCategory::SupplyChain => "SUPPLY",
        SecurityCategory::RuntimeSafety => "RUNTIME",
        SecurityCategory::ResourceLimits => "RESOURCES",
        SecurityCategory::Misconfiguration => "CONFIG",
    }
}

// ── Optimize Report ──────────────────────────────────────────────────

fn render_optimize_report(result: &OptimizedDockerfile) {
    print_header(
        "OptiDock AI",
        "Dockerfile optimization",
        &[
            ("Original", &result.original_path),
            ("Output", &result.output_path),
        ],
    );

    let change_count = result.changes_applied.len().to_string();
    print_section(&format!("Changes Applied ({})", change_count));
    for change in &result.changes_applied {
        println!("  {} {}", paint_accent("•"), change);
    }

    print_section("Next Steps");
    println!("  {} Review the optimized Dockerfile at `{}`", paint_accent("•"), result.output_path);
    println!("  {} Run `optidock benchmark` to compare image sizes", paint_accent("•"));
    println!("  {} Run `optidock security` to verify security posture", paint_accent("•"));
}

// ── Benchmark Report ─────────────────────────────────────────────────

fn render_benchmark_report(result: &BenchmarkResult) {
    print_header(
        "OptiDock AI",
        "Docker benchmark",
        &[
            ("Baseline", &result.baseline.tag),
            ("Build success", if result.baseline.build_success { "yes" } else { "no" }),
        ],
    );

    print_section("Baseline Metrics");
    render_image_metrics(&result.baseline);

    if let Some(ref opt) = result.optimized {
        print_section("Optimized Metrics");
        render_image_metrics(opt);
    }

    print_section("Summary");
    println!("  {} {}", paint_accent("•"), result.improvement_summary);
}

fn render_image_metrics(m: &optidock_core::ImageMetrics) {
    let size_mb = m.size_bytes as f64 / 1_048_576.0;
    println!("  {} {}", paint_muted("Tag"), m.tag);
    println!("  {} {:.1} MB ({} bytes)", paint_muted("Size"), size_mb, m.size_bytes);
    println!("  {} {}", paint_muted("Layers"), m.layer_count);
    println!("  {} {} ms", paint_muted("Build time"), m.build_time_ms);
    println!(
        "  {} {}",
        paint_muted("Status"),
        if m.build_success { paint_ok(" SUCCESS ") } else { paint_critical(" FAILED ") }
    );
}

// ── Deploy Report ────────────────────────────────────────────────────

fn render_deploy_report(record: &DeploymentRecord) {
    print_header(
        "OptiDock AI",
        "Container deployed",
        &[
            ("Container", &record.container_id),
            ("Image", &record.image),
            ("Name", &record.name),
            ("Ports", &record.port_mapping),
        ],
    );

    print_section("Status");
    println!("  {} Container is running.", paint_ok(" LIVE "));
    println!("  {} Started at {}", paint_muted("Time"), record.started_at);
    println!("  {} Resource limits: 512MB RAM, 1 CPU", paint_muted("Limits"));

    print_section("Management");
    println!("  {} `optidock monitor` to check status", paint_accent("•"));
    println!("  {} `optidock rollback {}` to stop and remove", paint_accent("•"), record.name);
}

// ── Monitor Report ───────────────────────────────────────────────────

fn render_monitor_report(snapshot: &MonitorSnapshot) {
    let container_count = snapshot.containers.len().to_string();
    let image_count = snapshot.images.len().to_string();

    print_header(
        "OptiDock AI",
        "Container monitor",
        &[
            ("Containers", &container_count),
            ("Images", &image_count),
        ],
    );

    print_section("Running Containers");
    if snapshot.containers.is_empty() {
        println!("  {} No containers found.", paint_muted("empty"));
    } else {
        for c in &snapshot.containers {
            let status_badge = if c.status.contains("Up") {
                paint_ok(" UP ")
            } else {
                paint_warn(" DOWN ")
            };
            println!(
                "  {} {} {} ({})",
                status_badge,
                paint_bold(&c.name),
                paint_muted(&c.image),
                c.ports
            );
        }
    }

    print_section("Local Images");
    if snapshot.images.is_empty() {
        println!("  {} No images found.", paint_muted("empty"));
    } else {
        for img in snapshot.images.iter().take(10) {
            println!(
                "  {} {}:{} ({})",
                paint_accent("•"),
                img.repository,
                img.tag,
                img.size
            );
        }
        if snapshot.images.len() > 10 {
            println!("  {} ... and {} more", paint_muted(""), snapshot.images.len() - 10);
        }
    }

    if let Some(ref disk) = snapshot.system_disk_usage {
        print_section("Disk Usage");
        for line in disk.lines() {
            println!("  {} {}", paint_accent("•"), line);
        }
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
        pad_line(
            &format!("{}  {}", paint_brand(title), paint_muted(subtitle)),
            width - 4
        )
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

fn auth_root() -> Result<PathBuf> {
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;
    let root = Path::new(&home).join(".optidock").join("auth");
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn auth_session_path() -> Result<PathBuf> {
    Ok(auth_root()?.join("session.json"))
}

fn save_session(session: &AuthSession) -> Result<()> {
    let payload = serde_json::to_string_pretty(session)?;
    fs::write(auth_session_path()?, payload)?;
    Ok(())
}

fn load_session() -> Result<AuthSession> {
    let payload = fs::read_to_string(auth_session_path()?)?;
    Ok(serde_json::from_str(&payload)?)
}

fn prompt_for(label: &str) -> Result<String> {
    print!("{} {}: ", paint_muted("input"), label);
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

fn confirm_unsafe_command(command: &str, reason: Option<&str>) -> Result<bool> {
    print_section("Permission Required");
    println!(
        "  {} {}",
        paint_warn(" UNSAFE "),
        "This command needs explicit operator approval."
    );
    println!("  {} {}", paint_muted("Command"), command);

    if let Some(reason) = reason {
        println!("  {} {}", paint_muted("Reason"), reason);
    }

    print!("{} Approve? Type `yes` to continue: ", paint_muted("input"));
    io::stdout().flush()?;

    let mut approval = String::new();
    io::stdin().read_line(&mut approval)?;
    Ok(approval.trim().eq_ignore_ascii_case("yes"))
}

async fn persist_live_chat_context(input: &str, path: &str, prompt_id: &str) {
    let Ok(session) = load_session() else {
        return;
    };

    let record = NewChatContextRecord {
        email: session.email,
        session_key: Some("live-mode".to_string()),
        context_label: Some(prompt_id.to_string()),
        context_payload: Some(path.to_string()),
        prompt_text: input.to_string(),
        response_text: Some(format!("Preset selected: {prompt_id}")),
    };

    if let Err(error) = store_chat_context(record).await {
        println!(
            "  {} {}",
            paint_warn(" DB "),
            format!("Supabase chat persistence skipped: {error}")
        );
    }
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

fn truncate_line(content: &str, max_chars: usize) -> String {
    let mut chars = content.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
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

fn paint_prompt(value: &str) -> String {
    format!("\x1b[1;97;100m {value} \x1b[0m ")
}
