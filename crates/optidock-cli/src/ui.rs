//! Theme-aware rendering helpers for the CLI.
//!
//! Delegates semantic color tokens, the statement prompt, the status bar, and
//! ASCII art to `optidock-tui`, while keeping the compact `paint_*` API used
//! throughout `main.rs`. Paint helpers resolve against a single lazily-created
//! `Theme` so every output section uses the same palette.

use optidock_tui::{
    art::{self, ArtLibrary},
    statusbar::{StatusBar},
    theme::{ColorToken, Theme},
    prompt::{PromptContext, RichPrompt},
};

/// Set once per process, used for every paint call.
static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();

/// The active theme (defaults to Dark when TERM is dumb/unknown).
pub fn theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let kind = optidock_tui::ThemeKind::detect();
        match kind {
            optidock_tui::ThemeKind::Light => Theme::light(),
            _ => Theme::dark(),
        }
    })
}

/// Wrap `value` in an ANSI foreground color for the given semantic token.
pub fn paint(token: ColorToken, value: &str) -> String {
    let c = theme().crossterm(token);
    let code = match c {
        crossterm::style::Color::Rgb { r, g, b } => format!("\x1b[38;2;{};{};{}m", r, g, b),
        _ => String::new(),
    };
    format!("{}{}\x1b[0m", code, value)
}

/// Paint a value bolded and colored.
pub fn paint_bold_color(token: ColorToken, value: &str) -> String {
    let c = theme().crossterm(token);
    let code = match c {
        crossterm::style::Color::Rgb { r, g, b } => format!("\x1b[1;38;2;{};{};{}m", r, g, b),
        _ => "\x1b[1m".to_string(),
    };
    format!("{}{}\x1b[0m", code, value)
}

/// A colored "chip" — inverse-ish background badge for status labels.
pub fn paint_badge(token: ColorToken, value: &str) -> String {
    let c = theme().crossterm(token);
    let code = match c {
        crossterm::style::Color::Rgb { r, g, b } => format!("\x1b[1;38;2;{};{};{}m", r, g, b),
        _ => String::new(),
    };
    format!("{}{}\x1b[0m", code, value)
}

// ─── Back-compat aliases used across main.rs ────────────────────────────────

pub fn paint_brand(value: &str) -> String {
    paint_bold_color(ColorToken::BrandPrimary, value)
}

pub fn paint_accent(value: &str) -> String {
    paint_bold_color(ColorToken::BrandAccent, value)
}

pub fn paint_bold(value: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", value)
}

pub fn paint_muted(value: &str) -> String {
    paint(ColorToken::TextMuted, value)
}

pub fn paint_ok(value: &str) -> String {
    paint_badge(ColorToken::Success, value)
}

pub fn paint_info(value: &str) -> String {
    paint_badge(ColorToken::Info, value)
}

pub fn paint_warn(value: &str) -> String {
    paint_badge(ColorToken::Warning, value)
}

pub fn paint_critical(value: &str) -> String {
    paint_badge(ColorToken::Error, value)
}

pub fn paint_prompt(value: &str) -> String {
    let c = theme().crossterm(ColorToken::BrandPrimary);
    let code = match c {
        crossterm::style::Color::Rgb { r, g, b } => format!("\x1b[1;38;2;{};{};{}m", r, g, b),
        _ => String::new(),
    };
    format!("{}➜ {}\x1b[0m", code, value)
}

// ─── Panels (kept box-drawing, theme-colored) ───────────────────────────────

pub fn paint_panel_top(width: usize) -> String {
    format!(
        "{}{}{}",
        paint_accent("╭"),
        paint_accent(&"─".repeat(width.saturating_sub(2))),
        paint_accent("╮")
    )
}

pub fn paint_panel_divider(width: usize) -> String {
    format!(
        "{}{}{}",
        paint_accent("├"),
        paint_accent(&"─".repeat(width.saturating_sub(2))),
        paint_accent("┤")
    )
}

pub fn paint_panel_bottom(width: usize) -> String {
    format!(
        "{}{}{}",
        paint_accent("╰"),
        paint_accent(&"─".repeat(width.saturating_sub(2))),
        paint_accent("╯")
    )
}

pub fn paint_panel_side() -> String {
    paint_accent("│")
}

// ─── Statement prompt ───────────────────────────────────────────────────────

/// A well-defined statement header, rendered through the RichPrompt engine.
pub fn statement_header(statement: &str) -> String {
    let ctx = PromptContext {
        workspace: workspace_name(),
        time_label: optidock_tui::prompt::short_time_label(),
        ..PromptContext::default()
    };
    RichPrompt::from_context(ctx, theme().clone()).statement_header(statement)
}

/// A compact statement line good for plain output.
pub fn statement_line(icon: &str, statement: &str) -> String {
    format!("{} {}", paint_accent(icon), paint_brand(statement))
}

// ─── Status bar ─────────────────────────────────────────────────────────────

/// Render a one-line status bar over `width` columns (for println! use).
pub fn status_bar(
    workspace: &str,
    mode: &str,
    provider: Option<&str>,
    model: Option<&str>,
    docker: Option<&str>,
    git_branch: Option<&str>,
    git_dirty: bool,
    key_hints: &[(&str, &str)],
    width: usize,
) -> String {
    StatusBar::from_context(
        workspace, mode, provider, model, docker, git_branch, git_dirty, key_hints,
    )
    .render_ansi(theme(), width)
}

/// Minimal right-aligned status bar.
pub fn status_bar_minimal(workspace: &str, mode: &str, width: usize) -> String {
    StatusBar::minimal(workspace, mode).render_ansi(theme(), width)
}

// ─── Banner / ASCII art ─────────────────────────────────────────────────────

/// The full brand banner (colored by the theme).
pub fn banner() -> Vec<String> {
    art::render_art("brand.logo", |i, line| {
        if i == 0 {
            paint_brand(line)
        } else {
            paint_accent(line)
        }
    })
    .map(|b| split_lines(&b))
    .unwrap_or_else(|| vec![paint_brand("  OPTIDOCK  AI")])
}

/// Split a multi-line painted string keeping the first line intact.
fn split_lines(s: &str) -> Vec<String> {
    s.split('\n').map(|l| l.to_string()).collect()
}

/// A decorative divider section header.
pub fn section_header(title: &str) -> String {
    format!("\n{} {}", paint_accent("●"), paint_bold(title))
}

/// Small helper: pull the leaf dir name of cwd for the prompt/status bar.
fn workspace_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "optidock".to_string())
}

/// The registered art library (for ad-hoc lookups).
pub fn art_library() -> &'static ArtLibrary {
    ArtLibrary::global()
}