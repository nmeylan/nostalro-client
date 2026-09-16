use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

const BASE_FONT_PX_HEIGHT: f32 = 15.0;

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

pub use ragnarok_game::cursor::MouseSnapPrefs;
pub use ragnarok_game::display::DisplayOptions;
pub use ragnarok_game::keybinding::{EmotionKeys, KeyBindings};
pub use ragnarok_profiling::debug::PacketTrace;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugConfig {
    pub trace_packet: PacketTrace,
    pub trace_effects: bool,
    pub trace_input: bool,
    pub trace_texture_load: bool,
    /// Logs the sprite magnification measured on map entry, along with the
    /// upscale factor derived from it.
    pub trace_sprite_scale: bool,
}

/// Behaviour the original game has no counterpart for, off unless opted into.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomConfig {
    /// Green aura under boss monsters at level 99 or above.
    pub boss_aura: bool,
    /// Base level a player's aura is drawn from. The original game hard-codes
    /// 99; raise it on a server whose reported levels go past that.
    pub aura_level: i16,
    /// Multiplies both fog distances. The original game's are tuned for its own
    /// window and camera range; raise this to push fog back on a screen that
    /// shows more of the map, lower it to pull fog in. 1.0 is the original.
    pub fog_scale: f32,
    /// Deprecated alias of `accessibility.bold_name_plates`, folded into it at
    /// load time and never written back.
    #[serde(skip_serializing)]
    pub accessibility: bool,
    /// Replay walks against the server clock the way the original game does:
    /// read 72 ms behind the estimate and corrected in 144 ms steps, so the
    /// walk cycle stalls and jumps under lag. Off keeps the smooth local walk.
    pub latency_coupled_walk: bool,
    pub filtering: CustomFilteringConfig,
    pub sound: CustomSoundConfig,
    pub window: CustomWindowConfig,
    pub skill: CustomSkillConfig,
}

impl Default for CustomConfig {
    fn default() -> Self {
        Self {
            boss_aura: false,
            aura_level: ragnarok_game::level_aura::LEVEL_AURA_THRESHOLD,
            fog_scale: 1.0,
            accessibility: false,
            latency_coupled_walk: false,
            filtering: CustomFilteringConfig::default(),
            sound: CustomSoundConfig::default(),
            window: CustomWindowConfig::default(),
            skill: CustomSkillConfig::default(),
        }
    }
}

/// Texture filtering per family. The original game filters all three, so `false`
/// is the deviation here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomFilteringConfig {
    /// Filter ground and model textures, over a mip chain. Off point-samples them.
    pub world: bool,
    /// Filter effect textures, both the STR ones and the primitive ones.
    pub effects: bool,
    /// Filter entity sprites. Off point-samples them, which keeps every texel
    /// hard and drops the dark rim filtering leaves on a silhouette.
    pub sprites: bool,
    /// Upload entity sprites enlarged, so filtering only softens a fraction of a
    /// source texel. The factor is derived from the camera on map entry and
    /// capped at 4; memory grows with its square. Ignored while `sprites` is off.
    pub sprite_upscale: bool,
}

impl Default for CustomFilteringConfig {
    fn default() -> Self {
        Self {
            world: true,
            effects: true,
            sprites: true,
            sprite_upscale: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomSkillConfig {
    pub al_teleport: CustomAlTeleportConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomAlTeleportConfig {
    /// Give the skill a level picker in the skill tree. The original game casts
    /// it at the learned level, with no way to pick a lower one.
    pub separate_lvl: bool,
    /// Answer the server's one-entry warp list without showing it, so a level 1
    /// cast warps straight away.
    pub skip_lvl1_menu: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomWindowConfig {
    /// Windows Escape must leave alone, by the names in
    /// `ui::escape::ESC_WINDOW_NAMES` (case- and space-insensitive). Escape then
    /// moves on to the next window behind them.
    pub exclude_close_via_esc: Vec<String>,
    /// Wrap a long item name inside the item information window, pushing the
    /// description down. The original game keeps it on one line and lets it run
    /// past the right edge.
    pub wrap_item_info_title: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomSoundConfig {
    /// Percentage of ACT frame sounds (monster grunts, footsteps) that play.
    pub act_percent: u32,
    /// Pan world sounds across the stereo field. Off keeps distance attenuation
    /// but centres everything.
    pub stereo: bool,
    /// Keep the mixer running while the window is not focused. The original
    /// game always pauses.
    pub play_when_unfocused: bool,
}

impl Default for CustomSoundConfig {
    fn default() -> Self {
        Self {
            act_percent: 100,
            stereo: true,
            play_when_unfocused: false,
        }
    }
}

/// RGBA colour written in config as `"#RRGGBB"` or `"#RRGGBBAA"`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexColor([f32; 4]);

impl HexColor {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        ])
    }

    pub fn rgba(&self) -> [f32; 4] {
        self.0
    }
}

impl std::str::FromStr for HexColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if !hex.is_ascii() || (hex.len() != 6 && hex.len() != 8) {
            return Err(format!("expected #RRGGBB or #RRGGBBAA, got {s:?}"));
        }
        let byte = |i: usize| {
            u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| format!("invalid hex color {s:?}"))
        };
        let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
        Ok(Self([
            byte(0)? as f32 / 255.0,
            byte(2)? as f32 / 255.0,
            byte(4)? as f32 / 255.0,
            alpha as f32 / 255.0,
        ]))
    }
}

