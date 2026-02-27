use crate::buffer::Buffer;
use crate::config::Config;
use crate::search::SearchState;
use crate::syntax::SyntaxHighlighter;

/// The current interaction mode.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Normal viewing mode
    Normal,
    /// User is typing a search query
    SearchInput {
        input: String,
        forward: bool,
    },
    /// User is typing a command (e.g. ":n", ":p", ":q")
    CommandInput {
        input: String,
    },
    /// Follow mode (tail -f)
    Follow,
}

/// Central application state.
pub struct App {
    /// All loaded file buffers
    pub buffers: Vec<Buffer>,
    /// Index of the currently active buffer
    pub active_buffer: usize,
    /// Current interaction mode
    pub mode: Mode,
    /// Viewport: first visible line (0-indexed)
    pub top_line: usize,
    /// Horizontal scroll offset (columns)
    pub left_col: usize,
    /// Terminal height available for content (excluding status bars)
    pub content_height: usize,
    /// Terminal width
    pub content_width: usize,
    /// Search state
    pub search: SearchState,
    /// Syntax highlighter
    pub highlighter: SyntaxHighlighter,
    /// App config
    pub config: Config,
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Whether to wrap long lines
    pub wrap_lines: bool,
    /// Status message (transient, shown in status bar)
    pub status_message: Option<String>,
    /// Should the app quit?
    pub quit: bool,
}

impl App {
    pub fn new(buffers: Vec<Buffer>, config: Config, highlighter: SyntaxHighlighter) -> Self {
        Self {
            buffers,
            active_buffer: 0,
            mode: Mode::Normal,
            top_line: 0,
            left_col: 0,
            content_height: 24,
            content_width: 80,
            search: SearchState::new(),
            highlighter,
            show_line_numbers: config.general.line_numbers,
            wrap_lines: config.general.wrap,
            config,
            status_message: None,
            quit: false,
        }
    }

    /// Get a reference to the active buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffers[self.active_buffer]
    }

    /// Total lines in the active buffer.
    pub fn total_lines(&self) -> usize {
        self.buffer().line_count()
    }

    /// The maximum value for top_line (so the last line is visible).
    pub fn max_top_line(&self) -> usize {
        self.total_lines().saturating_sub(self.content_height)
    }

    /// Scroll down by N lines, clamped.
    pub fn scroll_down(&mut self, n: usize) {
        self.top_line = std::cmp::min(self.top_line + n, self.max_top_line());
    }

    /// Scroll up by N lines, clamped.
    pub fn scroll_up(&mut self, n: usize) {
        self.top_line = self.top_line.saturating_sub(n);
    }

    /// Jump to a specific line, centering it in the viewport.
    pub fn goto_line(&mut self, line: usize) {
        let target = line.saturating_sub(self.content_height / 2);
        self.top_line = std::cmp::min(target, self.max_top_line());
    }

    /// Go to the top of the file.
    pub fn goto_top(&mut self) {
        self.top_line = 0;
    }

    /// Go to the bottom of the file.
    pub fn goto_bottom(&mut self) {
        self.top_line = self.max_top_line();
    }

    /// Switch to the next buffer (wraps around).
    pub fn next_buffer(&mut self) {
        if self.buffers.len() > 1 {
            self.active_buffer = (self.active_buffer + 1) % self.buffers.len();
            self.top_line = 0;
            self.left_col = 0;
            self.status_message = Some(format!(
                "Buffer {}/{}: {}",
                self.active_buffer + 1,
                self.buffers.len(),
                self.buffer().name
            ));
        }
    }

    /// Switch to the previous buffer (wraps around).
    pub fn prev_buffer(&mut self) {
        if self.buffers.len() > 1 {
            self.active_buffer = if self.active_buffer == 0 {
                self.buffers.len() - 1
            } else {
                self.active_buffer - 1
            };
            self.top_line = 0;
            self.left_col = 0;
            self.status_message = Some(format!(
                "Buffer {}/{}: {}",
                self.active_buffer + 1,
                self.buffers.len(),
                self.buffer().name
            ));
        }
    }

    /// Percentage through the file based on top_line.
    pub fn scroll_percentage(&self) -> u16 {
        if self.total_lines() == 0 {
            return 100;
        }
        let bottom = self.top_line + self.content_height;
        let effective = std::cmp::min(bottom, self.total_lines());
        ((effective as f64 / self.total_lines() as f64) * 100.0) as u16
    }

    /// Width of the line number gutter (digits + 1 space).
    pub fn gutter_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        let max_line = self.total_lines();
        let digits = if max_line == 0 {
            1
        } else {
            (max_line as f64).log10() as usize + 1
        };
        digits + 2 // e.g. " 42 " — padding on each side
    }

    /// Execute a search with the current query.
    pub fn execute_search(&mut self) {
        let smart_case = self.config.general.smart_case;
        let query = self.search.query_string.clone();
        if self.search.set_pattern(&query, smart_case).is_ok() {
            let buffer = &self.buffers[self.active_buffer];
            self.search.search_buffer(buffer);
            if self.search.match_count() > 0 {
                self.search.jump_to_line(self.top_line);
                if let Some(line) = self.search.current_match_line() {
                    self.goto_line(line);
                }
                self.status_message = Some(format!(
                    "/{} ({} matches)",
                    self.search.query_string,
                    self.search.match_count()
                ));
            } else {
                self.status_message = Some(format!("Pattern not found: {}", query));
            }
        } else {
            self.status_message = Some(format!("Invalid regex: {}", query));
        }
    }
}
