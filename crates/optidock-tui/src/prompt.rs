//! Rich prompt system — context-aware, themed, multi-line, git/dirty-aware.

use crate::theme::{ColorToken, Theme};

/// Where to place a prompt segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptSegmentKind {
    /// Left side before input
    Info,
    /// Right side (alignment)
    Status,
    /// Decorative separator
    Separator,
}

/// A single piece of the prompt line.
#[derive(Debug, Clone)]
pub struct PromptSegment {
    pub text: String,
    pub kind: PromptSegmentKind,
    pub bg: Option<ColorToken>,
    pub fg: Option<ColorToken>,
    pub bold: bool,
}

impl PromptSegment {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: PromptSegmentKind::Info, bg: None, fg: None, bold: false }
    }

    pub fn kind(mut self, kind: PromptSegmentKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn bg(mut self, token: ColorToken) -> Self {
        self.bg = Some(token);
        self
    }

    pub fn fg(mut self, token: ColorToken) -> Self {
        self.fg = Some(token);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    fn render(&self, theme: &Theme) -> String {
        // Build ANSI escape
        let mut ansi = String::new();
        if let Some(bg) = self.bg {
            let c = theme.crossterm(bg);
            use crossterm::style::Color;
            match c {
                Color::Rgb { r, g, b } => ansi.push_str(&format!("\x1b[48;2;{};{};{}m", r, g, b)),
                _ => {},
            }
        }
        if let Some(fg) = self.fg {
            let c = theme.crossterm(fg);
            use crossterm::style::Color;
            match c {
                Color::Rgb { r, g, b } => ansi.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b)),
                _ => {},
            }
        }
        if self.bold {
            ansi.push_str("\x1b[1m");
        }

        if !ansi.is_empty() {
            format!("{}{}\x1b[0m", ansi, self.text)
        } else {
            self.text.clone()
        }
    }
}

/// Runtime context that feeds the prompt.
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// Current workspace directory name.
    pub workspace: String,
    /// Active auth session (email/workspace) if logged in.
    pub auth_email: Option<String>,
    pub auth_workspace: Option<String>,
    /// API connectivity status string.
    pub api_status: String,
    /// Git branch name (if any) and dirty flag.
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub git_ahead: usize,
    pub git_behind: usize,
    /// Last command result (for failure indicator).
    pub last_exit_code: Option<i32>,
    pub last_command: Option<String>,
    /// Current provider / model info.
    pub provider: Option<String>,
    pub model: Option<String>,
    /// Elapsed millis of last long command.
    pub last_duration_ms: Option<u64>,
    /// Docker daemon / container health.
    pub docker_status: Option<String>,
    /// Version string.
    pub version: String,
    /// Current time label (short).
    pub time_label: String,
}

/// The fully-assembled, renderable rich prompt.
#[derive(Debug, Clone)]
pub struct RichPrompt {
    segments: Vec<PromptSegment>,
    theme: Theme,
    context: PromptContext,
    multi_line: bool,
}

impl RichPrompt {
    /// Create from context and theme.
    pub fn new(context: PromptContext, theme: Theme) -> Self {
        Self { segments: Vec::new(), theme, context, multi_line: true }
    }

    /// Build the default multi-line prompt from context.
    pub fn from_context(context: PromptContext, theme: Theme) -> Self {
        let mut prompt = Self { segments: Vec::new(), theme: theme.clone(), context: context.clone(), multi_line: true };
        prompt.build_default();
        prompt
    }