impl std::fmt::Display for HexColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b, a] = self.0.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        if a == 255 {
            write!(f, "#{r:02X}{g:02X}{b:02X}")
        } else {
            write!(f, "#{r:02X}{g:02X}{b:02X}{a:02X}")
        }
    }
}

impl Serialize for HexColor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

const BAR_BACKGROUND_COLOR: HexColor = HexColor::rgb(0x42, 0x42, 0x42);
const BAR_BORDER_COLOR: HexColor = HexColor::rgb(0x10, 0x18, 0x9C);

/// Geometry and frame colours of one overhead bar. The fill colour is not here:
/// HP takes it from the ratio and the entity type, SP and cast are fixed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarStyle {
    pub width: f32,
    pub height: f32,
    pub background_color: HexColor,
    pub border_color: HexColor,
    pub fill_color: HexColor,
    /// Fill below a quarter of the bar. The SP and cast defaults repeat
    /// `fill_color`, so only the HP bars change colour as they drain.
    pub fill_color_low: HexColor,
}

impl Default for BarStyle {
    fn default() -> Self {
        Self {
            width: 60.0,
            height: 5.0,
            background_color: BAR_BACKGROUND_COLOR,
            border_color: BAR_BORDER_COLOR,
            fill_color: HexColor::rgb(0x10, 0xEF, 0x21),
            fill_color_low: HexColor::rgb(0xFF, 0x00, 0x00),
        }
    }
}

impl BarStyle {
    fn monster_hp() -> Self {
        Self {
            fill_color: HexColor::rgb(0xFF, 0x00, 0xE7),
            fill_color_low: HexColor::rgb(0xFF, 0xFF, 0x00),
            ..Self::default()
        }
    }

    fn sp() -> Self {
        let fill = HexColor::rgb(0x18, 0x63, 0xDE);
        Self {
            fill_color: fill,
            fill_color_low: fill,
            ..Self::default()
        }
    }

