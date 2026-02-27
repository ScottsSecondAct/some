use crate::app::{App, Mode};
use crate::keymap::Action;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

/// Process a single crossterm event and mutate app state accordingly.
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Mouse(mouse) => handle_mouse(app, mouse),
        Event::Resize(width, height) => {
            let tab_bar_height = if app.has_tab_bar() { 1 } else { 0 };
            app.content_width = width as usize;
            app.content_height = (height as usize).saturating_sub(2 + tab_bar_height);
        }
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match &app.mode {
        Mode::Normal => handle_normal_key(app, key),
        Mode::SearchInput { .. } => handle_search_key(app, key),
        Mode::CommandInput { .. } => handle_command_key(app, key),
        Mode::Follow => handle_follow_key(app, key),
        Mode::FilterInput { .. } => handle_filter_key(app, key),
        Mode::Visual { .. } => handle_visual_key(app, key),
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) {
    // Handle pending two-key sequences (marks)
    if let Some(pk) = app.pending_key.take() {
        if let KeyCode::Char(c) = key.code {
            match pk {
                'm' => {
                    app.marks.insert(c, app.top_line);
                    app.status_message = Some(format!("Mark '{}' set", c));
                }
                '\'' => {
                    if let Some(&line) = app.marks.get(&c) {
                        app.goto_line(line);
                        app.status_message = Some(format!("Jumped to mark '{}'", c));
                    } else {
                        app.status_message = Some(format!("No mark '{}'", c));
                    }
                }
                _ => {}
            }
        }
        return;
    }

    match app.key_map.get(&key) {
        Some(Action::Quit) => app.quit = true,

        Some(Action::ScrollDown) => app.scroll_down(1),
        Some(Action::ScrollUp)   => app.scroll_up(1),

        Some(Action::HalfPageDown) => {
            let half = app.content_height / 2;
            app.scroll_down(half);
        }
        Some(Action::HalfPageUp) => {
            let half = app.content_height / 2;
            app.scroll_up(half);
        }
        Some(Action::FullPageDown) => app.scroll_down(app.content_height),
        Some(Action::FullPageUp)   => app.scroll_up(app.content_height),

        Some(Action::GotoTop)    => app.goto_top(),
        Some(Action::GotoBottom) => app.goto_bottom(),

        Some(Action::PrevBuffer) => app.prev_buffer(),
        Some(Action::NextBuffer) => app.next_buffer(),

        Some(Action::SearchForward) => {
            app.mode = Mode::SearchInput { input: String::new(), forward: true };
        }
        Some(Action::SearchBackward) => {
            app.mode = Mode::SearchInput { input: String::new(), forward: false };
        }

        Some(Action::NextMatch) => {
            if app.search.has_pattern() {
                if app.search.forward { app.search.next_match(); } else { app.search.prev_match(); }
                if let Some(line) = app.search.current_match_line() {
                    app.goto_line(line);
                    app.status_message = Some(format!(
                        "Match {}/{}",
                        app.search.current + 1,
                        app.search.match_count()
                    ));
                }
            }
        }
        Some(Action::PrevMatch) => {
            if app.search.has_pattern() {
                if app.search.forward { app.search.prev_match(); } else { app.search.next_match(); }
                if let Some(line) = app.search.current_match_line() {
                    app.goto_line(line);
                    app.status_message = Some(format!(
                        "Match {}/{}",
                        app.search.current + 1,
                        app.search.match_count()
                    ));
                }
            }
        }

        Some(Action::ToggleNumbers) => app.show_line_numbers = !app.show_line_numbers,
        Some(Action::ToggleWrap)    => app.wrap_lines = !app.wrap_lines,

        Some(Action::FollowMode) => {
            app.mode = Mode::Follow;
            app.goto_bottom();
            app.status_message = Some("Follow mode \u{2014} press q or Esc to exit".to_string());
        }

        Some(Action::EnterCommand) => {
            app.mode = Mode::CommandInput { input: String::new() };
        }
        Some(Action::Filter) => {
            app.mode = Mode::FilterInput { input: String::new() };
        }
        Some(Action::Visual) => {
            app.mode = Mode::Visual { anchor: app.top_line, cursor: app.top_line };
        }

        Some(Action::SetMark) => {
            app.pending_key = Some('m');
            app.status_message = Some("m \u{2014} press a letter to set mark".to_string());
        }
        Some(Action::JumpMark) => {
            app.pending_key = Some('\'');
            app.status_message = Some("' \u{2014} press a letter to jump to mark".to_string());
        }

        Some(Action::ScrollRight) => app.left_col += 4,
        Some(Action::ScrollLeft)  => app.left_col = app.left_col.saturating_sub(4),

        None => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    let (input, forward) = match &app.mode {
        Mode::SearchInput { input, forward } => (input.clone(), *forward),
        _ => return,
    };

    match key.code {
        KeyCode::Enter => {
            app.search.forward = forward;
            app.search.query_string = input;
            app.mode = Mode::Normal;
            app.execute_search();
        }
        KeyCode::Esc => {
            app.search.preview_matches.clear();
            app.mode = Mode::Normal;
            app.status_message = None;
        }
        KeyCode::Backspace => {
            let mut new_input = input;
            new_input.pop();
            app.status_message = Some(format!(
                "{}{}",
                if forward { "/" } else { "?" },
                new_input
            ));
            app.mode = Mode::SearchInput {
                input: new_input.clone(),
                forward,
            };
            // Live incremental preview
            let smart_case = app.config.general.smart_case;
            if app.search.set_pattern(&new_input, smart_case).is_ok() {
                let start = app.top_line;
                let end = app.top_line + app.content_height;
                let buf = &app.buffers[app.active_buffer];
                app.search.search_visible_lines(buf, start, end);
            } else {
                app.search.preview_matches.clear();
            }
        }
        KeyCode::Char(c) => {
            let mut new_input = input;
            new_input.push(c);
            app.status_message = Some(format!(
                "{}{}",
                if forward { "/" } else { "?" },
                new_input
            ));
            app.mode = Mode::SearchInput {
                input: new_input.clone(),
                forward,
            };
            // Live incremental preview
            let smart_case = app.config.general.smart_case;
            if app.search.set_pattern(&new_input, smart_case).is_ok() {
                let start = app.top_line;
                let end = app.top_line + app.content_height;
                let buf = &app.buffers[app.active_buffer];
                app.search.search_visible_lines(buf, start, end);
            } else {
                app.search.preview_matches.clear();
            }
        }
        _ => {}
    }
}

fn handle_command_key(app: &mut App, key: KeyEvent) {
    let input = match &app.mode {
        Mode::CommandInput { input } => input.clone(),
        _ => return,
    };

    match key.code {
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            execute_command(app, &input);
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.status_message = None;
        }
        KeyCode::Backspace => {
            let mut new_input = input;
            new_input.pop();
            app.status_message = Some(format!(":{}", new_input));
            app.mode = Mode::CommandInput { input: new_input };
        }
        KeyCode::Char(c) => {
            let mut new_input = input;
            new_input.push(c);
            app.status_message = Some(format!(":{}", new_input));
            app.mode = Mode::CommandInput { input: new_input };
        }
        _ => {}
    }
}

