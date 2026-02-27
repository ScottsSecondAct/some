mod app;
mod buffer;
mod cli;
mod config;
mod input;
mod keymap;
mod line_numbers;
mod search;
mod statusbar;
mod syntax;
mod viewer;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

fn main() -> Result<()> {
    let cli_args = cli::Cli::parse();

    // Load config and merge CLI flags
    let mut config = config::Config::load()
        .context("Failed to load configuration")?;
    config.merge_cli(&cli_args);

    // Set up syntax highlighting
    let syntax_enabled = !cli_args.no_syntax && !cli_args.plain;
    let highlighter = syntax::SyntaxHighlighter::new(
        &config.general.theme,
        syntax_enabled,
        config.general.themes_dir.as_deref(),
    );

    // Load buffers
    let buffers = if let Some(ref diff_path) = cli_args.diff {
        // Diff mode: compare first positional file against --diff FILE2
        if cli_args.files.is_empty() {
            eprintln!("some: --diff requires a FILE argument");
            std::process::exit(1);
        }
        let diff_buf = buffer::Buffer::from_diff(&cli_args.files[0], diff_path)
            .with_context(|| format!("Failed to create diff: {} vs {}", cli_args.files[0].display(), diff_path.display()))?;
        vec![diff_buf]
    } else if cli_args.files.is_empty() {
        // Read from stdin
        if atty::is(atty::Stream::Stdin) {
            eprintln!("Usage: some [OPTIONS] [FILE]...");
            eprintln!("Try 'some --help' for more information.");
            std::process::exit(1);
        }
        vec![buffer::Buffer::from_stdin()?]
    } else {
        let mut bufs = Vec::new();
        for path in &cli_args.files {
            match buffer::Buffer::from_file(path, config.general.mmap_threshold) {
                Ok(buf) => bufs.push(buf),
                Err(e) => {
                    eprintln!("some: {}: {}", path.display(), e);
                }
            }
        }
        if bufs.is_empty() {
            eprintln!("some: no files could be opened");
            std::process::exit(1);
        }
        bufs
    };

    // Build the application state
    let mut app = app::App::new(buffers, config.clone(), highlighter);

    // Start watching files for follow mode
    app.start_watching();

    // Apply CLI-specific overrides
    if let Some(line) = cli_args.start_line {
        app.goto_line(line.saturating_sub(1));
    }
    if let Some(ref pattern) = cli_args.pattern {
        app.search.query_string = pattern.clone();
        app.execute_search();
    }
    if cli_args.follow {
        app.mode = app::Mode::Follow;
        app.goto_bottom();
    }

    // Enter TUI
    run_tui(&mut app)?;

    Ok(())
}

/// Set up the terminal, run the event loop, then restore the terminal.
fn run_tui(app: &mut app::App) -> Result<()> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, app);

    // Restore terminal regardless of result
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to leave alternate screen")?;
    terminal.show_cursor()?;

    result
}

/// Poll-based event loop: render → check file changes → wait for input → repeat.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    use std::time::Duration;

    loop {
        terminal.draw(|frame| {
            viewer::render(frame, app);
        })?;

        // Check for file-change events (non-blocking); reload in follow mode
        let mut got_change = false;
        if let Some(rx) = &app.watcher_rx {
            while let Ok(ev) = rx.try_recv() {
                if let Ok(ev) = ev {
                    use notify::EventKind;
                    if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        got_change = true;
                    }
                }
            }
        }
        if got_change && app.mode == app::Mode::Follow {
            app.reload_active_buffer();
        }

        // Drain async search result batches
        app.drain_search_results();

        // Poll for terminal events with a short timeout (keeps follow mode responsive)
        if event::poll(Duration::from_millis(200))? {
            let ev = event::read().context("Failed to read terminal event")?;
            input::handle_event(app, ev);
        }

        if app.quit {
            break;
        }
    }
    Ok(())
}
