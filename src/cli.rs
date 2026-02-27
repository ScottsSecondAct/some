use clap::Parser;
use std::path::PathBuf;

/// some — A jazzed-up terminal file viewer.
/// Like 'less', but with syntax highlighting, line numbers, and more.
#[derive(Parser, Debug, Clone)]
#[command(name = "some", version, about, long_about = None)]
pub struct Cli {
    /// Files to view (reads stdin if none provided)
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Show line numbers
    #[arg(short = 'n', long = "line-numbers")]
    pub line_numbers: bool,

    /// Follow mode — watch file for appended data (like tail -f)
    #[arg(short = 'f', long = "follow")]
    pub follow: bool,

    /// Start at line N
    #[arg(short = 'N', long = "start-line", value_name = "LINE")]
    pub start_line: Option<usize>,

    /// Highlight pattern on open
    #[arg(short = 'p', long = "pattern", value_name = "REGEX")]
    pub pattern: Option<String>,

    /// Enable line wrapping
    #[arg(short = 'w', long = "wrap")]
    pub wrap: bool,

    /// Color theme name
    #[arg(short = 't', long = "theme", default_value = "base16-ocean.dark")]
    pub theme: String,

    /// Disable syntax highlighting
    #[arg(long = "no-syntax")]
    pub no_syntax: bool,

    /// Plain mode — no line numbers, no syntax, no colors
    #[arg(long = "plain")]
    pub plain: bool,

    /// Tab width for display
    #[arg(long = "tab-width", default_value = "4")]
    pub tab_width: u8,
}
