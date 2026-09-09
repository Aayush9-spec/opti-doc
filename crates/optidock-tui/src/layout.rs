//! Layout primitives — panels, tables, progress bars, sparklines, builder.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Gauge, Sparkline as RatiSparkline},
    Frame,
};

use crate::theme::{Theme, ColorToken};

/// A bordered panel with title, optional subtitle, and content lines.
#[derive(Debug, Clone)]
pub struct Panel {
    title: String,
    subtitle: Option<String>,
    lines: Vec<Line<'static>>,
    emphasis: bool,
    width: Option<u16>,
}

impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), subtitle: None, lines: Vec::new(), emphasis: false, width: None }
    }

    pub fn subtitle(mut self, sub: impl Into<String>) -> Self {
        self.subtitle = Some(sub.into());
        self
    }

    pub fn line(mut self, line: impl Into<Line<'static>>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn lines(mut self, lines: impl IntoIterator<Item = impl Into<Line<'static>>>) -> Self {
        self.lines.extend(lines.into_iter().map(Into::into));
        self
    }

    pub fn emphasis(mut self, emphasis: bool) -> Self {
        self.emphasis = emphasis;
        self
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    /// Render into a ratatui Frame at the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title_line = if let Some(sub) = &self.subtitle {
            Line::from(vec![
                Span::styled(&self.title, theme.style_bold(ColorToken::TextPrimary)),
                Span::styled("  ", theme.style(ColorToken::TextMuted)),
                Span::styled(sub, theme.style_dim(ColorToken::TextSecondary)),
            ])
        } else {
            Line::from(Span::styled(&self.title, theme.style_bold(ColorToken::TextPrimary)))
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title_line)
            .border_style(theme.panel_border_style(self.emphasis))
            .style(theme.panel_style());

        let paragraph = Paragraph::new(self.lines.clone())
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);
    }
}

/// Table widget with header, rows, and optional column constraints.
#[derive(Debug, Clone)]
pub struct TableWidget {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    column_widths: Option<Vec<Constraint>>,
    header_emphasis: bool,
}

impl TableWidget {
    pub fn new(headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { headers: headers.into_iter().map(Into::into).collect(), rows: Vec::new(), column_widths: None, header_emphasis: true }
    }

    pub fn add_row(mut self, row: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.rows.push(row.into_iter().map(Into::into).collect());
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<String>>>) -> Self {
        for row in rows {
            self = self.add_row(row);
        }
        self
    }

    pub fn column_widths(mut self, widths: impl IntoIterator<Item = Constraint>) -> Self {
        self.column_widths = Some(widths.into_iter().collect());
        self
    }

    pub fn header_emphasis(mut self, emphasis: bool) -> Self {
        self.header_emphasis = emphasis;
        self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let widths = self.column_widths.clone().unwrap_or_else(|| {
            vec![Constraint::Percentage(100 / self.headers.len().max(1) as u16); self.headers.len()]
        });

        let header = Row::new(
            self.headers.iter().map(|h| Cell::from(h.clone()).style(
                if self.header_emphasis { theme.table_header_style() } else { theme.style_bold(ColorToken::TextSecondary) }
            ))
        );

        let rows: Vec<Row> = self.rows.iter().enumerate().map(|(i, row)| {
            Row::new(row.iter().map(|c| Cell::from(c.clone()))).style(theme.table_row_style(i))
        }).collect();

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).border_style(theme.panel_border_style(false)).style(theme.panel_style()))
            .column_spacing(2);

        frame.render_widget(table, area);
    }
}

/// Progress bar (0.0 to 1.0) with label.
#[derive(Debug, Clone)]
pub struct ProgressBar {
    label: String,
    progress: f64, // 0.0..1.0
    show_percentage: bool,
    color_token: ColorToken,
}

impl ProgressBar {
    pub fn new(label: impl Into<String>, progress: f64) -> Self {
        Self { label: label.into(), progress: progress.clamp(0.0, 1.0), show_percentage: true, color_token: ColorToken::BrandPrimary }
    }

    pub fn color(mut self, token: ColorToken) -> Self {
        self.color_token = token;
        self
    }

    pub fn hide_percentage(mut self) -> Self {
        self.show_percentage = false;
        self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let percent = (self.progress * 100.0) as u16;
        let label = if self.show_percentage {
            format!("{} {}%", self.label, percent)
        } else {
            self.label.clone()
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).border_style(theme.panel_border_style(false)).style(theme.panel_style()))
            .gauge_style(theme.style(self.color_token))
            .label(label)
            .ratio(self.progress);

        frame.render_widget(gauge, area);
    }
}

/// Sparkline — single-line data visualization.
#[derive(Debug, Clone)]
pub struct Sparkline {
    label: String,
    data: Vec<u64>,
    color_token: ColorToken,
    max_value: Option<u64>,
}

impl Sparkline {
    pub fn new(label: impl Into<String>, data: impl IntoIterator<Item = u64>) -> Self {
        Self { label: label.into(), data: data.into_iter().collect(), color_token: ColorToken::BrandAccent, max_value: None }
    }

    pub fn color(mut self, token: ColorToken) -> Self {
        self.color_token = token;
        self
    }

