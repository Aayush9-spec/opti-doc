//! Bottom status bar — persistent context, key hints, mode indicator.

use crate::theme::{ColorToken, Theme};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Alignment of a status item within the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAlignment {
    Left,
    Center,
    Right,
}

/// A single item in the status bar.
#[derive(Debug, Clone)]
pub struct StatusItem {
    text: String,
    alignment: StatusAlignment,
    fg: ColorToken,
    bg: Option<ColorToken>,
    bold: bool,
    separator: bool, // show separator after this item
}

impl StatusItem {
    pub fn new(text: impl Into<String>, alignment: StatusAlignment) -> Self {
        Self { text: text.into(), alignment, fg: ColorToken::TextSecondary, bg: None, bold: false, separator: true }
    }

    pub fn with_fg(mut self, fg: ColorToken) -> Self {
        self.fg = fg;
        self
    }

    pub fn with_bg(mut self, bg: ColorToken) -> Self {
        self.bg = Some(bg);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn no_separator(mut self) -> Self {
        self.separator = false;
        self
    }

    fn render(&self, theme: &Theme) -> Span<'static> {
        let mut style = theme.style(self.fg);
        if self.bold { style = style.add_modifier(Modifier::BOLD); }
        if let Some(bg) = self.bg { style = style.bg(theme.ratatui(bg)); }
        let mut text = self.text.clone();
        if self.separator { text.push_str(" │ "); }
        Span::styled(text, style)
    }
}

