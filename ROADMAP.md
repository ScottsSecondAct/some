# Roadmap

## v0.1 — MVP ✅

Core pager functionality. All items below are implemented and building.

- [x] File display with scrolling (`j`/`k`, `d`/`u`, `Space`/`b`, `g`/`G`, arrow keys, PgUp/PgDn)
- [x] Line numbers — toggleable gutter (`-n` flag, `l` key)
- [x] Syntax highlighting — auto-detect from extension via syntect
- [x] Regex search — `/` to enter, `n`/`N` to navigate, all matches highlighted
- [x] Smart case — case-insensitive unless query contains uppercase
- [x] Status bar — filename, line range, scroll percentage, match count
- [x] Stdin support — `cat file | some`
- [x] Mouse scroll
- [x] Multiple files — `:n`/`:p` to switch buffers
- [x] Follow mode — `F` key (state machine; file watching not yet wired)
- [x] Horizontal scroll — `←`/`→` keys
- [x] Line wrap toggle — `w` key
- [x] Command mode — `:q`, `:<N>` jump to line, `:n`/`:p`
- [x] Binary file detection — warns on open
- [x] Large file handling — mmap for files ≥ 10 MB
- [x] Config file — `~/.config/some/config.toml`

---

## v0.2 — Polish & Power Features

- [ ] **Follow mode — file watching** (`notify` crate wired into the event loop; reload buffer on file append, auto-scroll to bottom)
- [ ] **Search highlight overlay on syntax spans** (currently: syntax highlight OR search highlight; need both simultaneously)
- [ ] **Clipboard** — visual selection mode (`v`), yank with `y` via `arboard`
- [ ] **Marks** — `m` + char to set, `'` + char to jump
- [ ] **Filtered view** — `&` + pattern shows only matching lines (grep-inside-pager)
- [ ] **Tab bar** — visual indicator when multiple files are open
- [ ] **Backward search direction** — `?` key wired up (state exists, logic needs to mirror `n`/`N` in reverse)
- [ ] **`:n` / `:p` keybindings** — also bind `[` / `]` in normal mode for buffer switching

---

## v0.3 — Power User

- [ ] **Custom themes** — load `.tmTheme` files from `~/.config/some/themes/`; ship presets (Monokai, Dracula, Nord, Catppuccin)
- [ ] **Git gutter** — show added/modified/deleted line indicators when file is inside a git repo
- [ ] **Diff mode** — `some --diff file1 file2` side-by-side view
- [ ] **Compressed file support** — transparent `.gz`, `.zst`, `.bz2` decompression
- [ ] **Hex dump fallback** — binary files shown as hex+ASCII rather than warned and dumped as-is
- [ ] **Async search** — run regex across large mmap'd files in a background thread, stream results to UI
- [ ] **Incremental search** — highlight matches as query is typed (before pressing Enter)
- [ ] **Config: custom keybindings** — override any binding in `[keys]` config section

---

## Performance Targets

| Scenario | Target |
|----------|--------|
| Cold start, 100 KB source file | < 50 ms to first render |
| 1 GB log file (mmap) | < 100 ms to open, ~80 MB index memory |
| Search across 1M-line file | Results start streaming < 200 ms |