fn handle_follow_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.status_message = None;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit = true;
        }
        _ => {}
    }
}

fn handle_filter_key(app: &mut App, key: KeyEvent) {
    let input = match &app.mode {
        Mode::FilterInput { input } => input.clone(),
        _ => return,
    };

    match key.code {
        KeyCode::Enter => {
            let query = input;
            app.mode = Mode::Normal;
            app.apply_filter(&query);
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.clear_filter();
            app.status_message = None;
        }
        KeyCode::Backspace => {
            let mut new_input = input;
            new_input.pop();
            app.status_message = Some(format!("&{}", new_input));
            app.mode = Mode::FilterInput { input: new_input };
        }
        KeyCode::Char(c) => {
            let mut new_input = input;
            new_input.push(c);
            app.status_message = Some(format!("&{}", new_input));
            app.mode = Mode::FilterInput { input: new_input };
        }
        _ => {}
    }
}

fn handle_visual_key(app: &mut App, key: KeyEvent) {
    let (anchor, cursor) = match &app.mode {
        Mode::Visual { anchor, cursor } => (*anchor, *cursor),
        _ => return,
    };
    let total = app.total_lines();

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            let new_cursor = (cursor + 1).min(total.saturating_sub(1));
            if new_cursor >= app.top_line + app.content_height {
                app.scroll_down(1);
            }
            app.mode = Mode::Visual { anchor, cursor: new_cursor };
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let new_cursor = cursor.saturating_sub(1);
            if new_cursor < app.top_line {
                app.scroll_up(1);
            }
            app.mode = Mode::Visual { anchor, cursor: new_cursor };
        }
        KeyCode::Char('y') => {
            app.yank_selection();
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollDown => app.scroll_down(3),
        MouseEventKind::ScrollUp => app.scroll_up(3),
        _ => {}
    }
}

fn execute_command(app: &mut App, cmd: &str) {
    match cmd.trim() {
        "q" | "quit" => app.quit = true,
        "n" | "next" => app.next_buffer(),
        "p" | "prev" => app.prev_buffer(),
        other => {
            if let Ok(line) = other.parse::<usize>() {
                app.goto_line(line.saturating_sub(1));
            } else {
                app.status_message = Some(format!("Unknown command: {}", other));
            }
        }
    }
}