    /// Build the default segment layout.
    fn build_default(&mut self) {
        // Row 1: context line — workspace, branch, provider, time, status indicators
        // Row 2: the actual input arrow

        // Workspace
        if !self.context.workspace.is_empty() {
            self.segments.push(PromptSegment::new(format!("  {} ", self.context.workspace))
                .fg(ColorToken::TextInverse).bg(ColorToken::BrandPrimary).bold());
        } else {
            self.segments.push(PromptSegment::new("  ●  ")
                .fg(ColorToken::BrandAccent).bg(ColorToken::BackgroundElevated));
        }

        // Separator arrow
        self.segments.push(PromptSegment::new(" ")
            .fg(ColorToken::BrandPrimary).bg(ColorToken::BackgroundSurface));

        // Auth
        if let Some(email) = &self.context.auth_email.clone() {
            let short = email.split('@').next().unwrap_or(email);
            let suffix = self.context.auth_workspace.clone().map(|w| format!("  {} ", w)).unwrap_or_default();
            self.segments.push(PromptSegment::new(format!(" {}@{}{suffix}", short, email.split('@').nth(1).unwrap_or(""), suffix = suffix))
                .fg(ColorToken::TextSecondary).bg(ColorToken::BackgroundSurface));
        } else {
            self.segments.push(PromptSegment::new("  ○ not logged in  ")
                .fg(ColorToken::TextMuted).bg(ColorToken::BackgroundSurface));
        }

        // Git branch
        if let Some(branch) = &self.context.git_branch.clone() {
            let dirty = if self.context.git_dirty { "*" } else { "" };
            let ahead_behind = if self.context.git_ahead > 0 || self.context.git_behind > 0 {
                format!(" ↑{}↓{}", self.context.git_ahead, self.context.git_behind)
            } else { String::new() };
            let color = if self.context.git_dirty { ColorToken::Warning } else { ColorToken::Success };
            self.segments.push(PromptSegment::new(format!("   {}{}{}{} ", branch, dirty, ahead_behind, ""))
                .fg(color).bg(ColorToken::BackgroundSurface));
        }

        // Provider / model
        if let Some(provider) = &self.context.provider.clone() {
            let model = self.context.model.clone().map(|m| format!(":{}", m)).unwrap_or_default();
            self.segments.push(PromptSegment::new(format!("  ◆ {}{} ", provider, model))
                .fg(ColorToken::BrandAccent).bg(ColorToken::BackgroundSurface));
        }

        // Docker status
        if let Some(status) = &self.context.docker_status.clone() {
            let (icon, color) = if status.to_lowercase().contains("running") || status.eq_ignore_ascii_case("healthy") {
                ("🐳", ColorToken::ContainerRunning)
            } else if status.to_lowercase().contains("error") {
                ("🐳", ColorToken::ContainerError)
            } else {
                ("🐋", ColorToken::ContainerStopped)
            };
            self.segments.push(PromptSegment::new(format!("  {} {} ", icon, status))
                .fg(color).bg(ColorToken::BackgroundSurface));
        }

        // Time
        if !self.context.time_label.is_empty() {
            self.segments.push(PromptSegment::new(format!("  {} ", self.context.time_label))
                .fg(ColorToken::TextMuted).bg(ColorToken::BackgroundSurface));
        }

        // Last result indicator (if failure)
        if let Some(code) = self.context.last_exit_code {
            if code != 0 {
                self.segments.push(PromptSegment::new(format!("  ✗ {} ", code))
                    .fg(ColorToken::TextInverse).bg(ColorToken::Error).bold());
            } else if let Some(dur) = self.context.last_duration_ms {
                if dur > 2_000 {
                    self.segments.push(PromptSegment::new(format!("  ✓ took {}ms ", dur))
                        .fg(ColorToken::Success).bg(ColorToken::BackgroundSurface));
                }
            }
        }
    }

    /// Append a custom segment.
    pub fn push(&mut self, seg: PromptSegment) {
        self.segments.push(seg);
    }

    /// Return the fully rendered ANSI string (multi-line prompt).
    pub fn render(&self) -> String {
        let mut out = String::new();

        // Top row: all Info segments
        let mut row1: String = self.segments.iter().map(|s| s.render(&self.theme)).collect();
        // Close background run
        row1.push_str("\x1b[0m");

        // Thin continuation divider (like Starship)
        let dim_rule = format!("\x1b[38;2;{};{};{}m─\x1b[0m",
            match self.theme.crossterm(ColorToken::BorderSubtle) { crossterm::style::Color::Rgb { r, .. } => r, _ => 48 },
            match self.theme.crossterm(ColorToken::BorderSubtle) { crossterm::style::Color::Rgb { g, .. } => g, _ => 54 },
            match self.theme.crossterm(ColorToken::BorderSubtle) { crossterm::style::Color::Rgb { b, .. } => b, _ => 61 },
        );

        // Second row: input arrow
        let arrow_fg = self.theme.crossterm(ColorToken::BrandPrimary);
        use crossterm::style::Color;
        let arrow = match arrow_fg {
            Color::Rgb { r, g, b } => format!("\x1b[1;38;2;{};{};{}m➜\x1b[0m ", r, g, b),
            _ => "➜ ".to_string(),
        };

        // Decide multi vs single line
        if self.multi_line {
            // Wrapped with top info + arrow on second line
            out.push_str(&row1);
            out.push('\n');
            out.push_str(&dim_rule);
            out.push('\n');
            out.push_str(&arrow);
        } else {
            out.push_str(&row1);
            out.push_str(&arrow);
        }

        out
    }

