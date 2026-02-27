# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`some` is a modern terminal pager (like `less`) written in Rust — a TUI file viewer with syntax highlighting, regex search, mouse support, and large file handling.

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
App (app.rs) runs: [Render] → [Read Input] → [Update State] → repeat
```

**Core modules:**
- `app.rs` — Central state: buffers, viewport offset, search state, config. Primary coordinator.
- `buffer.rs` — File content management. Files ≥10 MB use `memmap2` for memory-mapped I/O; smaller files load fully into memory. Builds a line index for O(1) line access.
- `input.rs` — Keyboard (vim-style bindings) and mouse event handling. Maps events to `App` method calls.
- `viewer.rs` — Ratatui rendering pipeline: composes content area, gutter, status bar, and search input bar.
- `search.rs` — Regex search state, match tracking, smart-case logic (case-insensitive unless query contains uppercase).
- `syntax.rs` — Syntax highlighting via `syntect` (200+ languages, auto-detected by filename extension).
- `config.rs` — Loads `~/.config/some/config.toml`, merges with CLI flags. See `config.example.toml` for all options.
- `cli.rs` — `clap`-based argument parsing.
- `statusbar.rs` / `line_numbers.rs` — Focused rendering components.

## Key Design Notes

- Multiple files are supported; navigate with `:n`/`:p` (next/previous buffer).
- Follow mode (`F` key) uses `notify` for file watching (like `tail -f`).
- Clipboard support via `arboard`.
- Config file location: `~/.config/some/config.toml`.
