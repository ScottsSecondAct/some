# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`some` is a modern terminal pager (like `less`) written in Rust — a TUI file viewer with syntax highlighting, regex search, mouse support, and large file handling. Currently at **v0.3**.

## Commands

```bash
cargo build                # Debug build
cargo build --release      # Optimized release build (LTO + stripped)
cargo install --path .     # Install to ~/.cargo/bin/some
cargo test                 # Run tests
cargo clippy               # Lint
cargo fmt                  # Format code
```

## Architecture

Event-driven TUI application built on `ratatui` + `crossterm`:

```
main.rs → config loading → CLI parsing → buffer loading → App
App (app.rs) runs: [Render] → [Drain channels] → [Read Input] → [Update State] → repeat
```

**Core modules:**
- `app.rs` — Central state: buffers, viewport offset, search state, config, key_map. Primary coordinator.
- `buffer.rs` — File content management. Transparently decompresses `.gz`/`.zst`/`.bz2`. Files ≥10 MB use `memmap2`; smaller files load into memory. Builds a line index for O(1) line access. Provides hex dump, git change indicators, diff buffer construction, and async search snapshots.
- `input.rs` — Keyboard (dispatches via `KeyMap` → `Action` enum) and mouse event handling.
- `viewer.rs` — Ratatui rendering pipeline: branches on binary (hex dump), diff, or normal text. Composes content area, gutter, status bar, and search input bar.
- `keymap.rs` — `Action` enum for all normal-mode operations; `KeyMap` with primary (user-overridable) and secondary (fixed alias) layers; `parse_key_spec()` for config string parsing.
- `search.rs` — Regex search state, match tracking, incremental preview (`preview_matches`), async search via `mpsc` (`SearchBatch` enum), smart-case logic.
- `syntax.rs` — Syntax highlighting via `syntect`. Loads bundled themes (Monokai, Dracula, Nord, Catppuccin-Mocha) and user themes from `~/.config/some/themes/`. Strips compression extensions for inner syntax detection.
- `config.rs` — Loads `~/.config/some/config.toml`, merges with CLI flags. Sections: `[general]` (includes `themes_dir`), `[colors]`, `[keys]` (custom keybindings via `KeysConfig`).
- `cli.rs` — `clap`-based argument parsing. Includes `--diff <FILE2>` for diff mode.
- `statusbar.rs` / `line_numbers.rs` — Focused rendering components. Status bar shows `[HEX]` and `[searching…]` indicators. Gutter colorizes the `│` separator by `GitChange`.

## Key Design Notes

- Multiple files are supported; navigate with `:n`/`:p` (next/previous buffer) or `[`/`]`.
- Follow mode (`F` key) uses `notify` for file watching (like `tail -f`).
- Clipboard support via `arboard`.
- Config file location: `~/.config/some/config.toml`.
- Async search: `execute_search()` spawns a `std::thread` with a text snapshot; `drain_search_results()` is called each event loop tick.
- Incremental search: `search_visible_lines()` populates `preview_matches` on every keystroke.
- Git gutter: shells out `git diff HEAD --unified=0`; called at open time and on reload.
- Diff mode: `Buffer::from_diff()` uses the `similar` crate; `buf.is_diff` skips syntax highlighting.
- Binary files: `buf.is_binary()` routes to hex dump rendering; no warning is printed.
- Keybindings: `KeyMap::build(&config.keys)` is called once in `App::new()`; secondary aliases (arrows, PgUp/Dn, Enter, Ctrl-C) cannot be overridden.
