use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, Mode};
use crate::line_numbers;
use crate::statusbar;
use crate::syntax::StyledSpan;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let tab_bar_height: u16 = if app.has_tab_bar() { 1 } else { 0 };
    app.content_height = (area.height as usize).saturating_sub(2 + tab_bar_height as usize);
    app.content_width = (area.width as usize).saturating_sub(app.gutter_width());

    if app.has_tab_bar() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),   // tab bar
                Constraint::Min(1),      // content
                Constraint::Length(1),   // status bar
                Constraint::Length(1),   // input/hint bar
            ])
            .split(area);
        render_tab_bar(frame, app, chunks[0]);
        render_content(frame, app, chunks[1]);
        statusbar::render(frame, app, chunks[2]);
        render_input_bar(frame, app, chunks[3]);
    } else {
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
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let max_name_len = 20usize;
    let mut spans: Vec<Span> = Vec::new();

    for (i, buf) in app.buffers.iter().enumerate() {
        let name = if buf.name.len() > max_name_len {
            format!("\u{2026}{}", &buf.name[buf.name.len() - (max_name_len - 1)..])
        } else {
            buf.name.clone()
        };
        let text = format!(" {} ", name);
        if i == app.active_buffer {
            spans.push(Span::styled(
                text,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                text,
                Style::default().fg(Color::DarkGray),
            ));
        }
        if i + 1 < app.buffers.len() {
            spans.push(Span::styled(
                "\u{2502}",
                Style::default().fg(Color::Rgb(60, 60, 60)),
            ));
        }
    }

    let bg = Style::default().bg(Color::Rgb(30, 34, 42));
    let paragraph = Paragraph::new(Line::from(spans)).style(bg);
    frame.render_widget(paragraph, area);
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

    let line_indices = app.active_lines();

    if let Some(gutter) = gutter_area {
        line_numbers::render(frame, app, gutter, &line_indices, &app.buffer().git_changes);
    }

    let search_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let preview_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Rgb(200, 160, 60));
    let visual_style = Style::default()
        .fg(Color::White)
        .bg(Color::Blue);

    let buf = app.buffer();
    let mut lines: Vec<Line> = Vec::new();

    // Binary files: render hex dump
    if buf.is_binary() {
        let hex_style = Style::default().fg(Color::Rgb(150, 200, 150));
        for &i in &line_indices {
            lines.push(Line::from(Span::styled(buf.hex_line(i), hex_style)));
        }
    } else if buf.is_diff {
        // Diff buffers: colorize by line prefix
        for &i in &line_indices {
            let text = buf.get_line(i).unwrap_or("");
            let style = match text.chars().next() {
                Some('+') => Style::default().fg(Color::Rgb(100, 220, 100)),
                Some('-') => Style::default().fg(Color::Rgb(220, 80, 80)),
                Some('@') => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                _         => Style::default().fg(Color::Rgb(170, 170, 170)),
            };
            lines.push(Line::from(Span::styled(text.to_string(), style)));
        }
    } else {
        // Normal text rendering
        let visual_range: Option<(usize, usize)> = match &app.mode {
            Mode::Visual { anchor, cursor } => {
                let lo = (*anchor).min(*cursor);
                let hi = (*anchor).max(*cursor);
                Some((lo, hi))
            }
            _ => None,
        };

        if app.highlighter.is_enabled() {
            let syntax = app.highlighter.detect_syntax(buf.path.as_deref());
            let mut hl = app.highlighter.create_highlight_lines(syntax);
            for &i in &line_indices {
                let text = buf.get_line(i).unwrap_or("");
                let is_selected = visual_range.map(|(lo, hi)| i >= lo && i <= hi).unwrap_or(false);
                if is_selected {
                    lines.push(Line::from(Span::styled(text.to_string(), visual_style)));
                } else {
                    let styled_spans = app.highlighter.highlight_line(text, &mut hl);
                    let search_ranges = app.search.matches_on_line(i);
                    let preview_ranges = app.search.preview_matches_on_line(i);
                    let spans = merge_syntax_search_preview(
                        styled_spans, &preview_ranges, preview_style,
                        &search_ranges, search_style,
                    );
                    lines.push(Line::from(spans));
                }
            }
        } else {
            for &i in &line_indices {
                let text = buf.get_line(i).unwrap_or("");
                let is_selected = visual_range.map(|(lo, hi)| i >= lo && i <= hi).unwrap_or(false);
                if is_selected {
                    lines.push(Line::from(Span::styled(text.to_string(), visual_style)));
                } else {
                    let search_ranges = app.search.matches_on_line(i);
                    let preview_ranges = app.search.preview_matches_on_line(i);
                    let plain_span = vec![StyledSpan {
                        text: text.to_string(),
                        style: Style::default(),
                    }];
                    let spans = merge_syntax_search_preview(
                        plain_span, &preview_ranges, preview_style,
                        &search_ranges, search_style,
                    );
                    lines.push(Line::from(spans));
                }
            }
        }
    }

    let visible_lines = line_indices.len();
    for _ in visible_lines..area.height as usize {
        lines.push(Line::from(Span::styled(
            "~",
            Style::default().fg(Color::DarkGray),
        )));
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
        Mode::FilterInput { input } => format!("&{}", input),
        Mode::Follow => "Waiting for data... (press Esc or q to stop)".to_string(),
        Mode::Visual { anchor, cursor } => {
            let lo = anchor.min(cursor);
            let hi = anchor.max(cursor);
            format!(
                "-- VISUAL -- lines {}-{} ({} selected)  y:yank  Esc:cancel",
                lo + 1,
                hi + 1,
                hi - lo + 1
            )
        }
        Mode::Normal => app
            .status_message
            .clone()
            .unwrap_or_else(|| "q:quit  /:search  ?:back-search  &:filter  v:visual  F:follow  ::cmd".to_string()),
    };

    let style = match &app.mode {
        Mode::SearchInput { .. } | Mode::CommandInput { .. } | Mode::FilterInput { .. } => {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        }
        Mode::Visual { .. } => Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 80)),
        _ => Style::default().fg(Color::DarkGray),
    };

    let paragraph = Paragraph::new(content).style(style);
    frame.render_widget(paragraph, area);
}