    pub fn max(mut self, max: u64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let max = self.max_value.unwrap_or_else(|| *self.data.iter().max().unwrap_or(&1));
        let data: Vec<u64> = self.data.iter().map(|&v| v.min(max)).collect();

        let sparkline = RatiSparkline::default()
            .block(Block::default().borders(Borders::ALL).border_style(theme.panel_border_style(false)).style(theme.panel_style()))
            .style(theme.style(self.color_token))
            .data(&data);

        // Render label + sparkline in a combined layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(self.label.len() as u16 + 2), Constraint::Min(10)])
            .split(area);

        let label_para = Paragraph::new(self.label.as_str()).style(theme.style(ColorToken::TextSecondary));
        frame.render_widget(label_para, chunks[0]);
        frame.render_widget(sparkline, chunks[1]);
    }
}

/// LayoutBuilder — fluent DSL for composing complex layouts.
#[derive(Debug, Default)]
pub struct LayoutBuilder {
    direction: Direction,
    constraints: Vec<Constraint>,
    children: Vec<LayoutNode>,
}

#[derive(Debug)]
enum LayoutNode {
    Leaf(Box<dyn Renderable>),
    Branch(LayoutBuilder),
}

trait Renderable: std::fmt::Debug {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme);
}

impl LayoutBuilder {
    pub fn horizontal() -> Self {
        Self { direction: Direction::Horizontal, constraints: Vec::new(), children: Vec::new() }
    }

    pub fn vertical() -> Self {
        Self { direction: Direction::Vertical, constraints: Vec::new(), children: Vec::new() }
    }

    pub fn constraint(mut self, c: Constraint) -> Self {
        self.constraints.push(c);
        self
    }

    pub fn percentage(mut self, p: u16) -> Self {
        self.constraints.push(Constraint::Percentage(p));
        self
    }

    pub fn length(mut self, l: u16) -> Self {
        self.constraints.push(Constraint::Length(l));
        self
    }

    pub fn ratio(mut self, num: u32, den: u32) -> Self {
        self.constraints.push(Constraint::Ratio(num, den));
        self
    }

    pub fn min(mut self, m: u16) -> Self {
        self.constraints.push(Constraint::Min(m));
        self
    }

    pub fn max(mut self, m: u16) -> Self {
        self.constraints.push(Constraint::Max(m));
        self
    }

    /// Add a panel as a child.
    pub fn panel(mut self, panel: Panel) -> Self {
        self.children.push(LayoutNode::Leaf(Box::new(panel)));
        self
    }

    /// Add a table as a child.
    pub fn table(mut self, table: TableWidget) -> Self {
        self.children.push(LayoutNode::Leaf(Box::new(table)));
        self
    }

    /// Add a progress bar as a child.
    pub fn progress(mut self, progress: ProgressBar) -> Self {
        self.children.push(LayoutNode::Leaf(Box::new(progress)));
        self
    }

    /// Add a sparkline as a child.
    pub fn sparkline(mut self, spark: Sparkline) -> Self {
        self.children.push(LayoutNode::Leaf(Box::new(spark)));
        self
    }

    /// Add a nested layout.
    pub fn nest(mut self, builder: LayoutBuilder) -> Self {
        self.children.push(LayoutNode::Branch(builder));
        self
    }

    /// Build and render into a frame.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.constraints.is_empty() || self.children.is_empty() {
            return;
        }

        let layout = Layout::default()
            .direction(self.direction)
            .constraints(self.constraints.clone())
            .split(area);

        for (i, child) in self.children.iter().enumerate() {
            if i >= layout.len() { break; }
            match child {
                LayoutNode::Leaf(r) => r.render(frame, layout[i], theme),
                LayoutNode::Branch(b) => b.render(frame, layout[i], theme),
            }
        }
    }
}

/// Implement Renderable for our widgets
impl Renderable for Panel {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        Panel::render(self, frame, area, theme);
    }
}

impl Renderable for TableWidget {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        TableWidget::render(self, frame, area, theme);
    }
}

impl Renderable for ProgressBar {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        ProgressBar::render(self, frame, area, theme);
    }
}

impl Renderable for Sparkline {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        Sparkline::render(self, frame, area, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_creation() {
        let panel = Panel::new("Test").line("Hello").line("World");
        assert_eq!(panel.lines.len(), 2);
    }

    #[test]
    fn table_widget() {
        let table = TableWidget::new(["Col1", "Col2"]).add_row(["A", "B"]).add_row(["C", "D"]);
        assert_eq!(table.headers.len(), 2);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn progress_bar_clamp() {
        let p = ProgressBar::new("Test", 1.5);
        assert_eq!(p.progress, 1.0);
        let p = ProgressBar::new("Test", -0.5);
        assert_eq!(p.progress, 0.0);
    }

    #[test]
    fn layout_builder() {
        let builder = LayoutBuilder::vertical()
            .percentage(50)
            .percentage(50)
            .panel(Panel::new("Top").line("Content"))
            .panel(Panel::new("Bottom").line("More"));
        assert_eq!(builder.constraints.len(), 2);
        assert_eq!(builder.children.len(), 2);
    }
}