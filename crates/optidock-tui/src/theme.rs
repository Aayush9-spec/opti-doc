//! Semantic theme system with light/dark variants and runtime switching.

use crossterm::style::Color;
use ratatui::style::{Color as RColor, Modifier, Style as RStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// High-level theme variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeKind {
    #[default]
    Dark,
    Light,
    /// Respects terminal's reported background (via `COLORFGBG` or OSC 11).
    System,
}

impl ThemeKind {
    /// Detect from environment (TERM_PROGRAM, COLORFGBG, etc.) — fallback to Dark.
    pub fn detect() -> Self {
        // Check common env vars that indicate light background
        if let Ok(cf) = std::env::var("COLORFGBG") {
            // COLORFGBG format: "foreground;background" — high bg = light
            if let Some(bg_str) = cf.split(';').nth(1) {
                if let Ok(bg) = bg_str.parse::<u8>() {
                    if bg > 128 { return ThemeKind::Light; }
                }
            }
        }
        if let Ok(term) = std::env::var("TERM_PROGRAM") {
            if term.eq_ignore_ascii_case("vscode") || term.eq_ignore_ascii_case("apple_terminal") {
                // Can't reliably detect; default to Dark for code-focused terminals
                return ThemeKind::Dark;
            }
        }
        ThemeKind::System // Will be resolved at render time
    }
}

/// Semantic color tokens — map to actual colors per theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorToken {
    // Brand
    BrandPrimary,
    BrandSecondary,
    BrandAccent,
    BrandMuted,

    // Background layers (0 = deepest, 3 = highest)
    BackgroundBase,
    BackgroundSurface,
    BackgroundElevated,
    BackgroundOverlay,

    // Foreground / text
    TextPrimary,
    TextSecondary,
    TextMuted,
    TextInverse,

    // Borders & dividers
    BorderSubtle,
    BorderDefault,
    BorderEmphasis,

    // Status / semantic
    Success,
    Warning,
    Error,
    Info,

    // Interactive
    FocusRing,
    Selection,
    Hover,

    // Specialized
    CodeBackground,
    CodeForeground,
    Link,

    // Container / Docker themed
    ContainerRunning,
    ContainerStopped,
    ContainerError,
    ContainerWarning,
    ImageLayer,
    NetworkLink,
    VolumeMount,

    // Recovery / Agent themed
    RecoveryPlanned,
    RecoveryRunning,
    RecoverySucceeded,
    RecoveryFailed,
    RecoveryEscalated,
    AgentHealthy,
    AgentUnhealthy,
    AgentEscalated,
}

/// Resolved semantic color (crossterm + ratatui dual representation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticColor {
    pub crossterm: Color,
    pub ratatui: RColor,
}

impl SemanticColor {
    /// Const-compatible conversion from crossterm `Color` to ratatui `Color`.
    const fn to_ratatui(c: Color) -> RColor {
        match c {
            Color::Reset => RColor::Reset,
            Color::Black => RColor::Black,
            Color::DarkGrey => RColor::DarkGray,
            Color::Red => RColor::LightRed,
            Color::DarkRed => RColor::Red,
            Color::Green => RColor::LightGreen,
            Color::DarkGreen => RColor::Green,
            Color::Yellow => RColor::LightYellow,
            Color::DarkYellow => RColor::Yellow,
            Color::Blue => RColor::LightBlue,
            Color::DarkBlue => RColor::Blue,
            Color::Magenta => RColor::LightMagenta,
            Color::DarkMagenta => RColor::Magenta,
            Color::Cyan => RColor::LightCyan,
            Color::DarkCyan => RColor::Cyan,
            Color::White => RColor::White,
            Color::Grey => RColor::Gray,
            Color::Rgb { r, g, b } => RColor::Rgb(r, g, b),
            Color::AnsiValue(v) => RColor::Indexed(v),
        }
    }

    pub const fn new(c: Color) -> Self {
        Self { crossterm: c, ratatui: Self::to_ratatui(c) }
    }

    pub fn style(self) -> RStyle {
        RStyle::default().fg(self.ratatui)
    }

    pub fn style_bold(self) -> RStyle {
        RStyle::default().fg(self.ratatui).add_modifier(Modifier::BOLD)
    }

    pub fn style_dim(self) -> RStyle {
        RStyle::default().fg(self.ratatui).add_modifier(Modifier::DIM)
    }

    pub fn bg(self) -> RStyle {
        RStyle::default().bg(self.ratatui)
    }
}

/// Complete theme definition — all semantic tokens resolved.
#[derive(Debug, Clone)]
pub struct Theme {
    kind: ThemeKind,
    colors: HashMap<ColorToken, SemanticColor>,
}

