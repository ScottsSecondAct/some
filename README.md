# some
![AI Assisted](https://img.shields.io/badge/AI%20Assisted-Claude-blue?logo=anthropic)

A fully functional terminal pager built in **Rust**, with syntax highlighting, regex search, mouse support, and a clean TUI — like `less`, but more.

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
- **Regex search** — `/` to search, `n`/`N` to navigate, all matches highlighted in the viewport; smart case (case-insensitive unless query contains uppercase)
- **Line numbers** — toggleable gutter, `l` key
- **Mouse support** — scroll wheel works out of the box
- **Large file handling** — memory-mapped I/O (`mmap`) for files above 10 MB; only the line index is heap-allocated
- **Stdin piping** — `cat file | some` works
- **Multiple files** — `some f1 f2 f3`, switch with `:n`/`:p`
- **Follow mode** — `F` key tails a file for new content, like `tail -f`
- **Line wrap toggle** — `w` key; horizontal scroll otherwise
- **Config file** — `~/.config/some/config.toml` for theme, colors, and defaults

## Architecture

`some` is an event-driven TUI application built on `ratatui` and `crossterm`. The startup sequence loads config and buffers once; the runtime is a tight render → input → mutate loop.

```
 startup
    │
    ├─ Config::load()          reads ~/.config/some/config.toml
    ├─ Config::merge_cli()     CLI flags take precedence over config
    ├─ Buffer::from_file()     indexes line offsets; chooses mmap or heap
    └─ App::new()              wires buffers, config, and highlighter together
         │
         └─ event_loop()
               │
               ├─ terminal.draw() ──► viewer::render(frame, &mut app)
               │                          ├─ update content_height / content_width
               │                          ├─ render_content()
               │                          │    ├─ line_numbers::render()
               │                          │    ├─ SyntaxHighlighter → StyledSpans
               │                          │    └─ search highlight overlay
               │                          ├─ statusbar::render()
               │                          └─ render_input_bar()
               │
               └─ input::handle_event(&mut app, event)
                     ├─ Mode::Normal       navigation, toggles, mode transitions
                     ├─ Mode::SearchInput  character accumulation → execute_search()
                     ├─ Mode::CommandInput `:q`, `:<N>`, `:n`, `:p`
                     └─ Mode::Follow       only q / Esc / Ctrl-C handled
```

All runtime state lives in `App`. Modules communicate by reading from and writing to `App` fields — there are no channels or shared state. The mode enum carries its own input buffer as data (`SearchInput { input: String, forward: bool }`), so there's no separate accumulator field to keep in sync.

## Technical Highlights

### Memory-Mapped File I/O

Files above 10 MB are opened with `memmap2` rather than read into a `Vec<u8>`. The OS pages in only the regions that are actually accessed — navigating to line 500,000 of a 2 GB log file reads only the pages that contain that region. The entire heap allocation for a 1 GB file with 10 million lines is ~80 MB for the line index (8 bytes per line offset), regardless of file size.

The line index is built in a single forward pass at open time, recording the byte offset of each `\n`. `get_line(n)` is then O(1): slice `data[offsets[n]..offsets[n+1]]`, strip the trailing newline, validate UTF-8. Both storage strategies (`Mmap` and `Memory`) share the same `as_bytes()` interface via an internal enum, so the rest of the code never branches on storage type.

### Syntax Highlighting and Stateful Lexers

Syntax highlighting uses `syntect`, which processes grammars written in TextMate format. The key subtlety: `HighlightLines` is **stateful** — it maintains the lexer's parse state across lines so that multi-line constructs (block comments, string literals, heredocs) are highlighted correctly. This means you cannot highlight an arbitrary line in isolation; you must feed lines sequentially from some known-good starting state.

The current implementation creates a fresh `HighlightLines` at the top of the visible window each render. For typical files and viewport sizes this is imperceptible, but it means syntax state from above the viewport is not carried in — a block comment opened on line 1 may not be highlighted correctly at line 3000 if the viewport starts there. Proper handling requires caching parse state at regular line intervals (a planned improvement).

### Smart Case Search

Search is smart-case by default, matching the behavior of ripgrep: if the query string is entirely lowercase, the regex is compiled with `case_insensitive(true)`. The moment any uppercase character appears in the query, the search becomes case-sensitive. This is implemented in a single `RegexBuilder` call — no preprocessing of the query string needed — and the behavior is toggled by the `smart_case` config option.

All match locations are computed eagerly on search commit (`search_buffer()` scans all lines and records `(line_index, byte_range)` pairs). Per-render, `matches_on_line(n)` filters this list for the visible range. For large files, search will move to a background thread with streamed results (see roadmap).

### TUI Rendering and the Viewport

The viewport is just two numbers: `top_line` (first visible line) and the terminal dimensions (`content_height`, `content_width`). Every scroll operation clamps `top_line` to `[0, total_lines - content_height]`. The terminal dimensions are read from the actual frame size at the start of every `render()` call, so resize events are handled implicitly — there's no separate resize handler that could get out of sync.

The layout splits into three horizontal bands (content, status bar, input bar) using ratatui's `Layout`. Content is further split vertically into gutter and text areas when line numbers are on. The gutter width is computed from the digit count of `total_lines`, so it never wastes space.

### Mode as a State Machine

The interaction mode is an enum where each variant owns its relevant data:

```rust
pub enum Mode {
    Normal,
    SearchInput { input: String, forward: bool },
    CommandInput { input: String },
    Follow,
}
```

Input handling is a pure `match` on mode, then a nested `match` on the key. Transitions are mutations of `app.mode`. There's no implicit state lurking in separate fields — if the mode changes, the associated data changes with it. The input bar renders directly by pattern-matching on the current mode, so it's impossible for the display to lag behind the actual state.

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
some file.rs                   # view a file
some -n file.rs                # with line numbers
some -p "fn main" file.rs      # open with search pre-highlighted
some -N 150 file.rs            # jump to line 150
some -f server.log             # follow mode (tail -f)
some f1.rs f2.rs f3.rs         # multiple files
cat build.log | some           # pipe from stdin
some --plain output.log        # no colors, no numbers
some -t "Solarized (dark)" f   # choose theme
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

## Keybindings

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
| `←` / `→` | Scroll horizontally |
| `/` | Search forward |
| `?` | Search backward |
| `n` / `N` | Next / previous match |
| `l` | Toggle line numbers |
| `w` | Toggle line wrap |
| `F` | Follow mode |
| `:n` / `:p` | Next / previous file |
| `:<N>` | Jump to line N |
| `q` / `Ctrl-C` | Quit |

## Configuration

Copy `config.example.toml` to `~/.config/some/config.toml`. All fields are optional — missing values use defaults.

```toml
[general]
theme = "base16-ocean.dark"   # syntax highlight theme
line_numbers = false          # show line numbers by default
wrap = false                  # wrap long lines
tab_width = 4                 # tab display width
mouse = true                  # enable mouse scroll
smart_case = true             # case-insensitive unless query has uppercase

[colors]
status_bar_bg = "#2b303b"
status_bar_fg = "#c0c5ce"
search_match_bg = "#ebcb8b"
search_match_fg = "#2b303b"
line_number_fg = "#65737e"
```

Built-in themes (from syntect): `base16-ocean.dark` (default), `base16-eighties.dark`, `base16-mocha.dark`, `InspiredGitHub`, `Solarized (dark)`, `Solarized (light)`.

CLI flags always override config file settings.

## Project Structure & Roadmap

See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for a full module breakdown and architecture diagram.

See [ROADMAP.md](ROADMAP.md) for planned features and version milestones.

## Development Process & AI Collaboration

This project was built with AI assistance (Claude) as a design partner and implementation accelerator:

- **Architecture**: Module boundaries, the `App` state machine, mmap strategy, and the `Mode`-as-data-enum design were designed collaboratively and then implemented by hand.
- **Debugging**: When modules were generated against mismatched APIs (wrong field names, missing module declarations, lifetime errors in `Span` borrows), Claude helped diagnose and fix root causes from compiler output rather than guessing.
- **Tradeoffs**: Decisions like where to clip viewport state, how to handle `HighlightLines` statefulness, and why search results are stored eagerly rather than computed on-demand were explicit discussions, not implicit choices.

Every line was reviewed and understood before integration. The AI didn't write the pager; it made it possible to build a complete, working tool in a compressed timeframe while actually understanding what was built.

## Skills Demonstrated

- **Rust**: Ownership and borrowing in a real application — lifetime annotations on `Span`, enum ownership patterns, `Cow` vs `String` tradeoffs, safe `unsafe` with mmap
- **Systems programming**: Memory-mapped I/O, raw terminal mode, byte-level line indexing, binary file detection
- **TUI development**: `ratatui` layout system, `crossterm` event handling, alternate screen management, stateful rendering
- **Event-driven architecture**: Mode state machine, clean separation between input handling and rendering, viewport invariants
- **Text processing**: UTF-8 boundary safety, regex compilation with `RegexBuilder`, syntect TextMate grammar integration
- **Rust tooling**: Cargo, `clippy`, release profile tuning (LTO, `opt-level = 3`, `strip = true`)

## License

MIT
