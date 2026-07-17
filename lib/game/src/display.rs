use serde::{Deserialize, Serialize};

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
