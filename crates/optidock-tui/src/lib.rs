//! Terminal GUI framework for OptiDock — themes, art, prompts, layout, animation, status bar.

pub mod animation;
pub mod art;
pub mod layout;
pub mod prompt;
pub mod statusbar;
pub mod theme;

use crossterm::style::Color;

pub use animation::{Spinner, SpinnerStyle, Transition};
pub use art::{ArtLibrary, AsciiArt};
pub use layout::{Panel, TableWidget, ProgressBar, Sparkline, LayoutBuilder};
pub use prompt::{RichPrompt, PromptContext, PromptSegment};
pub use statusbar::{StatusBar, StatusItem, StatusAlignment};
pub use theme::{Theme, ThemeKind, ColorToken, SemanticColor};

/// Re-exports for convenience
pub use crossterm::style::{Color as CrosstermColor, Attribute, Attributes};
pub use ratatui::style::{Style as RatatuiStyle, Modifier};

/// Initialize the terminal for TUI mode (alternate screen, raw mode, mouse capture).
pub fn init_terminal() -> anyhow::Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>> {
    use crossterm::{execute, terminal::{enable_raw_mode, EnterAlternateScreen}, event::EnableMouseCapture};
    use std::io::stdout;
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    Ok(ratatui::Terminal::new(backend)?)
}

/// Restore terminal to normal state.
pub fn restore_terminal(terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    use crossterm::{execute, terminal::{disable_raw_mode, LeaveAlternateScreen}, event::DisableMouseCapture};
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Quick ANSI color helper — maps semantic token to crossterm Color for raw print! use.
pub fn ansi_color(token: ColorToken, theme: &Theme) -> Color {
    theme.resolve(token).crossterm
}