//! Application settings and configuration

use crate::error::Result;
use serde::{Deserialize, Serialize};
use tui_kit_host::settings::{SettingsError, load_yaml_or_default, save_yaml};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub ui: UiSettings,
    pub analyzer: AnalyzerSettings,
    pub keybindings: KeybindingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub theme: String,
    pub show_line_numbers: bool,
    pub vim_mode: bool,
    pub tab_width: usize,
    pub wrap_text: bool,
    pub accent_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerSettings {
    pub include_private: bool,
    pub include_tests: bool,
    pub max_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingSettings {
    pub quit: String,
    pub search: String,
    pub help: String,
    pub next_tab: String,
    pub prev_tab: String,
    pub select: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui: UiSettings {
                theme: "default".into(),
                show_line_numbers: true,
                vim_mode: false,
                tab_width: 4,
                wrap_text: false,
                accent_color: "#4EBF71".into(),
            },
            analyzer: AnalyzerSettings {
                include_private: true,
                include_tests: false,
                max_depth: 10,
            },
            keybindings: KeybindingSettings {
                quit: "q".into(),
                search: "/".into(),
                help: "?".into(),
                next_tab: "Tab".into(),
                prev_tab: "Shift+Tab".into(),
                select: "Enter".into(),
            },
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        load_yaml_or_default("tui-kit", "rust-inspector.yaml").map_err(map_settings_error)
    }

    pub fn save(&self) -> Result<()> {
        save_yaml("tui-kit", "rust-inspector.yaml", self).map_err(map_settings_error)
    }
}

fn map_settings_error(err: SettingsError) -> crate::error::OracleError {
    match err {
        SettingsError::NoConfigDir => {
            crate::error::OracleError::Config("No config directory".into())
        }
        SettingsError::Io(e) => crate::error::OracleError::Io(e),
        SettingsError::Yaml(e) => crate::error::OracleError::Yaml(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let s = Settings::default();
        assert_eq!(s.ui.theme, "default");
        assert!(s.ui.show_line_numbers);
        assert_eq!(s.keybindings.quit, "q");
        assert_eq!(s.keybindings.search, "/");
        assert!(s.analyzer.include_private);
        assert_eq!(s.analyzer.max_depth, 10);
    }

    #[test]
    fn test_settings_roundtrip_yaml() {
        let s = Settings::default();
        let yaml = serde_yaml::to_string(&s).unwrap();
        let loaded: Settings = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(s.ui.theme, loaded.ui.theme);
        assert_eq!(s.keybindings.quit, loaded.keybindings.quit);
    }
}