    /// Single-line variant (e.g. for compact terminals).
    pub fn single_line(mut self) -> Self {
        self.multi_line = false;
        self
    }

    /// Plain (no ANSI) fallback for narrow / non-TTY output.
    pub fn plain(&self) -> String {
        let mut out = String::new();
        for seg in &self.segments {
            out.push_str(&seg.text);
        }
        out.push_str(" > ");
        out
    }

    /// Build a "statement prompt" block — a well-defined preamble before each tool
    /// output section. Call this at the beginning of any render_* section.
    pub fn statement_header(&self, statement: &str) -> String {
        let accent = self.theme.crossterm(ColorToken::BrandPrimary);
        use crossterm::style::Color;
        let accent_code = match accent {
            Color::Rgb { r, g, b } => format!("\x1b[38;2;{};{};{}m", r, g, b),
            _ => String::new(),
        };
        let reset = "\x1b[0m";
        let bold = "\x1b[1m";
        let muted = {
            let c = self.theme.crossterm(ColorToken::TextMuted);
            match c {
                Color::Rgb { r, g, b } => format!("\x1b[38;2;{};{};{}m", r, g, b),
                _ => String::new(),
            }
        };
        let ts = if self.context.time_label.is_empty() { String::new() } else { format!(" {}{}", muted, self.context.time_label) };
        format!(
            "{}── {}{}{} ──{} {}{}",
            accent_code, bold, statement, reset, muted, ts, reset
        )
    }
}

/// Render git info from a path — returns (branch, dirty, ahead, behind).
pub fn git_info_for_path(path: &std::path::Path) -> (Option<String>, bool, usize, usize) {
    let output = match std::process::Command::new("git").arg("-C").arg(path).arg("rev-parse").arg("--abbrev-ref").arg("HEAD").output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => return (None, false, 0, 0),
    };
    let branch = if output == "HEAD" { None } else { Some(output) };
    let dirty = std::process::Command::new("git").arg("-C").arg(path).arg("status").arg("--porcelain").output()
        .map(|o| !o.stdout.is_empty()).unwrap_or(false);
    let ahead = std::process::Command::new("git").arg("-C").arg(path).arg("rev-list").arg("--count").arg("@{upstream}..HEAD").output()
        .ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().ok()).unwrap_or(0);
    let behind = std::process::Command::new("git").arg("-C").arg(path).arg("rev-list").arg("--count").arg("HEAD..@{upstream}").output()
        .ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().ok()).unwrap_or(0);
    (branch, dirty, ahead, behind)
}

/// Short time label (e.g. "18:42:05").
pub fn short_time_label() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn renders_without_panic() {
        let ctx = PromptContext { workspace: "opti-doc".to_string(), git_branch: Some("main".to_string()), git_dirty: true, ..Default::default() };
        let p = RichPrompt::from_context(ctx, Theme::dark());
        let out = p.render();
        assert!(out.contains("opti-doc"));
        assert!(out.contains("main"));
    }

    #[test]
    fn plain_fallback_contains_text() {
        let ctx = PromptContext { workspace: "w".to_string(), ..Default::default() };
        let p = RichPrompt::from_context(ctx, Theme::dark());
        assert!(p.plain().contains("w"));
    }

    #[test]
    fn single_line_mode() {
        let ctx = PromptContext { workspace: "w".to_string(), ..Default::default() };
        let p = RichPrompt::from_context(ctx, Theme::dark()).single_line();
        assert!(!p.render().contains('\n'));
    }

    #[test]
    fn statement_header_has_statement() {
        let ctx = PromptContext { workspace: "w".to_string(), time_label: "12:00:00".to_string(), ..Default::default() };
        let p = RichPrompt::from_context(ctx, Theme::dark());
        assert!(p.statement_header("Analysis").contains("Analysis"));
    }
}