    fn cast() -> Self {
        let fill = HexColor::rgb(0x00, 0xCC, 0x00);
        Self {
            height: 7.0,
            fill_color: fill,
            fill_color_low: fill,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilityConfig {
    /// Draw name plates, floor-item labels and the pending-skill level in a bold
    /// weight with a heavier outline. The original game has one weight only.
    pub bold_name_plates: bool,
    pub hp_bar: BarStyle,
    pub monster_hp_bar: BarStyle,
    pub sp_bar: BarStyle,
    pub cast_bar: BarStyle,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            bold_name_plates: false,
            hp_bar: BarStyle::default(),
            monster_hp_bar: BarStyle::monster_hp(),
            sp_bar: BarStyle::sp(),
            cast_bar: BarStyle::cast(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginServer {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub packetver: u32,
}

impl Default for LoginServer {
    fn default() -> Self {
        Self {
            name: "Local".to_string(),
            host: "127.0.0.1".to_string(),
            port: 6900,
            packetver: 20111102,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Selectable connection (login) servers. The first is used by default; when
    /// more than one is present the client shows a selection screen before login.
    pub login_servers: Vec<LoginServer>,
    /// When true, the last login ID is stored in `saved_username` and pre-filled
    /// on the login screen (never the password).
    pub keep_login_id: bool,
    pub saved_username: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub bgm_enabled: bool,
    pub sfx_enabled: bool,
    pub bgm_path: String,
    pub emblem_path: String,
    /// Screenshot escape hatch: drops the pitch band, the indoor rotation clamp
    /// and the zoom clamp. Off keeps the original game's bands.
    pub free_camera: bool,
    pub dpi_scale: f32,
    pub grf_paths: Vec<String>,
    /// Optional directory of files extracted from a GRF. Its contents mirror the
    /// inside of the archive's `data/` folder (e.g. `sprite/…`, `texture/…`) and
    /// take priority over every entry in `grf_paths`.
    pub data_dir: Option<String>,
    pub debug_network_delay_ms: u32,
    pub debug: DebugConfig,
    pub window_state: HashMap<u32, WindowStateEntry>,
    pub hotkey_visible_rows: u8,
    pub battle_mode: bool,
    pub fog: bool,
    pub fullscreen: bool,
    /// The `/effect` flag: when false, one-shot skill/attack/item effects are
    /// dropped (keyed persistent visuals like auras stay).
    pub show_skill_effects: bool,
    pub refuse_trade: bool,
    pub refuse_party_invite: bool,
    /// NPC shop "Toggle Item Amount." checkbox: when set, dragging a stack in or
    /// out of the sell basket moves all of it instead of asking for a count.
    pub no_item_amount_question: bool,
    pub display: DisplayOptions,
    #[serde(default)]
    pub snap: MouseSnapPrefs,
    /// Slot of the character selected last, restored to preselect it (and its page)
    /// on the next character-select screen. Client-side only; the server sends no
    /// "last used" marker.
    pub last_char_slot: Option<u8>,
    /// Command sent (as chat) by the map-recovery window's warp button when a map
    /// cannot be loaded because its data is missing from the GRF.
    pub map_recovery_command: String,
    /// Chat commands bound to Alt+1..Alt+0 by the Shortcut List window (10 slots).
    pub shortcut_commands: Vec<String>,
    #[serde(default = "KeyBindings::defaults")]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub emotion_keys: EmotionKeys,
    /// GRF texture paths for the account-screen background. One is picked (at
    /// random) per session and stretched behind the login/server/character
    /// screens. Empty or all-missing falls back to the solid clear color.
    #[serde(default = "default_account_backgrounds")]
    pub account_backgrounds: Vec<String>,
    /// Account ids treated as GM: their characters use the Operator body sprite
    /// and render their name, guild name and chat in yellow.
    #[serde(default)]
    pub admin_account_ids: Vec<u32>,
    /// When the local player is a GM (its account id is in `admin_account_ids`),
    /// also render it as a GM to itself. Has no effect for non-GM accounts.
    #[serde(default)]
    pub see_self_as_gm_when_gm: bool,
    #[serde(default)]
    pub custom: CustomConfig,
    #[serde(default)]
    pub accessibility: AccessibilityConfig,
}


fn default_account_backgrounds() -> Vec<String> {
    vec![
        ragnarok_resources::ui::RAG_TITLE.to_string(),
        ragnarok_resources::ui::RAG_TITLE2.to_string(),
        ragnarok_resources::ui::RAG_TITLE3.to_string(),
    ]
}

fn default_map_recovery_command() -> String {
    "@go prontera".to_string()
}

fn default_shortcut_commands() -> Vec<String> {
    ragnarok_game::emotion::default_shortcut_commands()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            login_servers: vec![LoginServer::default()],
            keep_login_id: false,
            saved_username: String::new(),
            screen_width: 1024,
            screen_height: 768,
            bgm_volume: 0.8,
            sfx_volume: 0.8,
            bgm_enabled: true,
            sfx_enabled: true,
            bgm_path: "BGM".to_string(),
            emblem_path: "emblem".to_string(),
            free_camera: false,
            dpi_scale: 125.0,
            grf_paths: vec![ragnarok_resources::grf::DEFAULT_ARCHIVE.to_string()],
            data_dir: None,
            debug_network_delay_ms: 0,
            debug: DebugConfig::default(),
            window_state: HashMap::new(),
            hotkey_visible_rows: 1,
            battle_mode: false,
            fog: true,
            fullscreen: false,
            show_skill_effects: true,
            refuse_trade: false,
            no_item_amount_question: false,
            refuse_party_invite: false,
            display: DisplayOptions::default(),
            snap: MouseSnapPrefs::default(),
            last_char_slot: None,
            map_recovery_command: default_map_recovery_command(),
            shortcut_commands: default_shortcut_commands(),
            keybindings: KeyBindings::defaults(),
            emotion_keys: EmotionKeys::default(),
            account_backgrounds: default_account_backgrounds(),
            admin_account_ids: Vec::new(),
            see_self_as_gm_when_gm: false,
            custom: CustomConfig::default(),
            accessibility: AccessibilityConfig::default(),
        }
    }
}

impl Config {
    pub fn effective_bgm_volume(&self) -> f32 {
        if self.bgm_enabled {
            self.bgm_volume
        } else {
            0.0
        }
    }

    pub fn effective_sfx_volume(&self) -> f32 {
        if self.sfx_enabled {
            self.sfx_volume
        } else {
            0.0
        }
    }

    pub fn font_px_height(&self) -> f32 {
        BASE_FONT_PX_HEIGHT
    }

    pub fn is_gm_account(&self, account_id: u32) -> bool {
        self.admin_account_ids.contains(&account_id)
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
            let mut config: Config = serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse config: {e}, using defaults");
                Config::default()
            });
            config.keybindings.fill_missing_from_defaults();
            if config.custom.accessibility {
                config.accessibility.bold_name_plates = true;
            }
            config
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
        assert_eq!(parsed.login_servers.len(), 1);
        assert_eq!(parsed.login_servers[0].host, "127.0.0.1");
        assert_eq!(parsed.login_servers[0].port, 6900);
        assert_eq!(parsed.login_servers[0].packetver, 20111102);
        assert_eq!(parsed.screen_width, 1024);
        assert_eq!(parsed.grf_paths, vec!["data/data.grf"]);
        assert_eq!(parsed.account_backgrounds.len(), 3);
    }

    #[test]
    fn per_server_packetver_parses_and_defaults_to_none() {
        let json = r#"{"login_servers":[
            {"name":"Live","host":"live.example.com","port":6900, "packetver": 20120307},
            {"name":"Old","host":"10.0.0.1","port":6901,"packetver":20040101}
        ]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.login_servers.len(), 2);
        assert_eq!(config.login_servers[1].packetver, 20040101);
        assert_eq!(config.login_servers[1].host, "10.0.0.1");
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
    fn keybinding_remap_roundtrips_and_missing_filled() {
        use ragnarok_game::keybinding::{HotkeyAction, KeyChord};
        let mut config = Config::default();
        config.keybindings.set(
            HotkeyAction::ToggleInventory,
            KeyChord::new("KeyB", true, false, false),
        );
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.keybindings.get(HotkeyAction::ToggleInventory),
            Some(&KeyChord::new("KeyB", true, false, false))
        );

        let mut sparse: Config = serde_json::from_str(r#"{"keybindings": {}}"#).unwrap();
        assert!(sparse.keybindings.get(HotkeyAction::SitStand).is_none());
        sparse.keybindings.fill_missing_from_defaults();
        assert_eq!(
            sparse.keybindings.get(HotkeyAction::SitStand),
            Some(&KeyChord::new("Insert", false, false, false))
        );
    }

    #[test]
    fn partial_json_uses_defaults() {
        let json = r#"{"packetver": 20200401}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.login_servers.len(), 1);
        assert_eq!(config.login_servers[0].port, 6900);
        assert_eq!(config.screen_width, 1024);
        assert!(config.custom.window.exclude_close_via_esc.is_empty());
        assert!(!config.custom.window.wrap_item_info_title);
        assert!(!config.custom.skill.al_teleport.separate_lvl);

        let json = r#"{"custom": {"window": {"exclude_close_via_esc": ["Stats", "Inventory"]}}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.custom.window.exclude_close_via_esc,
            vec!["Stats".to_string(), "Inventory".to_string()]
        );
        assert!(!config.custom.skill.al_teleport.separate_lvl);

        let json = r#"{"custom": {"skill": {"al_teleport": {"separate_lvl": true}}}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.custom.skill.al_teleport.separate_lvl);
        assert!(!config.custom.skill.al_teleport.skip_lvl1_menu);
        assert!(!config.custom.boss_aura);
        assert!(config.custom.filtering.world);
        assert!(config.custom.filtering.effects);
        assert!(config.custom.filtering.sprites);
        assert!(!config.custom.filtering.sprite_upscale);
        assert_eq!(config.custom.fog_scale, 1.0);

        let json = r#"{"custom": {"fog_scale": 2.5}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.custom.fog_scale, 2.5);
    }

    #[test]
    fn debug_section_parses_and_legacy_trace_keys_ignored() {
        let json = r#"{
            "trace_packets_send": true,
            "trace_packets_recv": true,
            "debug": {"trace_packet": "unhandled", "trace_input": true}
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.debug.trace_packet, PacketTrace::Unhandled);
        assert!(config.debug.trace_input);
        assert!(!config.debug.trace_effects);
        assert!(!config.debug.trace_texture_load);
        assert!(!config.debug.trace_sprite_scale);

        let reparsed: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(reparsed.debug.trace_packet, PacketTrace::Unhandled);
    }

