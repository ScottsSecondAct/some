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

## v0.2 — Polish & Power Features ✅

- [x] **Follow mode — file watching** (`notify` crate wired into the event loop; reload buffer on file append, auto-scroll to bottom)
- [x] **Search highlight overlay on syntax spans** (syntax and search highlights coexist; search ranges overlaid on top of syntect spans)
- [x] **Clipboard** — visual selection mode (`v`), yank with `y` via `arboard`
- [x] **Marks** — `m` + char to set, `'` + char to jump
- [x] **Filtered view** — `&` + pattern shows only matching lines (grep-inside-pager)
- [x] **Tab bar** — visual indicator when multiple files are open, active buffer highlighted
- [x] **Backward search direction** — `?` enters backward search; `n`/`N` respect direction
- [x] **`[` / `]` keybindings** — prev/next buffer in normal mode (alongside `:n`/`:p`)

---

## v0.3 — Power User ✅

- [x] **Custom themes** — 4 bundled presets (Monokai, Dracula, Nord, Catppuccin-Mocha) embedded at compile time; user `.tmTheme` files loaded from `~/.config/some/themes/`; compression extensions stripped for inner syntax detection
- [x] **Git gutter** — shells out `git diff HEAD --unified=0`; added/modified/deleted line indicators shown in the line-number separator (green `│` / yellow `│` / red `▾`)
- [x] **Diff mode** — `some file1 --diff file2` unified diff in a single pane; colorized +/- lines via the `similar` crate
- [x] **Compressed file support** — transparent `.gz`, `.zst`/`.zstd`, `.bz2` decompression via `flate2`/`zstd`/`bzip2`; inner extension used for syntax detection (e.g. `file.rs.gz` → Rust highlighting)
- [x] **Hex dump fallback** — binary files shown as hex+ASCII dump instead of a warning; `[HEX]` mode indicator in status bar
- [x] **Async search** — search runs in a `std::thread`; results stream back via `mpsc` as `SearchBatch::Progress`/`Done`; status bar shows live match count and lines scanned
- [x] **Incremental search** — amber highlights appear in the viewport as the query is typed, before Enter is pressed; committed matches (bright yellow) replace them on confirm
- [x] **Config: custom keybindings** — `[keys]` section in `config.toml`; any normal-mode action can be rebound; secondary aliases (arrow keys, PgUp/Dn, Enter) are always available

---

## Performance Targets

| Scenario | Target |
|----------|--------|
| Cold start, 100 KB source file | < 50 ms to first render |
| 1 GB log file (mmap) | < 100 ms to open, ~80 MB index memory |
| Search across 1M-line file | Results start streaming < 200 ms |
