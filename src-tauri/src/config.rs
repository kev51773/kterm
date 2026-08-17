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

fn default_cols() -> u16 {
    120
}
fn default_rows() -> u16 {
    30
}
fn default_profile() -> String {
    "powershell".to_string()
}
fn default_ring_buffer_kb() -> usize {
    256
}
fn default_terminal_padding() -> u32 {
    8
}
fn default_reminder_seconds() -> u32 {
    10
}
fn default_reminder_trigger() -> String {
    "unfocused".to_string()
}
fn default_reminder_audio() -> bool {
    true
}
fn default_reminder_pulse() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_profile")]
    pub default_profile: String,
    #[serde(default = "default_cols")]
    pub default_cols: u16,
    #[serde(default = "default_rows")]
    pub default_rows: u16,
    #[serde(default = "default_ring_buffer_kb")]
    pub ring_buffer_kb: usize,
    #[serde(default = "default_terminal_padding")]
    pub terminal_padding: u32,
    #[serde(default = "default_reminder_seconds")]
    pub reminder_seconds: u32,
    #[serde(default = "default_reminder_trigger")]
    pub reminder_trigger: String,
    #[serde(default = "default_reminder_audio")]
    pub reminder_audio: bool,
    #[serde(default = "default_reminder_pulse")]
    pub reminder_pulse: bool,
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_profile: "powershell".to_string(),
            default_cols: 120,
            default_rows: 30,
            ring_buffer_kb: 256,
            terminal_padding: 8,
            reminder_seconds: 10,
            reminder_trigger: "unfocused".to_string(),
            reminder_audio: true,
            reminder_pulse: true,
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

    pub fn get_default_window_size(&self) -> (f64, f64) {
        let cols = if self.default_cols > 0 { self.default_cols as f64 } else { 120.0 };
        let rows = if self.default_rows > 0 { self.default_rows as f64 } else { 30.0 };
        let cell_w = 8.42;
        let cell_h = 17.0;
        let pad = (self.terminal_padding as f64) * 2.0;
        let w = (cols * cell_w + 16.0 + pad + 0.5).ceil();
        let h = (rows * cell_h + 41.0 + pad + 0.5).ceil();
        (w, h)
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

    #[test]
    fn test_default_font() {
        let font = FontConfig::default();
        assert_eq!(font.size, 14);
        assert!(font.family.contains("Consolas"));
    }

    #[test]
    fn test_default_theme() {
        let theme = ThemeConfig::default();
        assert_eq!(theme.background, "#0d0e11");
        assert_eq!(theme.foreground, "#cccccc");
        assert_eq!(theme.highlight, "#61afef");
    }

    #[test]
    fn test_get_default_window_size() {
        let cfg = AppConfig::default();
        let (w, h) = cfg.get_default_window_size();
        // cols=120, cell_w=8.42, pad=16, so w = 120*8.42 + 16 + 16 = 1042.4 -> 1043
        assert!(w > 1000.0);
        assert!(w < 1100.0);
        // rows=30, cell_h=17, pad=16, so h = 30*17 + 41 + 16 = 567
        assert!(h > 500.0);
        assert!(h < 650.0);
    }

    #[test]
    fn test_get_default_window_size_custom() {
        let cfg = AppConfig {
            default_cols: 80,
            default_rows: 24,
            terminal_padding: 0,
            ..AppConfig::default()
        };
        let (w, h) = cfg.get_default_window_size();
        // 80*8.42 + 16 + 0 = 689.6 -> 690
        assert!(w > 680.0 && w < 700.0);
        // 24*17 + 41 + 0 = 449
        assert!(h > 440.0 && h < 460.0);
    }

    #[test]
    fn test_get_config_path() {
        let path = AppConfig::get_config_path();
        assert!(path.to_string_lossy().contains("kterm"));
        assert!(path.to_string_lossy().ends_with("config.json"));
    }

    #[test]
    fn test_save_and_load() {
        let test_dir = std::env::temp_dir().join("kterm_test_config");
        let _ = std::fs::create_dir_all(&test_dir);
        let test_path = test_dir.join("config.json");

        let cfg = AppConfig {
            default_profile: "wsl".into(),
            default_cols: 100,
            default_rows: 25,
            ..AppConfig::default()
        };

        // Manually write to test path (not using save() since it uses get_config_path)
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        std::fs::write(&test_path, &json).unwrap();

        // Read back
        let content = std::fs::read_to_string(&test_path).unwrap();
        let loaded: AppConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.default_profile, "wsl");
        assert_eq!(loaded.default_cols, 100);
        assert_eq!(loaded.default_rows, 25);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.default_profile, deserialized.default_profile);
        assert_eq!(cfg.default_cols, deserialized.default_cols);
        assert_eq!(cfg.ring_buffer_kb, deserialized.ring_buffer_kb);
    }

    #[test]
    fn test_config_partial_json_fills_defaults() {
        let json = r#"{"default_profile": "cmd"}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.default_profile, "cmd");
        assert_eq!(cfg.default_cols, 120); // default
        assert_eq!(cfg.ring_buffer_kb, 256); // default
    }

    #[test]
    fn test_window_size_zero_cols() {
        let cfg = AppConfig {
            default_cols: 0,
            default_rows: 30,
            ..AppConfig::default()
        };
        let (w, _) = cfg.get_default_window_size();
        // Falls back to 120 cols
        assert!(w > 1000.0);
    }

    #[test]
    fn test_window_size_zero_rows() {
        let cfg = AppConfig {
            default_cols: 120,
            default_rows: 0,
            ..AppConfig::default()
        };
        let (_, h) = cfg.get_default_window_size();
        // Falls back to 30 rows
        assert!(h > 500.0);
    }
}
