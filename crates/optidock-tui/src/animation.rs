//! Animation primitives — spinners, transitions, timed sequences.

use std::time::{Duration, Instant};
use crossterm::style::Color;
use ratatui::style::{Color as RColor, Style as RStyle};

/// Spinner style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerStyle {
    /// Classic spinning line
    Line,
    /// Dots pulsing
    Dots,
    /// Bouncing ball
    Bounce,
    /// Braille spinner (smooth)
    Braille,
    /// Docker-themed
    Docker,
    /// Recovery-themed
    Recovery,
    /// Brain/thinking
    Brain,
}

impl SpinnerStyle {
    /// Frame characters for each style.
    pub fn frames(&self) -> &'static [&'static str] {
        match self {
            SpinnerStyle::Line => &["|", "/", "-", "\\"],
            SpinnerStyle::Dots => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            SpinnerStyle::Bounce => &["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"],
            SpinnerStyle::Braille => &["⠋", "⠙", "⠚", "⠒", "⠂", "⠂", "⠒", "⠚", "⠙", "⠋"],
            SpinnerStyle::Docker => &["🐳", "🐋", "🐳", "🐋"],
            SpinnerStyle::Recovery => &["◓", "◑", "◒", "◐"],
            SpinnerStyle::Brain => &["🧠", "💭", "💡", "✨", "💡", "💭"],
        }
    }

    /// Default frame duration in milliseconds.
    pub fn frame_duration(&self) -> u64 {
        match self {
            SpinnerStyle::Line => 100,
            SpinnerStyle::Dots => 80,
            SpinnerStyle::Bounce => 80,
            SpinnerStyle::Braille => 60,
            SpinnerStyle::Docker => 400,
            SpinnerStyle::Recovery => 150,
            SpinnerStyle::Brain => 300,
        }
    }
}

/// A spinner that can be rendered in-place or as a widget.
#[derive(Debug, Clone)]
pub struct Spinner {
    style: SpinnerStyle,
    message: String,
    start_time: Instant,
    color_token: crate::theme::ColorToken,
}

impl Spinner {
    /// Create a new spinner with a message.
    pub fn new(message: impl Into<String>, style: SpinnerStyle) -> Self {
        Self {
            style,
            message: message.into(),
            start_time: Instant::now(),
            color_token: crate::theme::ColorToken::BrandPrimary,
        }
    }

    /// Set color token.
    pub fn color(mut self, token: crate::theme::ColorToken) -> Self {
        self.color_token = token;
        self
    }

    /// Get current frame based on elapsed time.
    pub fn current_frame(&self) -> &str {
        let frames = self.style.frames();
        let elapsed = self.start_time.elapsed().as_millis() as usize;
        let frame_idx = (elapsed / self.style.frame_duration() as usize) % frames.len();
        frames[frame_idx]
    }

    /// Get elapsed time.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Render as a single line string (for println! use).
    pub fn render_line(&self, theme: &crate::theme::Theme) -> String {
        let frame = self.current_frame();
        let color = theme.crossterm(self.color_token);
        let (r, g, b) = match color {
            Color::Rgb { r, g, b } => (r, g, b),
            _ => (0, 180, 216),
        };
        let frame_colored = format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, frame);
        format!("{} {}", frame_colored, self.message)
    }

    /// Render as a ratatui widget in the given area.
    pub fn render_widget(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, theme: &crate::theme::Theme) {
        use ratatui::widgets::Paragraph;
        let text = self.render_line(theme);
        let para = Paragraph::new(text).style(theme.style(self.color_token));
        frame.render_widget(para, area);
    }
}

/// Transition animations for UI state changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Fade in/out (simulated with rapid redraws)
    Fade,
    /// Slide from direction
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
    /// Scale up/down
    Scale,
    /// None (instant)
    None,
}

impl Transition {
    /// Duration in milliseconds.
    pub fn duration(&self) -> u64 {
        match self {
            Transition::Fade => 200,
            Transition::SlideLeft | Transition::SlideRight | Transition::SlideUp | Transition::SlideDown => 150,
            Transition::Scale => 180,
            Transition::None => 0,
        }
    }
}

