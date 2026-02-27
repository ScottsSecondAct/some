use std::collections::HashMap;

use crate::buffer::Buffer;
use crate::config::Config;
use crate::keymap::KeyMap;
use crate::search::{SearchBatch, SearchState};
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
    /// User is typing a filter pattern
    FilterInput {
        input: String,
    },
    /// Visual line-selection mode
    Visual {
        anchor: usize,
        cursor: usize,
    },
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
    /// Named marks: char → top_line at time of mark
    pub marks: HashMap<char, usize>,
    /// Pending first key of a two-key sequence (e.g. 'm', '\'')
    pub pending_key: Option<char>,
    /// Active line filter: (query_string, matching line indices)
    pub filter: Option<(String, Vec<usize>)>,
    /// Scroll position within filtered lines
    pub top_filter_idx: usize,
    /// File-change event receiver (for follow mode)
    pub watcher_rx: Option<std::sync::mpsc::Receiver<notify::Result<notify::Event>>>,
    /// File watcher (kept alive as long as App is alive)
    watcher: Option<notify::RecommendedWatcher>,
    /// Key → Action dispatch table
    pub key_map: KeyMap,
}

impl App {
    pub fn new(mut buffers: Vec<Buffer>, config: Config, highlighter: SyntaxHighlighter) -> Self {
        // Load git change indicators for all file-backed buffers
        for buf in &mut buffers {
            if buf.path.is_some() && !buf.is_diff {
                buf.load_git_changes();
            }
        }
        let key_map = KeyMap::build(&config.keys);
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
            marks: HashMap::new(),
            pending_key: None,
            filter: None,
            top_filter_idx: 0,
            watcher_rx: None,
            watcher: None,
            key_map,
        }
    }

    /// Get a reference to the active buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffers[self.active_buffer]
    }

    /// Total display lines in the active buffer (hex rows for binary, text lines otherwise).
    pub fn total_lines(&self) -> usize {
        self.buffer().display_line_count()
    }

    /// The maximum value for top_line (so the last line is visible).
    pub fn max_top_line(&self) -> usize {
        self.total_lines().saturating_sub(self.content_height)
    }

    /// Returns true when a tab bar should be shown.
    pub fn has_tab_bar(&self) -> bool {
        self.buffers.len() > 1
    }

    /// The ordered list of line indices to display in the viewport.
    pub fn active_lines(&self) -> Vec<usize> {
        if let Some((_, ref indices)) = self.filter {
            let start = self.top_filter_idx;
            let end = (start + self.content_height).min(indices.len());
            if start >= indices.len() {
                vec![]
            } else {
                indices[start..end].to_vec()
            }
        } else {
            let start = self.top_line;
            let end = (start + self.content_height).min(self.total_lines());
            (start..end).collect()
        }
    }

    /// Scroll down by N lines, clamped. Operates on the filtered list when active.
    pub fn scroll_down(&mut self, n: usize) {
        if let Some((_, ref indices)) = self.filter {
            let max = indices.len().saturating_sub(self.content_height);
            self.top_filter_idx = (self.top_filter_idx + n).min(max);
        } else {
            self.top_line = std::cmp::min(self.top_line + n, self.max_top_line());
        }
    }

    /// Scroll up by N lines, clamped. Operates on the filtered list when active.
    pub fn scroll_up(&mut self, n: usize) {
        if self.filter.is_some() {
            self.top_filter_idx = self.top_filter_idx.saturating_sub(n);
        } else {
            self.top_line = self.top_line.saturating_sub(n);
        }
    }

    /// Jump to a specific line, centering it in the viewport.
    pub fn goto_line(&mut self, line: usize) {
        let target = line.saturating_sub(self.content_height / 2);
        self.top_line = std::cmp::min(target, self.max_top_line());
    }

    /// Go to the top of the file.
    pub fn goto_top(&mut self) {
        self.top_line = 0;
        self.top_filter_idx = 0;
    }

    /// Go to the bottom of the file.
    pub fn goto_bottom(&mut self) {
        self.top_line = self.max_top_line();
        if let Some((_, ref indices)) = self.filter {
            self.top_filter_idx = indices.len().saturating_sub(self.content_height);
        }
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
        if let Some((_, ref indices)) = self.filter {
            if indices.is_empty() {
                return 100;
            }
            let bottom = self.top_filter_idx + self.content_height;
            let effective = bottom.min(indices.len());
            ((effective as f64 / indices.len() as f64) * 100.0) as u16
        } else {
            if self.total_lines() == 0 {
                return 100;
            }
            let bottom = self.top_line + self.content_height;
            let effective = std::cmp::min(bottom, self.total_lines());
            ((effective as f64 / self.total_lines() as f64) * 100.0) as u16
        }
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

    /// Execute a search asynchronously, updating `search.matches` via a background thread.
    pub fn execute_search(&mut self) {
        let smart_case = self.config.general.smart_case;
        let query = self.search.query_string.clone();
        if self.search.set_pattern(&query, smart_case).is_err() {
            self.status_message = Some(format!("Invalid regex: {}", query));
            return;
        }
        let pattern = match self.search.pattern.clone() {
            Some(p) => p,
            None => {
                self.status_message = Some("Empty pattern".to_string());
                return;
            }
        };

        self.search.matches.clear();
        self.search.preview_matches.clear();
        self.search.is_searching = true;

        let snapshot = self.buffers[self.active_buffer].text_snapshot();
        let (tx, rx) = std::sync::mpsc::channel();
        self.search.search_rx = Some(rx);

        std::thread::spawn(move || {
            let mut batch = Vec::new();
            for (line_idx, text) in snapshot.iter().enumerate() {
                for mat in pattern.find_iter(text) {
                    batch.push((line_idx, mat.start()..mat.end()));
                }
                if line_idx % 10_000 == 9_999 {
                    let _ = tx.send(SearchBatch::Progress {
                        matches: std::mem::take(&mut batch),
                        lines_scanned: line_idx + 1,
                    });
                }
            }
            let _ = tx.send(SearchBatch::Done { matches: batch });
        });

        self.status_message = Some(format!("Searching /{} \u{2026}", self.search.query_string));
    }

    /// Apply a filter: keep only lines matching the regex.
    pub fn apply_filter(&mut self, query: &str) {
        if query.is_empty() {
            self.clear_filter();
            return;
        }
        match regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
        {
            Ok(re) => {
                let total = self.buffers[self.active_buffer].line_count();
                let indices: Vec<usize> = (0..total)
                    .filter(|&i| {
                        self.buffers[self.active_buffer]
                            .get_line(i)
                            .map(|l| re.is_match(l))
                            .unwrap_or(false)
                    })
                    .collect();
                let count = indices.len();
                self.filter = Some((query.to_string(), indices));
                self.top_filter_idx = 0;
                self.status_message = Some(format!("Filter: {} ({} lines)", query, count));
            }
            Err(e) => {
                self.status_message = Some(format!("Invalid filter regex: {}", e));
            }
        }
    }

    /// Clear the active filter.
    pub fn clear_filter(&mut self) {
        self.filter = None;
        self.top_filter_idx = 0;
    }

    /// Yank the visual selection to the clipboard and return to Normal mode.
    pub fn yank_selection(&mut self) {
        let (anchor, cursor) = match &self.mode {
            Mode::Visual { anchor, cursor } => (*anchor, *cursor),
            _ => return,
        };
        let start = anchor.min(cursor);
        let end = anchor.max(cursor);
        let buf = &self.buffers[self.active_buffer];
        let text: String = (start..=end)
            .filter_map(|i| buf.get_line(i))
            .collect::<Vec<_>>()
            .join("\n");
        let line_count = end - start + 1;
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(text) {
                Ok(_) => {
                    self.status_message = Some(format!("Yanked {} lines", line_count));
                }
                Err(e) => {
                    self.status_message = Some(format!("Clipboard error: {}", e));
                }
            },
            Err(e) => {
                self.status_message = Some(format!("Clipboard unavailable: {}", e));
            }
        }
        self.mode = Mode::Normal;
    }

    /// Start watching all buffer paths for changes (follow mode).
    pub fn start_watching(&mut self) {
        use notify::{RecursiveMode, Watcher};

        let paths: Vec<_> = self.buffers.iter().filter_map(|b| b.path.clone()).collect();
        if paths.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        match notify::RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        ) {
            Ok(mut watcher) => {
                for path in &paths {
                    let _ = watcher.watch(path.as_path(), RecursiveMode::NonRecursive);
                }
                self.watcher_rx = Some(rx);
                self.watcher = Some(watcher);
            }
            Err(e) => {
                eprintln!("Warning: failed to start file watcher: {}", e);
            }
        }
    }

    /// Drain pending async search result batches. Called each event loop tick.
    pub fn drain_search_results(&mut self) {
        while let Some(rx) = &self.search.search_rx {
            match rx.try_recv() {
                Ok(SearchBatch::Progress { matches, lines_scanned }) => {
                    self.search.matches.extend(matches);
                    self.status_message = Some(format!(
                        "Searching\u{2026} ({} matches, {}k lines)",
                        self.search.match_count(),
                        lines_scanned / 1000
                    ));
                }
                Ok(SearchBatch::Done { matches }) => {
                    self.search.matches.extend(matches);
                    self.search.is_searching = false;
                    self.search.search_rx = None;
                    self.search.jump_to_line(self.top_line);
                    if let Some(line) = self.search.current_match_line() {
                        self.goto_line(line);
                    }
                    if self.search.match_count() > 0 {
                        self.status_message = Some(format!(
                            "{}{} ({} matches)",
                            if self.search.forward { "/" } else { "?" },
                            self.search.query_string,
                            self.search.match_count()
                        ));
                    } else {
                        self.status_message = Some(format!(
                            "Pattern not found: {}", self.search.query_string
                        ));
                    }
                    break;
                }
                Err(_) => break,
            }
        }
    }

    /// Reload the active buffer from disk and refresh search results.
    pub fn reload_active_buffer(&mut self) {
        let mmap_threshold = self.config.general.mmap_threshold;
        if let Err(e) = self.buffers[self.active_buffer].reload(mmap_threshold) {
            self.status_message = Some(format!("Reload failed: {}", e));
            return;
        }
        if self.search.has_pattern() {
            let buffer = &self.buffers[self.active_buffer];
            self.search.search_buffer(buffer);
        }
        if self.mode == Mode::Follow {
            self.goto_bottom();
        }
        let buf = &mut self.buffers[self.active_buffer];
        if buf.path.is_some() && !buf.is_diff {
            buf.load_git_changes();
        }
    }
}
