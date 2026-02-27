use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect, start: usize, end: usize) {
    let width = app.gutter_width();
    let style = Style::default().fg(Color::DarkGray);
    let separator_style = Style::default().fg(Color::Rgb(60, 60, 60));

    let mut lines: Vec<Line> = Vec::new();
    for line_num in start..end {
        let num_str = format!("{:>width$}", line_num + 1, width = width - 2);
        lines.push(Line::from(vec![
            Span::styled(num_str, style),
            Span::styled(" \u{2502}", separator_style),
        ]));
    }
    for _ in (end - start)..area.height as usize {
        let padding = " ".repeat(width.saturating_sub(2));
        lines.push(Line::from(vec![
            Span::styled(padding, style),
            Span::styled(" \u{2502}", separator_style),
        ]));
    }
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}
