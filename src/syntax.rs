use ratatui::style::{Color, Style};
use std::path::Path;
use syntect::highlighting::{ThemeSet, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::easy::HighlightLines;

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
    pub fn new(theme_name: &str, enabled: bool) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .unwrap_or_else(|| {
                // Fall back to base16-ocean.dark
                theme_set.themes["base16-ocean.dark"].clone()
            });

        Self {
            syntax_set,
            theme,
            enabled,
        }
    }

    /// Detect the syntax for a file path, falling back to plain text.
    pub fn detect_syntax(&self, path: Option<&Path>) -> &SyntaxReference {
        if let Some(path) = path {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if let Some(syntax) = self.syntax_set.find_syntax_by_extension(ext) {
                    return syntax;
                }
            }
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