/// The status bar widget — composes items and renders at bottom of screen.
#[derive(Debug, Clone)]
pub struct StatusBar {
    items: Vec<StatusItem>,
    height: u16,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { items: Vec::new(), height: 1 }
    }

    pub fn add(mut self, item: StatusItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn height(mut self, h: u16) -> Self {
        self.height = h;
        self
    }

    /// Build default OptiDock status bar from context.
    pub fn from_context(
        workspace: &str,
        mode: &str,
        provider: Option<&str>,
        model: Option<&str>,
        docker: Option<&str>,
        git_branch: Option<&str>,
        git_dirty: bool,
        key_hints: &[(&str, &str)],
    ) -> Self {
        let mut bar = Self::new();

        // Left: workspace + mode
        bar = bar.add(StatusItem::new(format!("  {} ", workspace), StatusAlignment::Left)
            .with_fg(ColorToken::TextInverse).with_bg(ColorToken::BrandPrimary).bold());

        bar = bar.add(StatusItem::new(format!(" {} ", mode), StatusAlignment::Left)
            .with_fg(ColorToken::TextPrimary).with_bg(ColorToken::BackgroundSurface).bold());

        // Git branch
        if let Some(branch) = git_branch {
            let dirty_mark = if git_dirty { " *" } else { "" };
            let color = if git_dirty { ColorToken::Warning } else { ColorToken::Success };
            bar = bar.add(StatusItem::new(format!("  {}{} ", branch, dirty_mark), StatusAlignment::Left)
                .with_fg(color).with_bg(ColorToken::BackgroundSurface));
        }

        // Center: provider/model (if space permits)
        if let Some(p) = provider {
            let m = model.map(|m| format!(":{}", m)).unwrap_or_default();
            bar = bar.add(StatusItem::new(format!(" ◆ {}{} ", p, m), StatusAlignment::Center)
                .with_fg(ColorToken::BrandAccent).with_bg(ColorToken::BackgroundSurface));
        }

        // Docker
        if let Some(d) = docker {
            bar = bar.add(StatusItem::new(format!(" 🐳 {} ", d), StatusAlignment::Right)
                .with_fg(ColorToken::ContainerRunning).with_bg(ColorToken::BackgroundSurface));
        }

        // Key hints (right side)
        for (key, desc) in key_hints {
            bar = bar.add(StatusItem::new(format!(" {} {} ", key, desc), StatusAlignment::Right)
                .with_fg(ColorToken::TextMuted).with_bg(ColorToken::BackgroundSurface).no_separator());
        }

        // Trailing space
        bar = bar.add(StatusItem::new(" ", StatusAlignment::Right).no_separator());

        bar
    }

    /// Minimal bar for single-line prompts.
    pub fn minimal(workspace: &str, mode: &str) -> Self {
        Self::new()
            .add(StatusItem::new(format!(" {} ", workspace), StatusAlignment::Left)
                .with_fg(ColorToken::TextInverse).with_bg(ColorToken::BrandPrimary).bold())
            .add(StatusItem::new(format!(" {} ", mode), StatusAlignment::Left)
                .with_fg(ColorToken::TextPrimary).with_bg(ColorToken::BackgroundSurface))
            .add(StatusItem::new("  Ready  ", StatusAlignment::Right)
                .with_fg(ColorToken::Success).with_bg(ColorToken::BackgroundSurface))
    }

    /// Render into frame at bottom of given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let bar_area = Rect { x: area.x, y: area.y + area.height.saturating_sub(self.height), width: area.width, height: self.height };
        if bar_area.height == 0 || bar_area.width == 0 { return; }

        // Split into left/center/right groups
        let left_items: Vec<Span> = self.items.iter().filter(|i| i.alignment == StatusAlignment::Left).map(|i| i.render(theme)).collect();
        let center_items: Vec<Span> = self.items.iter().filter(|i| i.alignment == StatusAlignment::Center).map(|i| i.render(theme)).collect();
        let right_items: Vec<Span> = self.items.iter().filter(|i| i.alignment == StatusAlignment::Right).map(|i| i.render(theme)).collect();

        let left = Line::from(left_items);
        let center = Line::from(center_items);
        let right = Line::from(right_items);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(bar_area);

        let left_para = Paragraph::new(left).style(theme.style(ColorToken::BackgroundSurface)).alignment(Alignment::Left);
        let center_para = Paragraph::new(center).style(theme.style(ColorToken::BackgroundSurface)).alignment(Alignment::Center);
        let right_para = Paragraph::new(right).style(theme.style(ColorToken::BackgroundSurface)).alignment(Alignment::Right);

        frame.render_widget(left_para, chunks[0]);
        frame.render_widget(center_para, chunks[1]);
        frame.render_widget(right_para, chunks[2]);
    }

    /// Render as raw ANSI string for println! use (fallback).
    pub fn render_ansi(&self, theme: &Theme, width: usize) -> String {
        let left: String = self.items.iter().filter(|i| i.alignment == StatusAlignment::Left).map(|i| i.render(theme).content).collect();
        let center: String = self.items.iter().filter(|i| i.alignment == StatusAlignment::Center).map(|i| i.render(theme).content).collect();
        let right: String = self.items.iter().filter(|i| i.alignment == StatusAlignment::Right).map(|i| i.render(theme).content).collect();

        let used = left.chars().count() + center.chars().count() + right.chars().count();
        let padding = width.saturating_sub(used).max(0);
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;

        format!(
            "{}{}{}{}{}\x1b[0m",
            left,
            " ".repeat(left_pad),
            center,
            " ".repeat(right_pad),
            right
        )
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Mode indicator enum for consistent terminology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Command,
    Live,
    Debug,
    Help,
}

impl AppMode {
    pub fn label(&self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Command => "COMMAND",
            AppMode::Live => "LIVE",
            AppMode::Debug => "DEBUG",
            AppMode::Help => "HELP",
        }
    }

    pub fn color(&self) -> ColorToken {
        match self {
            AppMode::Normal => ColorToken::Info,
            AppMode::Command => ColorToken::Warning,
            AppMode::Live => ColorToken::ContainerRunning,
            AppMode::Debug => ColorToken::BrandAccent,
            AppMode::Help => ColorToken::BrandSecondary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bar_default() {
        let bar = StatusBar::default();
        assert_eq!(bar.height, 1);
        assert!(bar.items.is_empty());
    }

    #[test]
    fn from_context_builds_items() {
        let bar = StatusBar::from_context("opti-doc", "LIVE", Some("openai"), Some("gpt-4"), Some("running"), Some("main"), false, &[("?", "help")]);
        assert!(!bar.items.is_empty());
    }

    #[test]
    fn minimal_bar() {
        let bar = StatusBar::minimal("test", "NORMAL");
        assert_eq!(bar.items.len(), 3);
    }

    #[test]
    fn mode_labels() {
        assert_eq!(AppMode::Normal.label(), "NORMAL");
        assert_eq!(AppMode::Live.label(), "LIVE");
        assert_eq!(AppMode::Debug.label(), "DEBUG");
    }
}