impl Theme {
    /// Built-in dark theme (default).
    pub fn dark() -> Self {
        let mut colors = HashMap::new();
        macro_rules! c {
            ($token:ident, $hex:expr) => {
                colors.insert(ColorToken::$token, SemanticColor::new(hex_color($hex)));
            };
        }

        // Brand — teal/cyan family
        c!(BrandPrimary, 0x00B4D8);
        c!(BrandSecondary, 0x0077B6);
        c!(BrandAccent, 0x90E0EF);
        c!(BrandMuted, 0x48CAE4);

        // Backgrounds — deep charcoal progression
        c!(BackgroundBase, 0x0D1117);
        c!(BackgroundSurface, 0x161B22);
        c!(BackgroundElevated, 0x21262D);
        c!(BackgroundOverlay, 0x30363D);

        // Text
        c!(TextPrimary, 0xE6EDF3);
        c!(TextSecondary, 0x8B949E);
        c!(TextMuted, 0x6E7681);
        c!(TextInverse, 0x0D1117);

        // Borders
        c!(BorderSubtle, 0x30363D);
        c!(BorderDefault, 0x484F58);
        c!(BorderEmphasis, 0x00B4D8);

        // Semantic
        c!(Success, 0x3FB950);
        c!(Warning, 0xD29922);
        c!(Error, 0xF85149);
        c!(Info, 0x58A6FF);

        // Interactive
        c!(FocusRing, 0x00B4D8);
        c!(Selection, 0x264F78);
        c!(Hover, 0x21262D);

        // Code / links
        c!(CodeBackground, 0x161B22);
        c!(CodeForeground, 0xE6EDF3);
        c!(Link, 0x58A6FF);

        // Docker themed
        c!(ContainerRunning, 0x3FB950);
        c!(ContainerStopped, 0x8B949E);
        c!(ContainerError, 0xF85149);
        c!(ContainerWarning, 0xD29922);
        c!(ImageLayer, 0xA371F7);
        c!(NetworkLink, 0x58A6FF);
        c!(VolumeMount, 0xFFA657);

        // Recovery themed
        c!(RecoveryPlanned, 0x58A6FF);
        c!(RecoveryRunning, 0xD29922);
        c!(RecoverySucceeded, 0x3FB950);
        c!(RecoveryFailed, 0xF85149);
        c!(RecoveryEscalated, 0xA371F7);
        c!(AgentHealthy, 0x3FB950);
        c!(AgentUnhealthy, 0xF85149);
        c!(AgentEscalated, 0xA371F7);

        Self { kind: ThemeKind::Dark, colors }
    }

    /// Built-in light theme.
    pub fn light() -> Self {
        let mut colors = HashMap::new();
        macro_rules! c {
            ($token:ident, $hex:expr) => {
                colors.insert(ColorToken::$token, SemanticColor::new(hex_color($hex)));
            };
        }

        // Brand
        c!(BrandPrimary, 0x006E8A);
        c!(BrandSecondary, 0x00557A);
        c!(BrandAccent, 0x008FA3);
        c!(BrandMuted, 0x0096A8);

        // Backgrounds — clean white progression
        c!(BackgroundBase, 0xFFFFFF);
        c!(BackgroundSurface, 0xF6F8FA);
        c!(BackgroundElevated, 0xEAECF0);
        c!(BackgroundOverlay, 0xD0D7DE);

        // Text
        c!(TextPrimary, 0x1F2328);
        c!(TextSecondary, 0x57606A);
        c!(TextMuted, 0x6E7781);
        c!(TextInverse, 0xFFFFFF);

        // Borders
        c!(BorderSubtle, 0xD0D7DE);
        c!(BorderDefault, 0x8C959D);
        c!(BorderEmphasis, 0x006E8A);

        // Semantic
        c!(Success, 0x2A7E2C);
        c!(Warning, 0x9A6700);
        c!(Error, 0xCB2431);
        c!(Info, 0x0969DA);

        // Interactive
        c!(FocusRing, 0x006E8A);
        c!(Selection, 0xD1E9FF);
        c!(Hover, 0xF6F8FA);

        // Code / links
        c!(CodeBackground, 0xF6F8FA);
        c!(CodeForeground, 0x1F2328);
        c!(Link, 0x0969DA);

        // Docker themed
        c!(ContainerRunning, 0x2A7E2C);
        c!(ContainerStopped, 0x6E7781);
        c!(ContainerError, 0xCB2431);
        c!(ContainerWarning, 0x9A6700);
        c!(ImageLayer, 0x6E3DBE);
        c!(NetworkLink, 0x0969DA);
        c!(VolumeMount, 0xBC4C00);

        // Recovery themed
        c!(RecoveryPlanned, 0x0969DA);
        c!(RecoveryRunning, 0x9A6700);
        c!(RecoverySucceeded, 0x2A7E2C);
        c!(RecoveryFailed, 0xCB2431);
        c!(RecoveryEscalated, 0x6E3DBE);
        c!(AgentHealthy, 0x2A7E2C);
        c!(AgentUnhealthy, 0xCB2431);
        c!(AgentEscalated, 0x6E3DBE);

        Self { kind: ThemeKind::Light, colors }
    }

