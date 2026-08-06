use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: u32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Consolas, 'Courier New', monospace".to_string(),
            size: 14,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub highlight: String,
    #[serde(default = "default_title_bar")]
    pub title_bar: String,
    #[serde(default = "default_active_tab")]
    pub active_tab: String,
    #[serde(default = "default_inactive_tab")]
    pub inactive_tab: String,
    #[serde(default = "default_tab_hover")]
    pub tab_hover: String,
    #[serde(default = "default_active_tab_fg")]
    pub active_tab_fg: String,
    #[serde(default = "default_inactive_tab_fg")]
    pub inactive_tab_fg: String,
}

fn default_title_bar() -> String {
    "#21252b".to_string()
}
fn default_active_tab() -> String {
    "#0d0e11".to_string()
}
fn default_inactive_tab() -> String {
    "#181a1f".to_string()
}
fn default_tab_hover() -> String {
    "#282c34".to_string()
}
fn default_active_tab_fg() -> String {
    "#ffffff".to_string()
}
fn default_inactive_tab_fg() -> String {
    "#abb2bf".to_string()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background: "#0d0e11".to_string(),
            foreground: "#cccccc".to_string(),
            highlight: "#61afef".to_string(),
            title_bar: default_title_bar(),
            active_tab: default_active_tab(),
            inactive_tab: default_inactive_tab(),
            tab_hover: default_tab_hover(),
            active_tab_fg: default_active_tab_fg(),
            inactive_tab_fg: default_inactive_tab_fg(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_profile: String,
    pub ring_buffer_kb: usize,
    pub terminal_padding: u32,
    pub font: FontConfig,
    pub theme: ThemeConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_profile: "powershell".to_string(),
            ring_buffer_kb: 256,
            terminal_padding: 8,
            font: FontConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn get_config_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("kterm").join("config.json")
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            PathBuf::from(home).join(".config").join("kterm").join("config.json")
        } else {
            PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        let cfg = Self::default();
        let _ = cfg.save();
        cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&path, json_str)
            .map_err(|e| format!("Failed to write config file '{}': {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.default_profile, "powershell");
        assert_eq!(cfg.ring_buffer_kb, 256);
        assert_eq!(cfg.terminal_padding, 8);
    }
}
