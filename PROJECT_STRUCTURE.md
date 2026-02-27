# Project Structure

## Source Layout

```
src/
├── main.rs          Entry point: arg parsing, terminal setup, event loop
├── app.rs           Central application state
├── buffer.rs        File loading, line indexing, decompression, hex dump, git gutter, diff
├── viewer.rs        TUI rendering (text, hex, diff)
├── input.rs         Keyboard and mouse event handling
├── keymap.rs        Action enum, KeyMap, configurable key dispatch
├── search.rs        Regex search engine (sync + async)
├── syntax.rs        Syntax highlighting wrapper (bundled + user themes)
├── config.rs        Config file loading and CLI merging
├── statusbar.rs     Status bar rendering
├── line_numbers.rs  Line number gutter rendering (with git change indicators)
└── cli.rs           CLI argument definitions (clap)

assets/
└── themes/
    ├── Monokai.tmTheme
    ├── Dracula.tmTheme
    ├── Nord.tmTheme
    └── Catppuccin-Mocha.tmTheme
```

---

## Data Flow

```
main.rs
  │
  ├─ Config::load()             reads ~/.config/some/config.toml
  ├─ Config::merge_cli()        CLI flags override config values
  ├─ Buffer::from_diff()        (--diff mode) or
  │  Buffer::from_file()        (transparent decompression, mmap or heap) or
  │  Buffer::from_stdin()
  ├─ SyntaxHighlighter::new()   loads bundled + user .tmTheme files
  ├─ App::new()                 builds KeyMap, loads git changes for each buffer
  └─ App::start_watching()      spawns notify watcher for follow mode
       │
       └─ event_loop()
            │
            ├─ terminal.draw() ──► viewer::render(frame, app)
            │                         ├─ render_tab_bar()   (when >1 buffer)
            │                         ├─ render_content()
            │                         │    ├─ if binary  → hex dump rows
            │                         │    ├─ if is_diff → colorized +/-/@@ lines
            │                         │    └─ otherwise  → syntax + search overlay
            │                         │         ├─ line_numbers::render() with git changes
            │                         │         ├─ SyntaxHighlighter → StyledSpans
            │                         │         ├─ amber preview overlay (incremental)
            │                         │         └─ yellow match overlay (committed)
            │                         ├─ statusbar::render()
            │                         └─ render_input_bar()
            │
            ├─ watcher_rx.try_recv()     file-change events (follow mode)
            │    └─ reload_active_buffer() + goto_bottom()
            │
            ├─ app.drain_search_results()  async search batches
            │    └─ SearchBatch::Progress  → extend matches, update status
            │    └─ SearchBatch::Done      → finalize, jump to first match
            │
            └─ input::handle_event(app, event)
                  ├─ handle_normal_key()   dispatch via app.key_map (Action enum)
                  │    └─ pending_key      two-key sequences: m<c>, '<c>
                  ├─ handle_search_key()   character accumulation + live preview
                  ├─ handle_command_key()  Enter → execute_command()
                  ├─ handle_follow_key()   only q / Esc / Ctrl-C
                  ├─ handle_filter_key()   character accumulation → apply_filter()
                  └─ handle_visual_key()   j/k extend selection, y yanks
```

---

## Module Details

### `main.rs`
Entry point. Orchestrates startup: parses CLI args, loads config, opens buffers (including `--diff` mode), creates `App`, enters raw mode, runs the event loop, restores terminal on exit. The event loop drains the notify watcher channel, the async search channel (`app.drain_search_results()`), and then polls for input events with a 200 ms timeout.

### `app.rs` — `App`
The single source of truth for all runtime state:

```rust
pub struct App {
    pub buffers: Vec<Buffer>,
    pub active_buffer: usize,
    pub mode: Mode,
    pub top_line: usize,
    pub left_col: usize,
    pub content_height: usize,
    pub content_width: usize,
    pub search: SearchState,
    pub highlighter: SyntaxHighlighter,
    pub config: Config,
    pub show_line_numbers: bool,
    pub wrap_lines: bool,
    pub status_message: Option<String>,
    pub quit: bool,
    pub marks: HashMap<char, usize>,
    pub pending_key: Option<char>,
    pub filter: Option<(String, Vec<usize>)>,
    pub top_filter_idx: usize,
    pub watcher_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>>,
    pub key_map: KeyMap,
    // (watcher kept alive via private field)
}
```

`Mode` encodes the current interaction state — each variant carries its own data:

```rust
pub enum Mode {
    Normal,
    SearchInput  { input: String, forward: bool },
    CommandInput { input: String },
    Follow,
    FilterInput  { input: String },
    Visual       { anchor: usize, cursor: usize },
}
```

Key methods: `execute_search()` spawns an async search thread; `drain_search_results()` is called each tick to process `SearchBatch` messages; `reload_active_buffer()` re-decompresses if needed and refreshes git changes.

### `buffer.rs` — `Buffer`
File content + O(1) line access. Transparently decompresses `.gz`/`.zst`/`.bz2` before indexing — mmap is skipped for decompressed content. For uncompressed files, chooses mmap or heap based on size threshold. `reload()` re-decompresses if the original path has a compression extension.

