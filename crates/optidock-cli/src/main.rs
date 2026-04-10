mod auth;

use anyhow::Result;
use auth::{
    doctor_report as db_auth_doctor_report, login_operator, recent_chat_context,
    signup_operator, store_chat_context,
};
use clap::{Parser, Subcommand};
use optidock_agent::{
    build_architecture_prompt, build_chat_prompt, default_ai_runtime_config,
    default_pipeline_context, moderate_pipeline, provider_summary, run_analysis,
};
use optidock_core::{
    AiRuntimeConfig, DeploymentStrategy, DockerfileAnalysis, NewChatContextRecord,
    PipelineModerationReport, PipelineStatus, Severity,
};
use optidock_runner::{
    command_check, evaluate_command_policy, run_shell_command, CommandExecution, CommandRisk,
};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

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
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthSession {
    email: String,
    workspace: String,
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
            ("Mode", "Oracle-backed terminal signup"),
            ("Store", "Oracle + local session"),
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
        "  {} Oracle account provisioned for {}.",
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
            ("Mode", "Oracle-backed terminal login"),
            ("Store", "Oracle + local session"),
        ],
    );

    let email = prompt_for("Email")?;
    let password = prompt_for("Password")?;
    let account = match login_operator(&email, &password).await {
        Ok(account) => account,
        Err(error) => {
            println!(
                "  {} Invalid Oracle credentials.",
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
    println!(
        "  {} none required by default",
        paint_muted("Ollama / local")
    );
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
