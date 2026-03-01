use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub free_camera: bool,
    pub grf_paths: Vec<String>,
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
            free_camera: false,
            grf_paths: vec!["data/data.grf".to_string()],
        }
    }
}

impl Config {
    pub fn load_or_default(path: &str) -> Self {
        let path = Path::new(path);
        if path.exists() {
            let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
                tracing::warn!("Failed to read config: {e}, using defaults");
                return String::new();
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