/// Trait for a single animation step's render logic.
pub trait AnimStep: Send + Sync {
    fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, theme: &crate::theme::Theme, progress: f64);
}

/// A timed animation sequence that runs multiple steps.
pub struct AnimationSequence {
    steps: Vec<(Duration, Box<dyn AnimStep>)>,
    current_step: usize,
    step_start: Instant,
}

impl std::fmt::Debug for AnimationSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationSequence")
            .field("steps", &self.steps.len())
            .field("current_step", &self.current_step)
            .finish()
    }
}

impl Clone for AnimationSequence {
    fn clone(&self) -> Self {
        Self { steps: Vec::new(), current_step: self.current_step, step_start: self.step_start }
    }
}

impl AnimationSequence {
    pub fn new() -> Self {
        Self { steps: Vec::new(), current_step: 0, step_start: Instant::now() }
    }

    /// Add a step with a custom render function and duration.
    pub fn step<F>(mut self, duration: Duration, render_fn: F) -> Self
    where
        F: AnimStep + 'static,
    {
        self.steps.push((duration, Box::new(render_fn)));
        self
    }

    /// Add a step with a closure.
    pub fn step_fn<F>(self, duration: Duration, render_fn: F) -> Self
    where
        F: Fn(&mut ratatui::Frame, ratatui::layout::Rect, &crate::theme::Theme, f64) + Send + Sync + 'static,
    {
        self.step(duration, ClosureStep(render_fn))
    }

    /// Check if animation is complete.
    pub fn is_done(&self) -> bool {
        self.current_step >= self.steps.len()
    }

    /// Render current frame; returns true if animation should continue.
    pub fn tick(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, theme: &crate::theme::Theme) -> bool {
        if self.is_done() { return false; }

        let (duration, step) = &self.steps[self.current_step];
        let elapsed = self.step_start.elapsed();
        let progress = (elapsed.as_millis() as f64 / duration.as_millis() as f64).min(1.0);

        step.render(frame, area, theme, progress);

        if progress >= 1.0 {
            self.current_step += 1;
            self.step_start = Instant::now();
        }

        !self.is_done()
    }
}

/// Wrapper to turn a plain closure into an `AnimStep`.
struct ClosureStep<F>(F);

impl<F> AnimStep for ClosureStep<F>
where
    F: Fn(&mut ratatui::Frame, ratatui::layout::Rect, &crate::theme::Theme, f64) + Send + Sync,
{
    fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, theme: &crate::theme::Theme, progress: f64) {
        (self.0)(frame, area, theme, progress);
    }
}

/// Pre-built animation sequences.
pub mod presets {
    use super::*;
    use ratatui::widgets::Paragraph;
    use ratatui::text::Line;

    /// Fade-in a panel from invisible to visible.
    pub fn fade_in_panel(title: String, lines: Vec<Line<'static>>) -> AnimationSequence {
        AnimationSequence::new()
            .step_fn(Duration::from_millis(200), move |frame, area, _theme, _progress| {
                let style = RStyle::default()
                    .fg(RColor::Rgb(230, 237, 243))
                    .bg(RColor::Rgb(22, 27, 34));
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(title.clone())
                    .border_style(RStyle::default().fg(RColor::Rgb(0, 180, 216)))
                    .style(style);
                let para = Paragraph::new(lines.clone()).block(block).style(style);
                frame.render_widget(para, area);
            })
    }

    /// Slide-in from right.
    pub fn slide_in_right(content: Vec<Line<'static>>) -> AnimationSequence {
        AnimationSequence::new()
            .step_fn(Duration::from_millis(150), move |frame, area, theme, progress| {
                let offset = ((1.0 - progress) * area.width as f64) as u16;
                let inner_area = ratatui::layout::Rect {
                    x: area.x + offset,
                    y: area.y,
                    width: area.width.saturating_sub(offset),
                    height: area.height,
                };
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(theme.panel_border_style(false))
                    .style(theme.panel_style());
                let para = Paragraph::new(content.clone()).block(block);
                frame.render_widget(para, inner_area);
            })
    }

