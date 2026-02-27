use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration, loaded from ~/.config/some/config.toml
/// with CLI flags taking precedence.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub colors: ColorConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub theme: String,
    pub line_numbers: bool,
    pub wrap: bool,
    pub tab_width: u8,
    pub mouse: bool,
    pub smart_case: bool,
    /// Bytes threshold above which mmap is used
    pub mmap_threshold: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    pub status_bar_fg: String,
    pub status_bar_bg: String,
    pub search_match_fg: String,
    pub search_match_bg: String,
    pub line_number_fg: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            colors: ColorConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: "base16-ocean.dark".to_string(),
            line_numbers: false,
            wrap: false,
            tab_width: 4,
            mouse: true,
            smart_case: true,
            mmap_threshold: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            status_bar_fg: "#cdd6f4".to_string(),
            status_bar_bg: "#1e1e2e".to_string(),
            search_match_fg: "#1e1e2e".to_string(),
            search_match_bg: "#f9e2af".to_string(),
            line_number_fg: "#6c7086".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        Ok(Config::default())
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("some").join("config.toml"))
    }

    pub fn merge_cli(&mut self, cli: &crate::cli::Cli) {
        if cli.line_numbers {
            self.general.line_numbers = true;
        }
        if cli.wrap {
            self.general.wrap = true;
        }
        if cli.plain {
            self.general.line_numbers = false;
        }
        if cli.tab_width != 4 {
            self.general.tab_width = cli.tab_width;
        }
        if cli.theme != "base16-ocean.dark" {
            self.general.theme = cli.theme.clone();
        }
    }
}
