use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, Mode};
use crate::line_numbers;
use crate::statusbar;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.content_height = (area.height as usize).saturating_sub(2);
    app.content_width = (area.width as usize).saturating_sub(app.gutter_width());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_content(frame, app, chunks[0]);
    statusbar::render(frame, app, chunks[1]);
    render_input_bar(frame, app, chunks[2]);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let gutter_width = app.gutter_width() as u16;

    let (gutter_area, content_area) = if gutter_width > 0 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(gutter_width), Constraint::Min(1)])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    let start = app.top_line;
    let end = (start + area.height as usize).min(app.total_lines());

    if let Some(gutter) = gutter_area {
        line_numbers::render(frame, app, gutter, start, end);
    }

    let buf = app.buffer();
    let mut lines: Vec<Line> = Vec::new();

    if app.highlighter.is_enabled() {
        let syntax = app.highlighter.detect_syntax(buf.path.as_deref());
        let mut hl = app.highlighter.create_highlight_lines(syntax);
        for i in start..end {
            let text = buf.get_line(i).unwrap_or("");
            let styled_spans = app.highlighter.highlight_line(text, &mut hl);
            let search_ranges = app.search.matches_on_line(i);
            if search_ranges.is_empty() {
                let ratatui_spans: Vec<Span> = styled_spans
                    .into_iter()
                    .map(|s| Span::styled(s.text, s.style))
                    .collect();
                lines.push(Line::from(ratatui_spans));
            } else {
                // Flatten styled spans to plain text, then overlay search highlights as owned spans
                let plain: String = styled_spans.into_iter().map(|s| s.text).collect();
                lines.push(Line::from(highlight_search_owned(plain, &search_ranges)));
            }
        }
    } else {
        for i in start..end {
            let text = buf.get_line(i).unwrap_or("");
            let search_ranges = app.search.matches_on_line(i);
            if search_ranges.is_empty() {
                lines.push(Line::from(text.to_string()));
            } else {
                let spans = highlight_search_in_line(text, &search_ranges);
                lines.push(Line::from(spans));
            }
        }
    }

    let visible_lines = end - start;
    for _ in visible_lines..area.height as usize {
        lines.push(Line::from(Span::styled("~", Style::default().fg(Color::DarkGray))));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, content_area);
}

fn render_input_bar(frame: &mut Frame, app: &App, area: Rect) {
    let content = match &app.mode {
        Mode::SearchInput { input, forward } => {
            let prefix = if *forward { "/" } else { "?" };
            format!("{}{}", prefix, input)
        }
        Mode::CommandInput { input } => format!(":{}", input),
        Mode::Follow => "Waiting for data... (press Esc or q to stop)".to_string(),
        Mode::Normal => app
            .status_message
            .clone()
            .unwrap_or_else(|| "Press q to quit, / to search, : for commands".to_string()),
    };

    let style = match &app.mode {
        Mode::SearchInput { .. } | Mode::CommandInput { .. } => {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        }
        _ => Style::default().fg(Color::DarkGray),
    };

    let paragraph = Paragraph::new(content).style(style);
    frame.render_widget(paragraph, area);
}

/// Overlay search highlights on an owned String, producing owned Spans.
fn highlight_search_owned(text: String, ranges: &[std::ops::Range<usize>]) -> Vec<Span<'static>> {
    if ranges.is_empty() {
        return vec![Span::raw(text)];
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut last_end = 0;
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    for range in ranges {
        let s = range.start.min(text.len());
        let e = range.end.min(text.len());
        if s > last_end {
            spans.push(Span::raw(text[last_end..s].to_string()));
        }
        if e > s {
            spans.push(Span::styled(text[s..e].to_string(), hl));
        }
        last_end = e;
    }
    if last_end < text.len() {
        spans.push(Span::raw(text[last_end..].to_string()));
    }
    spans
}

fn highlight_search_in_line<'a>(text: &'a str, ranges: &[std::ops::Range<usize>]) -> Vec<Span<'a>> {
    if ranges.is_empty() {
        return vec![Span::raw(text)];
    }
    let mut spans = Vec::new();
    let mut last_end = 0;
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    for range in ranges {
        let s = range.start.min(text.len());
        let e = range.end.min(text.len());
        if s > last_end {
            spans.push(Span::raw(&text[last_end..s]));
        }
        if e > s {
            spans.push(Span::styled(&text[s..e], hl));
        }
        last_end = e;
    }
    if last_end < text.len() {
        spans.push(Span::raw(&text[last_end..]));
    }
    spans
}