Additional capabilities:
- `hex_line(n)` / `hex_line_count()` / `display_line_count()` — hex dump support for binary files
- `is_binary()` — checks the first 8 KB for null bytes
- `git_changes: HashMap<usize, GitChange>` — populated by `load_git_changes()`, which shells out `git diff HEAD --unified=0`
- `is_diff: bool` — marks synthetic diff buffers (created via `Buffer::from_diff()`)
- `text_snapshot()` — clones all lines to owned strings for the async search thread
- `from_diff(file_a, file_b)` — generates a unified diff via the `similar` crate

Two storage strategies:

| Condition | Strategy |
|-----------|----------|
| Compressed file | Decompress into `BufferSource::Memory(Vec<u8>)` |
| Uncompressed, size < `mmap_threshold` (10 MB) | `BufferSource::Memory(Vec<u8>)` |
| Uncompressed, size ≥ `mmap_threshold` | `BufferSource::Mmap(memmap2::Mmap)` |

### `viewer.rs`
Renders a frame using ratatui. Layout:

```
┌──────────────────────────────────────┐
│  tab bar  (when >1 buffer)           │
├──────────────────────────────────────┤
│  gutter │  content area              │  ← render_content()
│         │                            │
│       ~ │                            │  ← tilde for lines past EOF
├─────────┴────────────────────────────┤
│  status bar                          │  ← statusbar::render()
│  / search query or : command         │  ← render_input_bar()
└──────────────────────────────────────┘
```

`render_content()` branches on `buf.is_binary()` (hex rows), `buf.is_diff` (diff colorization), then falls through to the normal syntax+search path. The search overlay merges preview (amber) and committed (bright yellow) ranges via `merge_syntax_search_preview()`.

### `keymap.rs` — `KeyMap` / `Action`
Provides configurable key dispatch for normal mode. `Action` is an enum of all normal-mode actions. `KeyMap` holds two maps:

- **primary** — user-overridable bindings built from defaults + `[keys]` config overrides
- **secondary** — hardcoded aliases (arrow keys, PgUp/Dn, Home, End, Enter, Ctrl-C) that always work

`parse_key_spec()` parses strings like `"ctrl+d"`, `"space"`, `"G"` into `(KeyCode, KeyModifiers)`. `KeyMap::build(&config.keys)` is called once in `App::new()`.

### `input.rs`
Routes `crossterm::event::Event` to `App` mutations. Normal mode dispatch goes through `app.key_map.get(&key)` → `Action` match. Search key handler runs `search.search_visible_lines()` on each keystroke for live incremental preview; clears `preview_matches` on `Esc`.

### `search.rs` — `SearchState`
```rust
pub struct SearchState {
    pub pattern: Option<Regex>,
    pub query_string: String,
    pub matches: Vec<(usize, Range<usize>)>,   // committed full-file results
    pub current: usize,
    pub forward: bool,
    pub preview_matches: Vec<(usize, Range<usize>)>,  // live viewport preview
    pub is_searching: bool,
    pub search_rx: Option<mpsc::Receiver<SearchBatch>>,
}
```

`search_visible_lines(buf, start, end)` populates `preview_matches` for the current viewport. The async path: `execute_search()` in `App` clones a text snapshot, spawns a thread, and sends `SearchBatch::Progress` every 10,000 lines and `SearchBatch::Done` at the end.

### `syntax.rs` — `SyntaxHighlighter`
Wraps `syntect`. On construction, loads the default syntect theme set, then overlays four bundled themes (embedded via `include_bytes!`) and any user `.tmTheme` files from the themes directory (config `themes_dir` or `~/.config/some/themes/`). `detect_syntax()` strips compression extensions to find the inner syntax (e.g. `.rs.gz` → Rust).

### `config.rs` — `Config`
Three sections: `[general]`, `[colors]`, `[keys]`. `KeysConfig` has one `Option<String>` per bindable action; unset fields keep defaults. `themes_dir: Option<PathBuf>` in `GeneralConfig` overrides the default user theme directory.

### `statusbar.rs` / `line_numbers.rs`
`statusbar::render()` shows filename, buffer position, mode badge, `[HEX]` indicator for binary files, filter indicator, search info with `[searching…]` during async search, line range, and scroll percentage. `line_numbers::render()` accepts a `&HashMap<usize, GitChange>` and colorizes the `│` separator: green (added), yellow (modified), red `▾` (deleted).

---

## Key Invariants

- `app.top_line` is always ≤ `app.max_top_line()` (clamped by all scroll methods)
- `app.active_buffer` is always a valid index into `app.buffers`
- `content_height` and `content_width` are updated at the start of every `viewer::render()` call
- `SearchState.matches` is always in line-ascending order (populated by iterating lines 0..N)
- Binary files (`is_binary()`) never enter the syntax highlight path; `display_line_count()` returns hex row count for them
- Diff buffers (`is_diff`) skip syntax highlighting and git gutter loading
