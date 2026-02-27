# Project Structure

## Source Layout

```
src/
├── main.rs          Entry point: arg parsing, terminal setup, event loop
├── app.rs           Central application state
├── buffer.rs        File loading and line indexing
├── viewer.rs        TUI rendering
├── input.rs         Keyboard and mouse event handling
├── search.rs        Regex search engine
├── syntax.rs        Syntax highlighting wrapper
├── config.rs        Config file loading and CLI merging
├── statusbar.rs     Status bar rendering
├── line_numbers.rs  Line number gutter rendering
└── cli.rs           CLI argument definitions (clap)
```

---

## Data Flow

```
main.rs
  │
  ├─ Config::load()          reads ~/.config/some/config.toml
  ├─ Config::merge_cli()     CLI flags override config values
  ├─ Buffer::from_file()     or Buffer::from_stdin()
  ├─ SyntaxHighlighter::new()
  └─ App::new(buffers, config, highlighter)
       │
       └─ event_loop()
            │
            ├─ terminal.draw() ──► viewer::render(frame, app)
            │                         ├─ render_content()
            │                         ├─ statusbar::render()
            │                         └─ render_input_bar()
            │
            └─ input::handle_event(app, event)
                  ├─ handle_normal_key()
                  ├─ handle_search_key()
                  ├─ handle_command_key()
                  └─ handle_follow_key()
```

---

## Module Details

### `main.rs`
Entry point. Orchestrates startup: parses CLI args, loads config, opens buffers, creates `App`, enters raw mode, runs the event loop, restores terminal on exit. Terminal setup uses `crossterm` (`EnterAlternateScreen`, `EnableMouseCapture`). The event loop is `render → read event → mutate app → repeat`.

### `app.rs` — `App`
The single source of truth for all runtime state:

```rust
pub struct App {
    pub buffers: Vec<Buffer>,       // all open files
    pub active_buffer: usize,
    pub mode: Mode,                 // Normal | SearchInput | CommandInput | Follow
    pub top_line: usize,            // first visible line (viewport)
    pub left_col: usize,            // horizontal scroll offset
    pub content_height: usize,      // updated each frame from terminal size
    pub content_width: usize,
    pub search: SearchState,
    pub highlighter: SyntaxHighlighter,
    pub config: Config,
    pub show_line_numbers: bool,
    pub wrap_lines: bool,
    pub status_message: Option<String>,
    pub quit: bool,
}
```

`Mode` encodes the current interaction state — variants carry their own input buffer so there's no separate `input_buffer` field:

```rust
pub enum Mode {
    Normal,
    SearchInput { input: String, forward: bool },
    CommandInput { input: String },
    Follow,
}
```

All scrolling logic lives here (`scroll_down`, `scroll_up`, `goto_line`, `goto_top`, `goto_bottom`). `execute_search()` coordinates `SearchState` and `Buffer`.

### `buffer.rs` — `Buffer`
File content + O(1) line access. On load, does one pass over raw bytes to build `line_offsets: Vec<usize>` — byte offsets for the start of each line. `get_line(n)` slices directly into raw bytes and validates UTF-8. Strips trailing `\n`/`\r\n`.

Two storage strategies selected at load time:

| Condition | Strategy |
|-----------|----------|
| File size < `mmap_threshold` (default 10 MB) | `BufferSource::Memory(Vec<u8>)` |
| File size ≥ `mmap_threshold` | `BufferSource::Mmap(memmap2::Mmap)` |

For mmap'd files the OS manages page-in/out; the line index is the only heap allocation (~8 bytes/line).

### `viewer.rs`
Renders a frame using ratatui. Called once per event loop iteration via `terminal.draw()`. Layout:

```
┌─────────────────────────────────────┐
│  gutter │  content area             │  ← render_content()
│         │                           │
│       ~ │                           │  ← tilde for lines past EOF
├─────────┴───────────────────────────┤
│  status bar                         │  ← statusbar::render()
│  / search query or : command        │  ← render_input_bar()
└─────────────────────────────────────┘
```

Rendering pipeline per line:
1. Fetch raw text from `Buffer::get_line()`
2. If syntax enabled: run through `SyntaxHighlighter` → `Vec<StyledSpan>`
3. Overlay search match highlights (yellow bg) on top
4. Push resulting `ratatui::Line` to the paragraph

`render()` also updates `app.content_height` and `app.content_width` from the actual terminal size each frame.

### `input.rs`
Routes `crossterm::event::Event` to `App` mutations. Dispatch is by `app.mode`:

- **Normal** — navigation, mode transitions, toggles
- **SearchInput** — character accumulation into `Mode::SearchInput.input`, Enter commits to `app.search`
- **CommandInput** — same pattern, Enter calls `execute_command()`
- **Follow** — only `q`/`Esc`/`Ctrl-C` handled

`Resize` events update `app.content_width`/`content_height`. Mouse scroll maps to `scroll_down(3)`/`scroll_up(3)`.

### `search.rs` — `SearchState`
```rust
pub struct SearchState {
    pub pattern: Option<Regex>,
    pub query_string: String,
    pub matches: Vec<(usize, Range<usize>)>,  // (line_index, byte_range)
    pub current: usize,
    pub direction: SearchDirection,
}
```

`search_buffer()` does a full scan of all lines and collects every match location. `matches_on_line(n)` is called per-line during rendering. `next_match()`/`prev_match()` wrap around. Smart case: if `smart_case` is enabled and the query is all-lowercase, compiles with `case_insensitive(true)`.

### `syntax.rs` — `SyntaxHighlighter`
Thin wrapper around `syntect`. Holds a `SyntaxSet` (grammar definitions) and a loaded `Theme`. Key methods:

- `detect_syntax(path)` — looks up by file extension, falls back to plain text
- `create_highlight_lines(syntax)` → `HighlightLines` — stateful per-file highlighter (maintains lexer state across lines)
- `highlight_line(text, &mut hl)` → `Vec<StyledSpan>` — highlights one line, converts syntect colors to ratatui `Style`

**Note:** `HighlightLines` is stateful — it must be created at line 0 and called sequentially. Starting mid-file gives incorrect results for cross-line tokens (e.g., block comments). Current behavior: a new `HighlightLines` is created from the first visible line each frame, which is fast but slightly incorrect for tokens spanning lines above the viewport.

### `config.rs` — `Config`
Loaded from `~/.config/some/config.toml` (located via `dirs::config_dir()`). Deserializes with `serde`/`toml`. `Default` impl provides all defaults so a missing or partial config file works fine. `merge_cli()` applies CLI flag overrides after loading.

### `statusbar.rs` / `line_numbers.rs`
Focused single-function rendering modules. Both take `&App` and a `Rect`. The status bar renders: filename, buffer position indicator (when multiple files open), mode badge, search info, line range, and scroll percentage.

---

## Key Invariants

- `app.top_line` is always ≤ `app.max_top_line()` (clamped by all scroll methods)
- `app.active_buffer` is always a valid index into `app.buffers`
- `content_height` and `content_width` are updated at the start of every `viewer::render()` call before any rendering uses them
- `SearchState.matches` is always in line-ascending order (populated by iterating lines 0..N)
