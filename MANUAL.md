# some — User Manual

**Version 0.3** · Copyright © 2026 Scott Davis · MIT License

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Opening Files](#3-opening-files)
4. [The Interface](#4-the-interface)
5. [Navigation](#5-navigation)
6. [Search](#6-search)
7. [Filtered View](#7-filtered-view)
8. [Visual Selection and Clipboard](#8-visual-selection-and-clipboard)
9. [Marks](#9-marks)
10. [Multiple Files](#10-multiple-files)
11. [Follow Mode](#11-follow-mode)
12. [Viewing Binary Files](#12-viewing-binary-files)
13. [Compressed Files](#13-compressed-files)
14. [Git Change Indicators](#14-git-change-indicators)
15. [Diff Mode](#15-diff-mode)
16. [Command Mode](#16-command-mode)
17. [Configuration](#17-configuration)
18. [Themes](#18-themes)
19. [Custom Keybindings](#19-custom-keybindings)
20. [Command-Line Reference](#20-command-line-reference)
21. [Keybinding Reference](#21-keybinding-reference)

---

## 1. Introduction

`some` is a terminal file viewer — a modern replacement for `less`. It displays text files with syntax highlighting, lets you search with regular expressions, tail live log files, compare files with a built-in diff view, and inspect binary files as hex dumps. It is controlled entirely from the keyboard, with optional mouse scroll support.

If you already know `less` or `vim`, most of `some`'s keys will feel familiar. If you're new to terminal pagers, this manual covers everything you need.

---

## 2. Installation

**From source (requires Rust 1.70+):**

```sh
git clone https://github.com/ScottsSecondAct/some.git
cd some
cargo install --path .
```

This places the `some` binary in `~/.cargo/bin/`. Make sure that directory is on your `PATH`.

**Verify the installation:**

```sh
some --version
```

---

## 3. Opening Files

**View a single file:**

```sh
some file.txt
some src/main.rs
```

**View multiple files:**

```sh
some file1.txt file2.txt file3.txt
```

**Read from standard input:**

```sh
cat build.log | some
grep -r "error" . | some
command-that-produces-output | some
```

**Start at a specific line:**

```sh
some -N 250 server.log
```

**Open with a search pattern pre-highlighted:**

```sh
some -p "TODO" src/main.rs
some -p "error|warn" app.log
```

**Compare two files (diff view):**

```sh
some original.rs --diff modified.rs
```

---

## 4. The Interface

When `some` opens a file, the screen is divided into four areas:

```
┌─────────────────────────────────────────────────────────┐
│  tab bar  (only shown when more than one file is open)  │
├────────┬────────────────────────────────────────────────┤
│   1 │  │ fn main() {                                    │
│   2 │  │     let args = Cli::parse();                   │
│   3 │  │                                                │
│   4 │  │     let config = Config::load()?;              │
│     │  │ ~                                              │
│     │  │ ~                                              │
├────────┴────────────────────────────────────────────────┤
│ main.rs  /error (3 matches)  1-24/384 │ 6%             │
├─────────────────────────────────────────────────────────┤
│ q:quit  /:search  ?:back-search  &:filter  ...         │
└─────────────────────────────────────────────────────────┘
```

| Area | Description |
|------|-------------|
| **Tab bar** | Shows all open files; the active file is highlighted. Only visible with multiple files. |
| **Content area** | File content with optional line numbers and git gutter on the left. Lines past the end of file are shown as `~`. |
| **Status bar** | Filename, mode indicators, search info, line range, and scroll percentage. |
| **Input bar** | Shows the current mode prompt (search query, command, filter), or a key hint in Normal mode. |

### Status Bar Indicators

| Indicator | Meaning |
|-----------|---------|
| `[2/4]` | This is the 2nd of 4 open files |
| `[SEARCH]` | Search input mode is active |
| `[FILTER]` | Filter input mode is active |
| `[FOLLOW]` | Follow mode (tailing the file) |
| `[VISUAL]` | Visual selection mode |
| `[HEX]` | File is binary; displaying as hex dump |
| `[searching…]` | Async search is still running |

---

## 5. Navigation

`some` uses vim-style keys for navigation. Arrow keys and Page Up/Down also work everywhere.

### Basic Movement

| Key | Action |
|-----|--------|
| `j` or `↓` or `Enter` | Scroll down one line |
| `k` or `↑` | Scroll up one line |
| `d` or `Ctrl-D` | Half page down |
| `u` or `Ctrl-U` | Half page up |
| `Space` or `Page Down` | Full page down |
| `b` or `Page Up` | Full page up |
| `g` or `Home` | Jump to the top of the file |
| `G` or `End` | Jump to the bottom of the file |

### Horizontal Scrolling

When line wrap is off (the default), lines that extend beyond the terminal width are clipped. Scroll horizontally to see the rest:

| Key | Action |
|-----|--------|
| `→` | Scroll right 4 columns |
| `←` | Scroll left 4 columns |

### Display Toggles

| Key | Action |
|-----|--------|
| `l` | Toggle line numbers on/off |
| `w` | Toggle line wrapping on/off |

### Quitting

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Ctrl-C` | Quit |
| `:q` | Quit (command mode) |

---

## 6. Search

### Starting a Search

Press `/` to search forward through the file, or `?` to search backward. The input bar changes to a search prompt:

```
/pattern
```

Type your search pattern and press `Enter` to commit it. Press `Esc` to cancel without searching.

### Incremental Preview

As you type your pattern, `some` immediately highlights matches in the currently visible portion of the file in **amber**. This lets you refine your pattern before committing. When you press `Enter`, the full-file search runs and highlights change to **bright yellow**.

### Navigating Matches

| Key | Action |
|-----|--------|
| `n` | Jump to the next match |
| `N` | Jump to the previous match |

The direction of `n` and `N` respects whether you searched forward (`/`) or backward (`?`). After a forward search, `n` moves down the file. After a backward search, `n` moves up.

The status bar shows the current match position and total count:

```
/error (42 matches)  15-38/1204 │ 3%
```

### Regular Expressions

Search patterns are full regular expressions. Examples:

```
/TODO|FIXME          # match either word
/fn\s+\w+            # match function definitions
/^\s*//              # match comment lines
/\berror\b           # whole-word match
```

### Smart Case

By default, `some` uses smart-case matching: if your pattern is all lowercase, the search is case-insensitive. As soon as you include an uppercase letter, the search becomes case-sensitive.

| Pattern | Behavior |
|---------|----------|
| `error` | Matches `error`, `Error`, `ERROR` |
| `Error` | Matches `Error` only |

To disable smart case, set `smart_case = false` in your config file.

### Async Search on Large Files

For large files, the search runs in a background thread so the interface stays responsive. The status bar shows progress:

```
Searching… (1 247 matches, 83k lines)
```

You can keep scrolling and navigating while the search is running. Once complete, `n`/`N` navigate the full result set.

---

## 7. Filtered View

Filtering hides all lines that don't match a pattern, showing only the matching lines with their original line numbers intact. This is like running `grep` without leaving the pager.

**Enter filter mode:** Press `&`

The input bar shows `&` followed by your pattern. Press `Enter` to apply, `Esc` to cancel.

**Example:** To show only lines containing `ERROR`:

```
&ERROR
```

The status bar updates to show the filter is active:

```
main.rs [~ERROR 47L]
```

While a filter is active, scrolling and navigation operate over the filtered lines only. Line numbers in the gutter always reflect the original file positions.

**Clear the filter:** Press `Esc` while in Normal mode after a filter has been applied (re-enter Normal mode first if needed, then `&` again and `Esc`), or press `&` and submit an empty pattern.

> **Tip:** Combine filtering with search — filter to a relevant subset of lines, then search within those results.

---

## 8. Visual Selection and Clipboard

Visual mode lets you select a range of lines and copy them to the system clipboard.

### Entering Visual Mode

Press `v` in Normal mode. The current line is highlighted and becomes both the anchor and cursor of the selection.

```
-- VISUAL -- lines 10-10 (1 selected)  y:yank  Esc:cancel
```

### Extending the Selection

| Key | Action |
|-----|--------|
| `j` or `↓` | Extend selection down one line |
| `k` or `↑` | Shrink or extend selection up one line |

The selection always runs from the anchor line to the cursor line. Moving above the anchor extends upward; moving below extends downward.

### Copying

Press `y` to yank the selected lines to the system clipboard. `some` will report how many lines were copied and return to Normal mode:

```
Yanked 5 lines
```

### Cancelling

Press `Esc` or `q` to exit visual mode without copying.

---

## 9. Marks

Marks let you bookmark positions in a file and jump back to them instantly.

### Setting a Mark

Press `m` followed by any letter (`a`–`z`, `A`–`Z`):

```
m a     # set mark 'a' at the current scroll position
m t     # set mark 't'
```

The status bar confirms: `Mark 'a' set`

### Jumping to a Mark

Press `'` (single quote) followed by the mark letter:

```
' a     # jump to mark 'a'
```

Marks remember the `top_line` position at the time they were set. Jumping to a mark scrolls the viewport so that line is visible.

> **Tip:** Use marks when reading a long file — set a mark at an interesting location, continue reading, then jump back with `'` + the letter you chose.

---

## 10. Multiple Files

Open several files at once by listing them on the command line:

```sh
some src/*.rs
some main.rs lib.rs tests/integration.rs
```

### Switching Files

| Key | Action |
|-----|--------|
| `]` | Next file |
| `[` | Previous file |
| `:n` | Next file (command mode) |
| `:p` | Previous file (command mode) |

### Tab Bar

When more than one file is open, a tab bar appears at the top of the screen showing all filenames. The active file is highlighted in cyan. Files that don't fit on one line are truncated with `…` at the left.

### Buffer Indicator

The status bar shows the current position within the file list:

```
main.rs [2/5]
```

---

## 11. Follow Mode

Follow mode watches a file for new content and automatically scrolls to show it — like `tail -f`.

### Entering Follow Mode

Press `F` (or use the `--follow` / `-f` flag when opening):

```sh
some -f /var/log/syslog
some --follow server.log
```

The status bar shows `[FOLLOW]` and the viewport jumps to the bottom of the file. New lines are displayed as they are appended.

### Leaving Follow Mode

Press `q` or `Esc` to return to Normal mode.

> **Tip:** Follow mode works well for log files that grow continuously. `some` uses OS-level file watching (`inotify` on Linux), so it reacts immediately to new data rather than polling on a timer.

---

## 12. Viewing Binary Files

When `some` opens a file that contains binary data (null bytes detected in the first 8 KB), it automatically switches to **hex dump mode** rather than trying to display raw binary as text.

### Hex Dump Layout

Each row shows 16 bytes:

```
00000000  7f 45 4c 46 02 01 01 00  00 00 00 00 00 00 00 00  |.ELF............|
00000010  02 00 3e 00 01 00 00 00  30 10 40 00 00 00 00 00  |..>.....0.@.....|
00000020  40 00 00 00 00 00 00 00  b0 3a 00 00 00 00 00 00  |@........:......|
```

| Column | Content |
|--------|---------|
| Left | Byte offset (hexadecimal) |
| Middle | 16 bytes in hex, split into two groups of 8 |
| Right | ASCII representation; non-printable bytes shown as `.` |

The `[HEX]` indicator appears in the status bar. Navigation works the same as for text files — scroll through the dump with `j`/`k`, jump to top/bottom with `g`/`G`, etc.

---

## 13. Compressed Files

`some` transparently decompresses the following formats before displaying:

| Extension | Format |
|-----------|--------|
| `.gz` | gzip |
| `.zst`, `.zstd` | Zstandard |
| `.bz2` | bzip2 |

Simply open the compressed file as you would any other:

```sh
some access.log.gz
some backup.tar.gz       # shows the raw tar stream
some data.json.zst
```

Syntax highlighting is applied based on the inner filename. For example, `main.rs.gz` is highlighted as Rust, and `config.yaml.bz2` is highlighted as YAML.

In follow mode, `some` re-decompresses the file on each reload cycle.

---

## 14. Git Change Indicators

When viewing a file that is tracked in a git repository, `some` shows change indicators in the line-number gutter alongside each modified line. These reflect the diff between the working tree and `HEAD`.

### Indicator Key

| Gutter symbol | Color | Meaning |
|---------------|-------|---------|
| `│` | Green | Line was added (not present in HEAD) |
| `│` | Yellow | Line was modified since HEAD |
| `▾` | Red | A line was deleted at this position |
| `│` | Dim gray | Unchanged |

The indicators are loaded when the file is opened and refreshed whenever the buffer reloads (e.g. in follow mode).

> **Note:** Git indicators require `git` to be on your PATH and the file to be inside a git repository. If `git` is not available or the file is untracked, the gutter shows plain `│` separators.

---

## 15. Diff Mode

Diff mode shows a colorized unified diff between two files in a single pane.

### Usage

```sh
some original.rs --diff modified.rs
some v1/config.toml --diff v2/config.toml
```

The first positional argument is the "old" file; `--diff` specifies the "new" file.

### Display

Diff output is colorized by line type:

| Color | Line type |
|-------|-----------|
| Green | Added line (`+`) |
| Red | Removed line (`-`) |
| Cyan bold | Hunk header (`@@`) |
| Gray | Context line (unchanged) |

```
--- original.rs
+++ modified.rs
@@ -10,7 +10,9 @@
     let config = Config::load()?;
-    let highlighter = SyntaxHighlighter::new(&config.general.theme, enabled);
+    let highlighter = SyntaxHighlighter::new(
+        &config.general.theme,
+        enabled,
+        config.general.themes_dir.as_deref(),
+    );
     let mut app = App::new(buffers, config, highlighter);
```

All normal navigation, search, and mark features work in diff mode. Syntax highlighting is intentionally disabled for diff buffers — the diff colorization takes its place.

---

## 16. Command Mode

Press `:` to enter command mode. The input bar shows `:` followed by what you type. Press `Enter` to execute, `Esc` to cancel.

### Commands

| Command | Action |
|---------|--------|
| `:q` or `:quit` | Quit |
| `:n` or `:next` | Switch to the next file |
| `:p` or `:prev` | Switch to the previous file |
| `:<N>` | Jump to line N (e.g. `:150`) |

---

## 17. Configuration

`some` reads its configuration from `~/.config/some/config.toml`. If the file does not exist, built-in defaults are used. All fields are optional — you only need to include the settings you want to change.

A template with all available options is provided as `config.example.toml` in the source repository.

### Creating a Config File

```sh
mkdir -p ~/.config/some
cp config.example.toml ~/.config/some/config.toml
```

Then edit `~/.config/some/config.toml` with any text editor.

### `[general]` Section

```toml
[general]
# Syntax highlighting theme name
theme = "base16-ocean.dark"

# Show line numbers by default
line_numbers = false

# Wrap long lines by default
wrap = false

# Width used to display tab characters
tab_width = 4

# Enable mouse scroll wheel
mouse = true

# Smart case: case-insensitive search unless the pattern contains uppercase
smart_case = true

# Directory to load additional .tmTheme files from
# Default: ~/.config/some/themes/
# themes_dir = "/path/to/themes"
```

### `[colors]` Section

Colors are specified as hex RGB strings (`"#rrggbb"`).

```toml
[colors]
status_bar_bg      = "#2b303b"
status_bar_fg      = "#c0c5ce"
search_match_bg    = "#ebcb8b"
search_match_fg    = "#2b303b"
line_number_fg     = "#65737e"
```

### CLI Flags Override Config

Any setting controlled by a command-line flag takes precedence over the config file for that invocation. For example, `some -t Dracula file.rs` uses the Dracula theme even if `config.toml` specifies a different one.

---

## 18. Themes

Syntax highlighting colors are controlled by a theme. `some` ships with several built-in options and supports loading your own.

### Built-in Themes

**Bundled presets** (compiled into the binary):

| Name | Style |
|------|-------|
| `Monokai` | Dark, warm — classic editor theme |
| `Dracula` | Dark purple/pink |
| `Nord` | Dark, cool blue tones |
| `Catppuccin-Mocha` | Dark, pastel |

**syntect defaults** (from the syntect library):

| Name |
|------|
| `base16-ocean.dark` *(default)* |
| `base16-eighties.dark` |
| `base16-mocha.dark` |
| `InspiredGitHub` |
| `Solarized (dark)` |
| `Solarized (light)` |

### Selecting a Theme

**Temporarily (command line):**

```sh
some -t Monokai src/main.rs
some -t "Solarized (dark)" notes.md
```

**Permanently (config file):**

```toml
[general]
theme = "Dracula"
```

### Adding Your Own Themes

1. Find a `.tmTheme` file for your preferred theme (many editors and repositories distribute them).
2. Place the file in `~/.config/some/themes/`:

```sh
mkdir -p ~/.config/some/themes
cp MyTheme.tmTheme ~/.config/some/themes/
```

3. Use it by name (without the `.tmTheme` extension):

```sh
some -t MyTheme file.rs
```

Or set it in `config.toml`:

```toml
[general]
theme = "MyTheme"
```

If you want to load themes from a different directory, set `themes_dir` in the `[general]` section:

```toml
[general]
themes_dir = "/home/you/dotfiles/themes"
```

---

## 19. Custom Keybindings

Any normal-mode action can be rebound in the `[keys]` section of your config file. Input-mode keys (search, filter, command) are fixed and cannot be rebound.

### Key Specification Format

| Format | Example | Meaning |
|--------|---------|---------|
| Single character | `"e"` | The `e` key |
| Uppercase character | `"G"` | Shift+G |
| Control combination | `"ctrl+f"` | Ctrl+F |
| Named key | `"space"` | Space bar |
| Named key | `"enter"` | Enter/Return |
| Named key | `"tab"` | Tab |
| Named key | `"pagedown"` or `"pgdn"` | Page Down |
| Named key | `"pageup"` or `"pgup"` | Page Up |
| Named key | `"home"` / `"end"` | Home / End |
| Named key | `"up"` / `"down"` / `"left"` / `"right"` | Arrow keys |

### Bindable Actions

```toml
[keys]
quit            = "q"
scroll_down     = "j"
scroll_up       = "k"
half_page_down  = "ctrl+d"
half_page_up    = "ctrl+u"
full_page_down  = "space"
full_page_up    = "b"
goto_top        = "g"
goto_bottom     = "G"
prev_buffer     = "["
next_buffer     = "]"
search_forward  = "/"
search_backward = "?"
next_match      = "n"
prev_match      = "N"
toggle_numbers  = "l"
toggle_wrap     = "w"
follow_mode     = "F"
enter_command   = ":"
filter          = "&"
visual          = "v"
set_mark        = "m"
jump_mark       = "'"
scroll_right    = "right"
scroll_left     = "left"
```

### Example: `less`-Compatible Bindings

If you are more comfortable with `less`-style keys:

```toml
[keys]
scroll_down    = "e"
scroll_up      = "y"
full_page_down = "f"
full_page_up   = "b"
goto_top       = "g"
goto_bottom    = "G"
next_match     = "n"
prev_match     = "N"
```

### Fixed Aliases

The following bindings are always active regardless of the `[keys]` config and cannot be overridden. They exist so that `some` is usable even with a non-standard keybinding configuration:

- Arrow keys (`↑` `↓` `←` `→`) for scrolling
- `Page Up` / `Page Down`
- `Home` / `End`
- `Enter` for scroll-down
- `Ctrl-C` to quit

---

## 20. Command-Line Reference

```
some [OPTIONS] [FILE]...
```

### Options

| Flag | Short | Description |
|------|-------|-------------|
| `--line-numbers` | `-n` | Show line numbers |
| `--follow` | `-f` | Follow mode — tail the file |
| `--start-line <N>` | `-N` | Open at line N |
| `--pattern <REGEX>` | `-p` | Pre-highlight a search pattern |
| `--wrap` | `-w` | Enable line wrapping |
| `--theme <NAME>` | `-t` | Syntax highlight theme |
| `--no-syntax` | | Disable syntax highlighting |
| `--plain` | | No colors, no line numbers |
| `--tab-width <N>` | | Tab display width (default: 4) |
| `--diff <FILE2>` | | Show unified diff: FILE vs FILE2 |
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |

### Examples

```sh
# View a file with line numbers
some -n src/main.rs

# Jump straight to line 500
some -N 500 large_file.log

# Search for a pattern immediately on open
some -p "panic!" src/lib.rs

# Tail a log file
some -f /var/log/nginx/access.log

# Open multiple Rust files and navigate between them
some src/*.rs

# Disable all formatting (useful for copying text)
some --plain document.txt

# Use a specific theme
some -t Nord src/main.rs

# View a gzipped log
some access.log.gz

# Compare two versions of a file
some old_version.py --diff new_version.py

# Pipe output from another command
cargo build 2>&1 | some
```

---

## 21. Keybinding Reference

### Normal Mode

#### Navigation

| Key | Action |
|-----|--------|
| `j` · `↓` · `Enter` | Scroll down 1 line |
| `k` · `↑` | Scroll up 1 line |
| `d` · `Ctrl-D` | Half page down |
| `u` · `Ctrl-U` | Half page up |
| `Space` · `Page Down` | Full page down |
| `b` · `Page Up` | Full page up |
| `g` · `Home` | Go to top |
| `G` · `End` | Go to bottom |
| `→` | Scroll right 4 columns |
| `←` | Scroll left 4 columns |

#### Search

| Key | Action |
|-----|--------|
| `/` | Enter forward search |
| `?` | Enter backward search |
| `n` | Next match |
| `N` | Previous match |

#### Modes and Features

| Key | Action |
|-----|--------|
| `F` | Enter follow mode |
| `v` | Enter visual selection mode |
| `&` | Enter filter mode |
| `:` | Enter command mode |
| `l` | Toggle line numbers |
| `w` | Toggle line wrap |

#### Marks

| Key | Action |
|-----|--------|
| `m` `<c>` | Set mark `<c>` |
| `'` `<c>` | Jump to mark `<c>` |

#### Buffers

| Key | Action |
|-----|--------|
| `]` | Next file |
| `[` | Previous file |

#### Quit

| Key | Action |
|-----|--------|
| `q` · `Ctrl-C` | Quit |

---

### Search Input Mode

Entered with `/` or `?`.

| Key | Action |
|-----|--------|
| Any character | Append to pattern (preview updates live) |
| `Backspace` | Delete last character |
| `Enter` | Commit search and return to Normal |
| `Esc` | Cancel and return to Normal |

---

### Command Mode

Entered with `:`.

| Key | Action |
|-----|--------|
| Any character | Append to command |
| `Backspace` | Delete last character |
| `Enter` | Execute command |
| `Esc` | Cancel |

**Commands:** `:q` quit · `:n` next file · `:p` prev file · `:<N>` go to line N

---

### Filter Mode

Entered with `&`.

| Key | Action |
|-----|--------|
| Any character | Append to pattern |
| `Backspace` | Delete last character |
| `Enter` | Apply filter |
| `Esc` | Clear filter and return to Normal |

---

### Follow Mode

Entered with `F`.

| Key | Action |
|-----|--------|
| `q` · `Esc` | Return to Normal mode |
| `Ctrl-C` | Quit |

---

### Visual Mode

Entered with `v`.

| Key | Action |
|-----|--------|
| `j` · `↓` | Extend selection down |
| `k` · `↑` | Extend selection up |
| `y` | Yank selection to clipboard |
| `q` · `Esc` | Cancel and return to Normal |

---

*`some` v0.3 — Copyright © 2026 Scott Davis — MIT License*