    #[test]
    fn missing_debug_section_defaults_to_none() {
        let config: Config = serde_json::from_str(r#"{"packetver": 20120307}"#).unwrap();
        assert_eq!(config.debug.trace_packet, PacketTrace::None);
    }

    #[test]
    fn accessibility_bars_default_to_todays_look_and_parse_hex() {
        let config = Config::default();
        assert_eq!(config.accessibility.hp_bar.width, 60.0);
        assert_eq!(config.accessibility.hp_bar.height, 5.0);
        assert_eq!(config.accessibility.sp_bar.height, 5.0);
        assert_eq!(config.accessibility.cast_bar.height, 7.0);
        assert_eq!(
            config.accessibility.cast_bar.border_color.rgba(),
            [16.0 / 255.0, 24.0 / 255.0, 156.0 / 255.0, 1.0]
        );
        assert_eq!(
            config.accessibility.monster_hp_bar.fill_color.rgba(),
            [1.0, 0.0, 231.0 / 255.0, 1.0]
        );
        assert_eq!(
            config.accessibility.monster_hp_bar.fill_color_low.rgba(),
            [1.0, 1.0, 0.0, 1.0]
        );
        assert_eq!(
            config.accessibility.cast_bar.fill_color_low,
            config.accessibility.cast_bar.fill_color
        );

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(r##""border_color":"#10189C""##), "{json}");
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.accessibility.hp_bar.background_color,
            config.accessibility.hp_bar.background_color
        );

        let json = r##"{"accessibility": {"hp_bar": {"width": 90.0, "height": 8.0,
            "background_color": "#1A1A1A", "border_color": "#FFCC0080"}}}"##;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.accessibility.hp_bar.width, 90.0);
        assert_eq!(
            config.accessibility.hp_bar.border_color.rgba(),
            [1.0, 204.0 / 255.0, 0.0, 128.0 / 255.0]
        );
        assert_eq!(config.accessibility.sp_bar.height, 5.0);
        assert_eq!(config.accessibility.cast_bar.height, 7.0);
        assert_eq!(
            config.accessibility.monster_hp_bar.fill_color,
            BarStyle::monster_hp().fill_color
        );
        assert!(!config.accessibility.bold_name_plates);

        assert!("#GGG000".parse::<HexColor>().is_err());
        assert!(serde_json::from_str::<Config>(r#"{"accessibility": {"hp_bar": {"border_color": "red"}}}"#).is_err());
    }

    #[test]
    fn deprecated_custom_accessibility_folds_into_bold_name_plates() {
        let path = std::env::temp_dir().join("ragnarok_legacy_accessibility.json");
        std::fs::write(&path, r#"{"custom": {"accessibility": true}}"#).unwrap();
        let config = Config::load_or_default(path.to_str().unwrap());
        assert!(config.accessibility.bold_name_plates);
        assert!(!serde_json::to_string(&config).unwrap().contains(r#""accessibility":true"#));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let config = Config::load_or_default("/tmp/nonexistent_ragnarok_config.json");
    }

    #[test]
    fn sample_config_parses() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../config.sample.json");
        let config: Config = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

        assert_eq!(config.login_servers.len(), 1);
        assert_eq!(config.login_servers[0].packetver, 20111102);
        assert_eq!(config.grf_paths, vec!["data/data.grf".to_string()]);
        assert!(config.saved_username.is_empty());
        assert!(config.admin_account_ids.is_empty());
        assert!(config.window_state.is_empty());
    }
}
