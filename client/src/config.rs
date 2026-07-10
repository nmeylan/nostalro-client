use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

const BASE_FONT_PX_HEIGHT: f32 = 14.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowStateEntry {
    pub position: [f32; 2],
    pub open: bool,
    pub collapsed: bool,
}

impl Default for WindowStateEntry {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            open: false,
            collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayOptions {
    pub show_other_damage: bool,
    pub show_other_cast_bars: bool,
    pub hide_name_player: bool,
    pub hide_name_monster: bool,
    pub hide_name_npc: bool,
    pub show_level_aura: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_other_damage: true,
            show_other_cast_bars: true,
            hide_name_player: false,
            hide_name_monster: false,
            hide_name_npc: false,
            show_level_aura: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub packetver: u32,
    pub login_ip: String,
    pub login_port: u16,
    pub screen_width: u32,
    pub screen_height: u32,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub bgm_enabled: bool,
    pub sfx_enabled: bool,
    pub bgm_path: String,
    pub free_camera: bool,
    pub dpi_scale: f32,
    pub grf_paths: Vec<String>,
    pub enhanced_lag_compensation: bool,
    pub debug_network_delay_ms: u32,
    pub debug_overlay: bool,
    pub trace_packets_send: bool,
    pub trace_packets_recv: bool,
    pub window_state: HashMap<u32, WindowStateEntry>,
    pub hotkey_visible_rows: u8,
    pub battle_mode: bool,
    pub fog: bool,
    pub display: DisplayOptions,
    /// Slot of the character selected last, restored to preselect it (and its page)
    /// on the next character-select screen. Client-side only; the server sends no
    /// "last used" marker.
    pub last_char_slot: Option<u8>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            packetver: 20120307,
            login_ip: "127.0.0.1".to_string(),
            login_port: 6900,
            screen_width: 1024,
            screen_height: 768,
            bgm_volume: 0.8,
            sfx_volume: 0.8,
            bgm_enabled: true,
            sfx_enabled: true,
            bgm_path: "BGM".to_string(),
            free_camera: false,
            dpi_scale: 120.0,
            grf_paths: vec!["data/data.grf".to_string()],
            enhanced_lag_compensation: false,
            debug_network_delay_ms: 0,
            debug_overlay: false,
            trace_packets_send: false,
            trace_packets_recv: false,
            window_state: HashMap::new(),
            hotkey_visible_rows: 1,
            battle_mode: false,
            fog: false,
            display: DisplayOptions::default(),
            last_char_slot: None,
        }
    }
}

impl Config {
    pub fn effective_bgm_volume(&self) -> f32 {
        if self.bgm_enabled { self.bgm_volume } else { 0.0 }
    }

    pub fn effective_sfx_volume(&self) -> f32 {
        if self.sfx_enabled { self.sfx_volume } else { 0.0 }
    }

    pub fn font_px_height(&self) -> f32 {
        BASE_FONT_PX_HEIGHT
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn load_or_default(path: &str) -> Self {
        let path = Path::new(path);
        if path.exists() {
            let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
                tracing::warn!("Failed to read config: {e}, using defaults");
                String::new()
            });
            serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse config: {e}, using defaults");
                Config::default()
            })
        } else {
            let config = Config::default();
            if let Ok(json) = serde_json::to_string_pretty(&config) {
                let _ = std::fs::write(path, json);
            }
            config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.packetver, 20120307);
        assert_eq!(parsed.login_port, 6900);
        assert_eq!(parsed.screen_width, 1024);
        assert_eq!(parsed.grf_paths, vec!["data/data.grf"]);
    }

    #[test]
    fn window_state_roundtrips() {
        let mut config = Config::default();
        config.window_state.insert(
            800,
            WindowStateEntry {
                position: [100.0, 200.0],
                open: true,
                collapsed: false,
            },
        );
        config.window_state.insert(
            900,
            WindowStateEntry {
                position: [50.0, 60.0],
                open: false,
                collapsed: true,
            },
        );
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        let inv = parsed.window_state.get(&800).unwrap();
        assert_eq!(inv.position, [100.0, 200.0]);
        assert!(inv.open);
        assert!(!inv.collapsed);
        let eq = parsed.window_state.get(&900).unwrap();
        assert!(!eq.open);
        assert!(eq.collapsed);
    }

    #[test]
    fn partial_json_uses_defaults() {
        let json = r#"{"packetver": 20200401}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.packetver, 20200401);
        assert_eq!(config.login_port, 6900);
        assert_eq!(config.screen_width, 1024);
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let config = Config::load_or_default("/tmp/nonexistent_ragnarok_config.json");
        assert_eq!(config.packetver, 20120307);
    }
}