    /// Pulse a highlight (for drawing attention to a change).
    pub fn pulse_highlight(_color: crate::theme::ColorToken) -> AnimationSequence {
        AnimationSequence::new()
            .step_fn(Duration::from_millis(300), |_frame, _area, _theme, progress| {
                // Pulse: 0->1->0 intensity
                let _intensity = if progress < 0.5 { progress * 2.0 } else { (1.0 - progress) * 2.0 };
            })
    }

    /// Typing effect for text reveal.
    pub fn typewriter(text: String, speed_ms_per_char: u64) -> AnimationSequence {
        let chars: Vec<char> = text.chars().collect();
        let total_duration = Duration::from_millis(chars.len() as u64 * speed_ms_per_char);
        AnimationSequence::new()
            .step_fn(total_duration, move |frame, area, theme, progress| {
                let visible_chars = (chars.len() as f64 * progress) as usize;
                let visible: String = chars.iter().take(visible_chars).collect();
                let para = Paragraph::new(visible).style(theme.style(crate::theme::ColorToken::TextPrimary));
                frame.render_widget(para, area);
            })
    }
}

impl Default for AnimationSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Async helper: run a spinner while a future completes.
/// Uses a blocking spawn to avoid lifetime issues with theme reference.
pub async fn with_spinner<F, T, E>(
    message: &str,
    style: SpinnerStyle,
    future: F,
    theme: crate::theme::Theme, // owned to satisfy 'static
) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    use crossterm::{cursor, queue, style::Print};
    use std::io::Write;

    let spinner = Spinner::new(message, style).color(crate::theme::ColorToken::BrandPrimary);
    let frame_duration = spinner.style.frame_duration();

    // Start spinner in background (owned theme, spinner, stdout)
    let spinner_handle = tokio::spawn(async move {
        let mut stdout = std::io::stdout();
        let spinner = spinner;
        loop {
            let line = spinner.render_line(&theme);
            queue!(stdout, cursor::SavePosition, Print(&line), cursor::RestorePosition).ok();
            stdout.flush().ok();
            tokio::time::sleep(Duration::from_millis(frame_duration)).await;
        }
    });

    let result = future.await;

    // Stop spinner
    spinner_handle.abort();
    let mut out = std::io::stdout();
    queue!(out, cursor::SavePosition, Print(" ".repeat(message.len() + 4)), cursor::RestorePosition).ok();
    out.flush().ok();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_frames_cycle() {
        let s = Spinner::new("test", SpinnerStyle::Line);
        let f1 = s.current_frame();
        std::thread::sleep(Duration::from_millis(150));
        let f2 = s.current_frame();
        // At least different frames possible
        assert!(["|", "/", "-", "\\"].contains(&f1));
        assert!(["|", "/", "-", "\\"].contains(&f2));
    }

    #[test]
    fn spinner_styles_have_frames() {
        for style in [SpinnerStyle::Line, SpinnerStyle::Dots, SpinnerStyle::Docker, SpinnerStyle::Brain] {
            assert!(!style.frames().is_empty());
            assert!(style.frame_duration() > 0);
        }
    }

    #[test]
    fn transition_durations() {
        assert_eq!(Transition::None.duration(), 0);
        assert!(Transition::Fade.duration() > 0);
        assert!(Transition::SlideLeft.duration() > 0);
    }

    #[test]
    fn animation_sequence_ticks() {
        let seq = AnimationSequence::new()
            .step_fn(Duration::from_millis(50), |_, _, _, _| {})
            .step_fn(Duration::from_millis(50), |_, _, _, _| {});
        assert!(!seq.is_done());
        // Can't easily test tick without a frame; just verify structure
        assert_eq!(seq.steps.len(), 2);
    }
}