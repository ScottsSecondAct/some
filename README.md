# some
![AI Assisted](https://img.shields.io/badge/AI%20Assisted-Claude-blue?logo=anthropic)

A fully functional terminal pager built in **Rust**, with syntax highlighting, regex search, mouse support, and a clean TUI — like `less`, but more. Now at **v0.3** with custom themes, git change indicators, unified diff view, transparent decompression, hex dump of binary files, async/incremental search, and configurable keybindings.

## Why This Project

`less` is one of the most-used programs on any Unix system, yet it hasn't fundamentally changed in decades. Its interface is opaque, its output is plain, and inspecting a source file means squinting at monochrome text. Modern terminals support true color, mouse input, and Unicode — but most pagers ignore all of it.

Building a pager from scratch is a deceptively interesting systems problem. It touches file I/O and memory layout (how do you navigate a 2 GB log file without reading it all into memory?), terminal control (raw mode, alternate screens, escape sequences), incremental rendering (only redraw what changed), text processing (UTF-8 boundary safety, regex matching, syntax tokenization), and event-driven architecture. Every `less` keybinding you've ever typed has a state machine behind it.

This project was developed with AI assistance (Anthropic's Claude) as a design and implementation collaborator — the same way a professional engineer uses documentation, a senior colleague, or Stack Overflow. Architecture decisions, module boundaries, and every tradeoff were made and understood by hand. The AI accelerated the work; it didn't replace the thinking.

## What It Looks Like

```
┌─────────────────────────────────────────────────────────────────────┐
│   1 │ use ratatui::prelude::*;                                       │
│   2 │ use ratatui::widgets::Paragraph;                               │
│   3 │                                                                 │
│   4 │ use crate::app::{App, Mode};                                   │
│   5 │ use crate::line_numbers;                                        │
│   6 │ use crate::statusbar;                                           │
│   7 │                                                                 │
│   8 │ pub fn render(frame: &mut Frame, app: &mut App) {              │
│   9 │     let area = frame.area();                                    │
│  10 │     app.content_height = (area.height as usize)                │
│     │                                                                 │
│ viewer.rs [1/3] [SEARCH]  /render (4 matches)  8-10/124 │ 8%        │
│ /render                                                               │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

- **Syntax highlighting** — auto-detected from file extension via TextMate grammars (200+ languages)
- **Custom themes** — 4 bundled presets (Monokai, Dracula, Nord, Catppuccin-Mocha) plus user `.tmTheme` files from `~/.config/some/themes/`
- **Regex search** — `/` forward, `?` backward, `n`/`N` navigate respecting direction; smart case; all matches highlighted in the viewport *alongside* syntax coloring
- **Incremental search** — amber highlights appear in the viewport as the query is typed; bright yellow on commit
- **Async search** — full-file search runs in a background thread; results stream to the UI with a live match counter
- **Line numbers** — toggleable gutter with git change indicators (`l` key)
- **Git gutter** — green/yellow/red markers on the line-number separator show added, modified, and deleted lines
- **Mouse support** — scroll wheel works out of the box
- **Large file handling** — memory-mapped I/O (`mmap`) for files above 10 MB; only the line index is heap-allocated
- **Compressed files** — transparent `.gz`, `.zst`/`.zstd`, `.bz2` decompression; inner extension used for syntax detection
- **Hex dump** — binary files are displayed as a hex+ASCII dump; `[HEX]` indicator in the status bar
- **Unified diff** — `some file1 --diff file2` shows a colorized unified diff in a single pane
- **Stdin piping** — `cat file | some` works
- **Multiple files** — `some f1 f2 f3`, switch with `:n`/`:p` or `[`/`]`; tab bar shows all open files
- **Follow mode** — `F` key tails a file for new content, like `tail -f`; backed by `notify` file watching
- **Filtered view** — `&` + regex keeps only matching lines visible; `Esc` to clear
- **Visual selection** — `v` enters visual mode; `j`/`k` extend the selection; `y` yanks to the system clipboard
- **Marks** — `m<c>` sets a named mark, `'<c>` jumps back to it
- **Line wrap toggle** — `w` key; horizontal scroll otherwise
- **Custom keybindings** — override any normal-mode key in `[keys]` config section
- **Config file** — `~/.config/some/config.toml` for theme, colors, keybindings, and defaults

## Architecture

`some` is an event-driven TUI application built on `ratatui` and `crossterm`. The startup sequence loads config and buffers once; the runtime is a tight render → drain channels → input loop.

```
 startup
    │
    ├─ Config::load()          reads ~/.config/some/config.toml
    ├─ Config::merge_cli()     CLI flags take precedence over config
    ├─ Buffer::from_diff()     (--diff mode)  or
    │  Buffer::from_file()     (decompresses .gz/.zst/.bz2, mmap or heap)  or
    │  Buffer::from_stdin()
    ├─ SyntaxHighlighter::new()  loads bundled + user .tmTheme files
    ├─ App::new()              builds KeyMap, loads git changes per buffer
    └─ App::start_watching()   spawns notify watcher for all file paths
         │
         └─ event_loop()  ← poll-based, 200 ms timeout
               │
               ├─ terminal.draw() ──► viewer::render(frame, &mut app)
               │                          ├─ render_tab_bar()  (when >1 buffer)
               │                          ├─ render_content()
               │                          │    ├─ if binary → hex dump rows
               │                          │    ├─ if diff   → colorized +/-/@@ lines
               │                          │    └─ otherwise
               │                          │         ├─ line_numbers::render() (git colors)
               │                          │         ├─ SyntaxHighlighter → StyledSpans
               │                          │         ├─ amber preview overlay
               │                          │         └─ yellow match overlay
               │                          ├─ statusbar::render()
               │                          └─ render_input_bar()
               │
               ├─ watcher_rx.try_recv()   file-change events → reload + goto_bottom
               │
               ├─ app.drain_search_results()  async search batches via mpsc
               │
               └─ input::handle_event(&mut app, event)
                     ├─ Mode::Normal       dispatch via KeyMap → Action enum
                     │    └─ pending_key   two-key sequences: m<c>, '<c>
                     ├─ Mode::SearchInput  char accumulation + live preview update
                     ├─ Mode::CommandInput `:q`, `:<N>`, `:n`, `:p`
                     ├─ Mode::Follow       only q / Esc / Ctrl-C handled
                     ├─ Mode::FilterInput  char accumulation → apply_filter()
                     └─ Mode::Visual       j/k extend selection, y yanks
```

All runtime state lives in `App`. The three background channels all sit on `App` and are drained non-blockingly each tick: the `notify` file-watcher receiver, the async search `mpsc::Receiver<SearchBatch>`, and (implicitly) the terminal event queue.

## Technical Highlights

### Memory-Mapped File I/O

Files above 10 MB are opened with `memmap2` rather than read into a `Vec<u8>`. The OS pages in only the regions that are actually accessed — navigating to line 500,000 of a 2 GB log file reads only the pages that contain that region. The entire heap allocation for a 1 GB file with 10 million lines is ~80 MB for the line index (8 bytes per line offset), regardless of file size.

The line index is built in a single forward pass at open time, recording the byte offset of each `\n`. `get_line(n)` is then O(1): slice `data[offsets[n]..offsets[n+1]]`, strip the trailing newline, validate UTF-8. Both storage strategies (`Mmap` and `Memory`) share the same `as_bytes()` interface via an internal enum, so the rest of the code never branches on storage type.

### Transparent Decompression

When a file has a `.gz`, `.zst`/`.zstd`, or `.bz2` extension, `Buffer::from_file()` pipes it through the appropriate decoder (`flate2`, `zstd`, or `bzip2`) before indexing. The mmap path is skipped — decompressed content lives in a `Vec<u8>`. The original path is retained so `reload()` can re-decompress on follow-mode updates. Syntax detection strips the compression extension to find the inner type, so `server.log.gz` gets log highlighting and `main.rs.gz` gets Rust highlighting.

### Async Search with Incremental Preview

Two layers of search feedback:

1. **Incremental preview**: while the query is being typed, `search_visible_lines()` scans only the current viewport and populates `preview_matches`, rendered in amber. This is synchronous and fast because it covers at most a screenful of lines.

2. **Async full-file search**: on Enter, `execute_search()` clones the buffer into a `Vec<String>` snapshot (safe to send across threads), spawns a `std::thread`, and streams results back via `std::sync::mpsc`. The event loop drains `SearchBatch::Progress` messages each tick, extending `search.matches` and updating the status bar with a live count. On `SearchBatch::Done`, the viewport jumps to the first match.

### Git Gutter

`load_git_changes()` shells out `git diff HEAD --unified=0 -- <path>`, parses `@@ -old +new @@` hunk headers from stdout, and records a `GitChange` (Added / Modified / Deleted) for each affected line index. This runs at buffer-open time and on every reload. The line-number gutter colorizes the `│` separator based on these entries: green for added lines, yellow for modified, red `▾` for the line before a deletion point.

### Syntax Highlighting and Stateful Lexers

Syntax highlighting uses `syntect`, which processes grammars in TextMate format. The key subtlety: `HighlightLines` is **stateful** — it maintains the lexer's parse state across lines so that multi-line constructs (block comments, string literals, heredocs) are highlighted correctly. A new `HighlightLines` is created from the top of the visible window each render — fast in practice, but tokens spanning lines above the viewport may not be highlighted correctly. Proper handling requires caching parse state at regular line intervals (planned).

### Smart Case Search

Search is smart-case by default: if the query is entirely lowercase, the regex is compiled with `case_insensitive(true)`. The moment any uppercase character appears, the search becomes case-sensitive. This is a single `RegexBuilder` call — no preprocessing needed.

### TUI Rendering and the Viewport

The viewport is two numbers: `top_line` and the terminal dimensions (`content_height`, `content_width`). Every scroll clamps `top_line` to `[0, total_lines - content_height]`. Dimensions are read from the actual frame size at the start of every `render()` call — resize events are handled implicitly.

`render_content()` branches on `buf.is_binary()` (hex dump), `buf.is_diff` (diff colorization), then falls through to the normal path that merges syntax spans with preview and committed search ranges.

### Configurable Keybindings

`keymap.rs` defines an `Action` enum for every normal-mode operation and a `KeyMap` struct with two layers: a **primary** map built from hardcoded defaults overridden by `[keys]` config values, and a **secondary** alias map for arrow keys, PgUp/Dn, Home/End, Enter, and Ctrl-C that cannot be overridden. `parse_key_spec()` translates strings like `"ctrl+d"`, `"space"`, or `"G"` to `(KeyCode, KeyModifiers)` pairs.

## Build

**Requirements:** Rust 1.70+

```sh
git clone https://github.com/you/some
cd some

cargo build --release          # optimized binary → ./target/release/some
cargo install --path .         # install to ~/.cargo/bin/some
cargo test                     # run unit tests
cargo clippy                   # lint
```

## Usage

```sh
some file.rs                        # view a file
some -n file.rs                     # with line numbers
some -p "fn main" file.rs           # open with search pre-highlighted
some -N 150 file.rs                 # jump to line 150
some -f server.log                  # follow mode (tail -f)
some f1.rs f2.rs f3.rs              # multiple files
cat build.log | some                # pipe from stdin
some --plain output.log             # no colors, no numbers
some -t Monokai file.rs             # choose theme
some file.rs.gz                     # view compressed file
some /bin/ls                        # binary → hex dump
some old.rs --diff new.rs           # unified diff view
```

| Flag | Description |
|------|-------------|
| `-n`, `--line-numbers` | Show line numbers |
| `-f`, `--follow` | Follow mode (tail -f) |
| `-N <LINE>` | Start at line N |
| `-p <REGEX>` | Pre-highlight a search pattern |
| `-w`, `--wrap` | Enable line wrapping |
| `-t <THEME>` | Color theme name |
| `--no-syntax` | Disable syntax highlighting |
| `--plain` | No colors, no line numbers |
| `--tab-width <N>` | Tab display width (default: 4) |
| `--diff <FILE2>` | Show unified diff against FILE2 |

## Keybindings

All normal-mode bindings can be overridden in `[keys]` config. The defaults:

### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` / `Enter` | Scroll down one line |
| `k` / `↑` | Scroll up one line |
| `d` / `Ctrl-D` | Half page down |
| `u` / `Ctrl-U` | Half page up |
| `Space` / `PgDn` | Full page down |
| `b` / `PgUp` | Full page up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `←` / `→` | Scroll horizontally (4 cols) |

### Search
| Key | Action |
|-----|--------|
| `/` | Search forward (amber preview while typing) |
| `?` | Search backward |
| `n` | Next match (respects direction) |
| `N` | Previous match (respects direction) |

### View & Display
| Key | Action |
|-----|--------|
| `l` | Toggle line numbers |
| `w` | Toggle line wrap |
| `&` | Filter — show only matching lines |
| `F` | Follow mode (tail -f) |

### Marks
| Key | Action |
|-----|--------|
| `m<c>` | Set mark at current position |
| `'<c>` | Jump to mark `<c>` |

### Visual Selection
| Key | Action |
|-----|--------|
| `v` | Enter visual line-selection mode |
| `j` / `k` | Extend selection down / up |
| `y` | Yank selection to clipboard |
| `Esc` | Exit visual mode |

### Buffers & Commands
| Key | Action |
|-----|--------|
| `[` / `]` | Previous / next file |
| `:n` / `:p` | Previous / next file (command mode) |
| `:<N>` | Jump to line N |
| `:q` | Quit |
| `q` / `Ctrl-C` | Quit |

## Configuration

Copy `config.example.toml` to `~/.config/some/config.toml`. All fields are optional.

```toml
[general]
theme = "base16-ocean.dark"   # syntax highlight theme
line_numbers = false          # show line numbers by default
wrap = false                  # wrap long lines
tab_width = 4                 # tab display width
mouse = true                  # enable mouse scroll
smart_case = true             # case-insensitive unless query has uppercase
# themes_dir = "~/.config/some/themes"  # directory for extra .tmTheme files

[colors]
status_bar_bg = "#2b303b"
status_bar_fg = "#c0c5ce"
search_match_bg = "#ebcb8b"
search_match_fg = "#2b303b"
line_number_fg = "#65737e"

[keys]
# Override any normal-mode binding. Unset = keep default.
# scroll_down    = "j"
# half_page_down = "ctrl+d"
# goto_bottom    = "G"
# (see config.example.toml for the full list)
```

**Bundled themes:** `Monokai`, `Dracula`, `Nord`, `Catppuccin-Mocha` plus all syntect built-ins (`base16-ocean.dark` default, `base16-eighties.dark`, `base16-mocha.dark`, `InspiredGitHub`, `Solarized (dark)`, `Solarized (light)`).

**User themes:** Drop any `.tmTheme` file into `~/.config/some/themes/` (or the directory set by `themes_dir`) and pass its name to `-t`.

CLI flags always override config file settings.

## Project Structure & Roadmap

See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for a full module breakdown and architecture diagram.

See [ROADMAP.md](ROADMAP.md) for planned features and version milestones.

## Development Process & AI Collaboration

This project was built with AI assistance (Claude) as a design partner and implementation accelerator:

- **Architecture**: Module boundaries, the `App` state machine, mmap strategy, the `Mode`-as-data-enum design, and the two-layer `KeyMap` were designed collaboratively and then implemented by hand.
- **Debugging**: When modules were generated against mismatched APIs (wrong field names, missing module declarations, lifetime errors in `Span` borrows), Claude helped diagnose and fix root causes from compiler output rather than guessing.
- **Tradeoffs**: Decisions like where to clip viewport state, how to handle `HighlightLines` statefulness, why search results are stored eagerly, and how to safely share buffer content across threads (snapshot vs. Arc) were explicit discussions, not implicit choices.

Every line was reviewed and understood before integration. The AI didn't write the pager; it made it possible to build a complete, working tool in a compressed timeframe while actually understanding what was built.

## Skills Demonstrated

- **Rust**: Ownership and borrowing in a real application — lifetime annotations on `Span`, enum ownership patterns, safe `unsafe` with mmap, cross-thread data sharing via snapshot
- **Systems programming**: Memory-mapped I/O, raw terminal mode, byte-level line indexing, binary file detection, transparent decompression
- **TUI development**: `ratatui` layout system, `crossterm` event handling, alternate screen management, stateful rendering
- **Concurrency**: `std::thread` + `std::sync::mpsc` for async search; non-blocking channel draining in an event loop
- **Event-driven architecture**: Mode state machine, clean separation between input handling and rendering, viewport invariants, configurable key dispatch via `Action` enum
- **Text processing**: UTF-8 boundary safety, regex compilation with `RegexBuilder`, syntect TextMate grammar integration, unified diff generation via `similar`
- **Rust tooling**: Cargo, `clippy`, release profile tuning (LTO, `opt-level = 3`, `strip = true`)

## License

MIT — Copyright (c) 2026 Scott Davis
