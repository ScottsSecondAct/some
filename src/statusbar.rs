use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use crate::app::{App, Mode};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let buf = app.buffer();
    let total = app.total_lines();
    let top = app.top_line + 1;
    let bottom = (app.top_line + app.content_height).min(total);
    let pct = app.scroll_percentage();

    let buffer_indicator = if app.buffers.len() > 1 {
        format!(" [{}/{}]", app.active_buffer + 1, app.buffers.len())
    } else {
        String::new()
    };

    let mode_indicator = match &app.mode {
        Mode::Normal => "",
        Mode::SearchInput { .. } => " [SEARCH]",
        Mode::CommandInput { .. } => " [COMMAND]",
        Mode::Follow => " [FOLLOW]",
        Mode::FilterInput { .. } => " [FILTER]",
        Mode::Visual { .. } => " [VISUAL]",
    };

    let hex_indicator = if buf.is_binary() { " [HEX]" } else { "" };

    let filter_indicator = if let Some((ref q, ref idx)) = app.filter {
        format!(" [~{} {}L]", q, idx.len())
    } else {
        String::new()
    };

    let left = format!(" {}{}{}{}{} ", buf.name, buffer_indicator, mode_indicator, hex_indicator, filter_indicator);

    let searching_indicator = if app.search.is_searching { " [searching…]" } else { "" };

    let search_info = if app.search.has_pattern() {
        format!(
            " {}{} ({} matches){} \u{2502}",
            if app.search.forward { "/" } else { "?" },
            app.search.query_string,
            app.search.match_count(),
            searching_indicator,
        )
    } else {
        String::new()
    };

    let right = format!("{}  {}-{}/{} \u{2502} {}% ", search_info, top, bottom, total, pct);

    let available = area.width as usize;
    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let padding = if available > left_len + right_len {
        " ".repeat(available - left_len - right_len)
    } else {
        String::new()
    };

    let status = format!("{}{}{}", left, padding, right);
    let style = Style::default().fg(Color::Rgb(192, 197, 206)).bg(Color::Rgb(43, 48, 59));
    let paragraph = Paragraph::new(status).style(style);
    frame.render_widget(paragraph, area);
}