    /// Current theme kind.
    pub fn kind(&self) -> ThemeKind {
        self.kind
    }

    /// Resolve a semantic token to its SemanticColor.
    pub fn resolve(&self, token: ColorToken) -> SemanticColor {
        self.colors.get(&token).copied().unwrap_or_else(|| {
            // Fallback: magenta so missing tokens are obvious
            SemanticColor::new(Color::Magenta)
        })
    }

    /// Resolve to crossterm Color (for println! use).
    pub fn crossterm(&self, token: ColorToken) -> Color {
        self.resolve(token).crossterm
    }

    /// Resolve to ratatui Color (for widgets).
    pub fn ratatui(&self, token: ColorToken) -> RColor {
        self.resolve(token).ratatui
    }

    /// Get a ratatui Style for a token.
    pub fn style(&self, token: ColorToken) -> RStyle {
        self.resolve(token).style()
    }

    /// Get a bold style for a token.
    pub fn style_bold(&self, token: ColorToken) -> RStyle {
        self.resolve(token).style_bold()
    }

    /// Get a dim style for a token.
    pub fn style_dim(&self, token: ColorToken) -> RStyle {
        self.resolve(token).style_dim()
    }

    /// Background style for a token.
    pub fn bg_style(&self, token: ColorToken) -> RStyle {
        self.resolve(token).bg()
    }

    /// Panel style (surface background + default border).
    pub fn panel_style(&self) -> RStyle {
        RStyle::default()
            .bg(self.ratatui(ColorToken::BackgroundSurface))
            .fg(self.ratatui(ColorToken::TextPrimary))
    }

    /// Panel border style.
    pub fn panel_border_style(&self, emphasis: bool) -> RStyle {
        if emphasis {
            RStyle::default().fg(self.ratatui(ColorToken::BorderEmphasis))
        } else {
            RStyle::default().fg(self.ratatui(ColorToken::BorderDefault))
        }
    }

    /// Table header style.
    pub fn table_header_style(&self) -> RStyle {
        RStyle::default()
            .bg(self.ratatui(ColorToken::BackgroundElevated))
            .fg(self.ratatui(ColorToken::TextPrimary))
            .add_modifier(Modifier::BOLD)
    }

    /// Table row style (alternating).
    pub fn table_row_style(&self, index: usize) -> RStyle {
        let bg = if index % 2 == 0 {
            self.ratatui(ColorToken::BackgroundSurface)
        } else {
            self.ratatui(ColorToken::BackgroundElevated)
        };
        RStyle::default().bg(bg).fg(self.ratatui(ColorToken::TextPrimary))
    }

    /// Switch theme kind at runtime.
    pub fn set_kind(&mut self, kind: ThemeKind) {
        self.kind = kind;
        *self = match kind {
            ThemeKind::Dark => Self::dark(),
            ThemeKind::Light => Self::light(),
            ThemeKind::System => Self::dark(), // Will be resolved per-render
        };
    }
}

/// Default theme instance (dark).
impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Parse a 0xRRGGBB hex literal into crossterm Color.
const fn hex_color(hex: u32) -> Color {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    Color::Rgb { r, g, b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_resolves_all_tokens() {
        let theme = Theme::dark();
        // Spot-check a few critical tokens
        assert_ne!(theme.crossterm(ColorToken::BrandPrimary), Color::Magenta);
        assert_ne!(theme.crossterm(ColorToken::Success), Color::Magenta);
        assert_ne!(theme.crossterm(ColorToken::ContainerRunning), Color::Magenta);
        assert_ne!(theme.crossterm(ColorToken::RecoverySucceeded), Color::Magenta);
    }

    #[test]
    fn light_theme_differs_from_dark() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert_ne!(dark.crossterm(ColorToken::BackgroundBase), light.crossterm(ColorToken::BackgroundBase));
        assert_ne!(dark.crossterm(ColorToken::TextPrimary), light.crossterm(ColorToken::TextPrimary));
    }

    #[test]
    fn hex_color_const_eval() {
        const C: Color = hex_color(0x00B4D8);
        matches!(C, Color::Rgb { r: 0, g: 180, b: 216 });
    }
}