/// Merge syntax spans with preview (amber) and committed (bright yellow) search highlights.
/// Preview ranges are overlaid first; committed matches overwrite on the same byte positions.
fn merge_syntax_search_preview(
    syntax_spans: Vec<StyledSpan>,
    preview_ranges: &[std::ops::Range<usize>],
    preview_style: Style,
    search_ranges: &[std::ops::Range<usize>],
    search_style: Style,
) -> Vec<Span<'static>> {
    // Build a combined set of highlights: preview first, search second (wins on overlap)
    // We'll process them as two ordered passes merged into a unified overlay.
    if preview_ranges.is_empty() && search_ranges.is_empty() {
        return syntax_spans
            .into_iter()
            .map(|s| Span::styled(s.text, s.style))
            .collect();
    }

    // Build a vec of (start, end, style) sorted by start
    let mut highlights: Vec<(usize, usize, Style)> = Vec::new();
    for r in preview_ranges {
        highlights.push((r.start, r.end, preview_style));
    }
    for r in search_ranges {
        highlights.push((r.start, r.end, search_style));
    }
    highlights.sort_by_key(|h| h.0);

    let mut result: Vec<Span<'static>> = Vec::new();
    let mut byte_pos: usize = 0;

    for span in &syntax_spans {
        let span_start = byte_pos;
        let span_end = byte_pos + span.text.len();
        let mut local_pos: usize = 0;

        for &(hl_start, hl_end, hl_style) in &highlights {
            if hl_end <= span_start || hl_start >= span_end {
                continue;
            }
            let hl_local_start = hl_start.saturating_sub(span_start);
            let hl_local_end = (hl_end - span_start).min(span.text.len());
            let actual_start = hl_local_start.max(local_pos);

            if hl_local_start > local_pos {
                let s = &span.text[local_pos..hl_local_start];
                if !s.is_empty() {
                    result.push(Span::styled(s.to_string(), span.style));
                }
            }
            if hl_local_end > actual_start {
                let s = &span.text[actual_start..hl_local_end];
                if !s.is_empty() {
                    result.push(Span::styled(s.to_string(), hl_style));
                }
            }
            if hl_local_end > local_pos {
                local_pos = hl_local_end;
            }
        }

        if local_pos < span.text.len() {
            let s = &span.text[local_pos..];
            result.push(Span::styled(s.to_string(), span.style));
        }

        byte_pos = span_end;
    }

    result
}
