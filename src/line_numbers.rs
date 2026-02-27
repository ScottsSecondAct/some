use std::collections::HashMap;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use crate::app::App;
use crate::buffer::GitChange;

pub fn render(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    line_indices: &[usize],
    git_changes: &HashMap<usize, GitChange>,
) {
    let width = app.gutter_width();
    let style = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();
    for &line_idx in line_indices {
        let num_str = format!("{:>width$}", line_idx + 1, width = width - 2);

        let (sep_char, sep_style) = match git_changes.get(&line_idx) {
            Some(GitChange::Added)    => ("\u{2502}", Style::default().fg(Color::Green)),
            Some(GitChange::Modified) => ("\u{2502}", Style::default().fg(Color::Yellow)),
            Some(GitChange::Deleted)  => ("\u{25be}", Style::default().fg(Color::Red)),
            None                      => ("\u{2502}", Style::default().fg(Color::Rgb(60, 60, 60))),
        };

        lines.push(Line::from(vec![
            Span::styled(num_str, style),
            Span::styled(format!(" {}", sep_char), sep_style),
        ]));
    }
    for _ in line_indices.len()..area.height as usize {
        let padding = " ".repeat(width.saturating_sub(2));
        lines.push(Line::from(vec![
            Span::styled(padding, style),
            Span::styled(" \u{2502}", Style::default().fg(Color::Rgb(60, 60, 60))),
        ]));
    }
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}
