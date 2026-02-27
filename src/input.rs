use crate::app::{App, Mode};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

/// Process a single crossterm event and mutate app state accordingly.
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Mouse(mouse) => handle_mouse(app, mouse),
        Event::Resize(width, height) => {
            app.content_width = width as usize;
            app.content_height = (height as usize).saturating_sub(2);
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
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => app.quit = true,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit = true,

        (KeyCode::Char('j'), KeyModifiers::NONE)
        | (KeyCode::Down, KeyModifiers::NONE)
        | (KeyCode::Enter, KeyModifiers::NONE) => app.scroll_down(1),

        (KeyCode::Char('k'), KeyModifiers::NONE)
        | (KeyCode::Up, KeyModifiers::NONE) => app.scroll_up(1),

        (KeyCode::Char('d'), KeyModifiers::CONTROL)
        | (KeyCode::Char('d'), KeyModifiers::NONE) => {
            let half = app.content_height / 2;
            app.scroll_down(half);
        }

        (KeyCode::Char('u'), KeyModifiers::CONTROL)
        | (KeyCode::Char('u'), KeyModifiers::NONE) => {
            let half = app.content_height / 2;
            app.scroll_up(half);
        }

        (KeyCode::Char(' '), KeyModifiers::NONE)
        | (KeyCode::PageDown, KeyModifiers::NONE) => {
            app.scroll_down(app.content_height);
        }

        (KeyCode::Char('b'), KeyModifiers::NONE)
        | (KeyCode::PageUp, KeyModifiers::NONE) => {
            app.scroll_up(app.content_height);
        }

        (KeyCode::Char('g'), KeyModifiers::NONE)
        | (KeyCode::Home, KeyModifiers::NONE) => app.goto_top(),

        (KeyCode::Char('G'), KeyModifiers::NONE | KeyModifiers::SHIFT)
        | (KeyCode::End, KeyModifiers::NONE) => app.goto_bottom(),

        (KeyCode::Char('/'), KeyModifiers::NONE) => {
            app.mode = Mode::SearchInput {
                input: String::new(),
                forward: true,
            };
        }

        (KeyCode::Char('?'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            app.mode = Mode::SearchInput {
                input: String::new(),
                forward: false,
            };
        }

        (KeyCode::Char('n'), KeyModifiers::NONE) => {
            if app.search.has_pattern() {
                app.search.next_match();
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

        (KeyCode::Char('N'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            if app.search.has_pattern() {
                app.search.prev_match();
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

        (KeyCode::Char('l'), KeyModifiers::NONE) => {
            app.show_line_numbers = !app.show_line_numbers;
        }

        (KeyCode::Char('w'), KeyModifiers::NONE) => {
            app.wrap_lines = !app.wrap_lines;
        }

        (KeyCode::Char('F'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            app.mode = Mode::Follow;
            app.goto_bottom();
            app.status_message = Some("Follow mode — press q or Esc to exit".to_string());
        }

        (KeyCode::Char(':'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            app.mode = Mode::CommandInput {
                input: String::new(),
            };
        }

        (KeyCode::Right, KeyModifiers::NONE) => {
            app.left_col += 4;
        }
        (KeyCode::Left, KeyModifiers::NONE) => {
            app.left_col = app.left_col.saturating_sub(4);
        }

        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    let (input, forward) = match &app.mode {
        Mode::SearchInput { input, forward } => (input.clone(), *forward),
        _ => return,
    };

    match key.code {
        KeyCode::Enter => {
            app.search.query_string = input;
            app.mode = Mode::Normal;
            app.execute_search();
        }
        KeyCode::Esc => {
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
                input: new_input,
                forward,
            };
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
                input: new_input,
                forward,
            };
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
