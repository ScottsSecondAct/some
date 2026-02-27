use ratatui::style::{Color, Style};
use std::path::Path;
use syntect::highlighting::{ThemeSet, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::easy::HighlightLines;

const BUNDLED_THEMES: &[(&str, &[u8])] = &[
    ("Monokai",          include_bytes!("../assets/themes/Monokai.tmTheme")),
    ("Dracula",          include_bytes!("../assets/themes/Dracula.tmTheme")),
    ("Nord",             include_bytes!("../assets/themes/Nord.tmTheme")),
    ("Catppuccin-Mocha", include_bytes!("../assets/themes/Catppuccin-Mocha.tmTheme")),
];

/// Manages syntax highlighting using syntect.
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
    enabled: bool,
}

/// A styled span of text for rendering.
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

impl SyntaxHighlighter {
    pub fn new(theme_name: &str, enabled: bool, themes_dir: Option<&Path>) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let mut theme_set = ThemeSet::load_defaults();

        // Load bundled themes
        for (name, bytes) in BUNDLED_THEMES {
            let mut cursor = std::io::Cursor::new(*bytes);
            if let Ok(theme) = ThemeSet::load_from_reader(&mut cursor) {
                theme_set.themes.insert(name.to_string(), theme);
            }
        }

        // Load user themes (override bundled themes with same name)
        let user_dir = themes_dir
            .map(|p| p.to_path_buf())
            .or_else(|| dirs::config_dir().map(|d| d.join("some").join("themes")));

        if let Some(dir) = user_dir {
            if dir.exists() {
                if let Ok(extra) = ThemeSet::load_from_folder(&dir) {
                    for (name, theme) in extra.themes {
                        theme_set.themes.insert(name, theme);
                    }
                }
            }
        }

        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .unwrap_or_else(|| {
                theme_set.themes["base16-ocean.dark"].clone()
            });

        Self {
            syntax_set,
            theme,
            enabled,
        }
    }

    /// Detect the syntax for a file path, falling back to plain text.
    /// Strips compression extensions (.gz/.zst/.bz2) to detect inner syntax.
    pub fn detect_syntax(&self, path: Option<&Path>) -> &SyntaxReference {
        if let Some(path) = path {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            // Try direct extension match first
            if let Some(syntax) = self.syntax_set.find_syntax_by_extension(ext) {
                return syntax;
            }

            // Strip compression extensions and retry with inner extension
            if matches!(ext, "gz" | "zst" | "zstd" | "bz2") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if let Some(inner_ext) = Path::new(stem).extension().and_then(|e| e.to_str()) {
                    if let Some(syntax) = self.syntax_set.find_syntax_by_extension(inner_ext) {
                        return syntax;
                    }
                }
            }

            // Try by filename
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(syntax) = self.syntax_set.find_syntax_by_extension(name) {
                    return syntax;
                }
            }
        }
        self.syntax_set.find_syntax_plain_text()
    }

    /// Highlight a single line, returning a list of styled spans.
    /// If syntax highlighting is disabled, returns the line as a single unstyled span.
    pub fn highlight_line(
        &self,
        line: &str,
        highlighter: &mut HighlightLines,
    ) -> Vec<StyledSpan> {
        if !self.enabled {
            return vec![StyledSpan {
                text: line.to_string(),
                style: Style::default(),
            }];
        }

        match highlighter.highlight_line(line, &self.syntax_set) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text)| StyledSpan {
                    text: text.to_string(),
                    style: syntect_to_ratatui_style(&style),
                })
                .collect(),
            Err(_) => vec![StyledSpan {
                text: line.to_string(),
                style: Style::default(),
            }],
        }
    }

    /// Create a new highlighter instance for a given syntax.
    pub fn create_highlight_lines<'a>(&'a self, syntax: &'a SyntaxReference) -> HighlightLines<'a> {
        HighlightLines::new(syntax, &self.theme)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Convert a syntect style to a ratatui style.
fn syntect_to_ratatui_style(style: &syntect::highlighting::Style) -> Style {
    let fg = Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    );
    Style::default().fg(fg)
}
