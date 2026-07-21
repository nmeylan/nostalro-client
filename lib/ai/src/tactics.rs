use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::consts::{
    BasicTactic, CastTactic, ChaseTactic, KsTactic, KiteTactic, PushbackTactic, RescueTactic,
    SkillClass, SnipeTactic,
};

/// Skill-use column: `0` never, `100` always, `N>0` up to N casts, `N<0` a
/// single cast at level `-N`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "i32", into = "i32")]
pub enum SkillUse {
    Never,
    Always,
    Times(u8),
    OnceAtLevel(u8),
}

impl From<i32> for SkillUse {
    fn from(v: i32) -> Self {
        match v {
            0 => SkillUse::Never,
            100 => SkillUse::Always,
            n if n > 0 => SkillUse::Times(n as u8),
            n => SkillUse::OnceAtLevel((-n) as u8),
        }
    }
}

impl From<SkillUse> for i32 {
    fn from(v: SkillUse) -> i32 {
        match v {
            SkillUse::Never => 0,
            SkillUse::Always => 100,
            SkillUse::Times(n) => n as i32,
            SkillUse::OnceAtLevel(l) => -(l as i32),
        }
    }
}

impl Default for SkillUse {
    fn default() -> Self {
        SkillUse::Never
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tactic {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    pub basic: BasicTactic,
    pub skill: SkillUse,
    pub kite: KiteTactic,
    pub cast: CastTactic,
    pub pushback: PushbackTactic,
    /// Skill id, or a negative debuff-status code.
    pub debuff: i32,
    pub skill_class: SkillClass,
    pub rescue: RescueTactic,
    /// SP reserve; `-1` resolves to `AttackSkillReserveSP` at read time.
    pub sp: i32,
    pub snipe: SnipeTactic,
    pub ks: KsTactic,
    pub weight: f32,
    pub chase: ChaseTactic,
}

impl Tactic {
    pub fn default_row() -> Self {
        Tactic {
            id: 0,
            name: "Default".to_string(),
            basic: BasicTactic::AttackMed,
            skill: SkillUse::Always,
            kite: KiteTactic::React,
            cast: CastTactic::React,
            pushback: PushbackTactic::Never,
            debuff: 0,
            skill_class: SkillClass::Both,
            rescue: RescueTactic::Never,
            sp: -1,
            snipe: SnipeTactic::Ok,
            ks: KsTactic::Never,
            weight: 1.0,
            chase: ChaseTactic::Normal,
        }
    }

    pub fn treasure_row() -> Self {
        Tactic {
            id: 13,
            name: "Treasure Box".to_string(),
            basic: BasicTactic::ReactLow,
            skill: SkillUse::Never,
            kite: KiteTactic::Never,
            weight: 0.0,
            ..Tactic::default_row()
        }
    }
}

/// PVP row (8 columns), keyed by player id or friend class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PvpTactic {
    pub key: i32,
    pub basic: BasicTactic,
    pub skill: SkillUse,
    pub kite: KiteTactic,
    pub cast: CastTactic,
    pub pushback: PushbackTactic,
    pub debuff: i32,
    pub skill_class: SkillClass,
    pub rescue: RescueTactic,
}

/// Per-class tactic lookup with the reference fallback chain
/// (per-mob → treasure-chest → default).
pub struct TacticTable {
    rows: HashMap<u32, Tactic>,
    default: Tactic,
    treasure: Tactic,
}

impl TacticTable {
    pub fn from_rows(rows: &[Tactic]) -> Self {
        let map: HashMap<u32, Tactic> = rows.iter().map(|t| (t.id, t.clone())).collect();
        let default = map.get(&0).cloned().unwrap_or_else(Tactic::default_row);
        let treasure = map.get(&13).cloned().unwrap_or_else(Tactic::treasure_row);
        TacticTable { rows: map, default, treasure }
    }

    pub fn resolve(&self, class_id: u32) -> &Tactic {
        if let Some(t) = self.rows.get(&class_id) {
            return t;
        }
        if is_treasure_class(class_id) {
            return &self.treasure;
        }
        &self.default
    }
}

impl Default for TacticTable {
    fn default() -> Self {
        TacticTable::from_rows(&default_tactics())
    }
}

fn is_treasure_class(class_id: u32) -> bool {
    (1324..=1363).contains(&class_id) || (1938..=1946).contains(&class_id)
}

/// Builds a per-mob row. Every ported row shares the same cast/pushback/debuff/
/// rescue/sp/snipe/ks columns, so only the varying ones are passed.
fn tac(
    id: u32,
    name: &str,
    basic: i32,
    skill: i32,
    kite: i32,
    class: i32,
    weight: f32,
    chase: i32,
) -> Tactic {
    Tactic {
        id,
        name: name.to_string(),
        basic: basic.into(),
        skill: skill.into(),
        kite: kite.into(),
        cast: CastTactic::React,
        pushback: PushbackTactic::Never,
        debuff: 0,
        skill_class: class.into(),
        rescue: RescueTactic::Never,
        sp: -1,
        snipe: SnipeTactic::Ok,
        ks: KsTactic::Never,
        weight,
        chase: chase.into(),
    }
}

/// Homunculus per-mob default tactics, ported from the reference tactic table.
pub fn default_tactics() -> Vec<Tactic> {
    vec![
        tac(0, "Default", 3, 100, 1, -1, 1.0, -1),
        tac(13, "Treasure Box", 5, 0, 0, -1, 0.0, -1),
        tac(1005, "Familiar", 2, 0, 0, -1, 0.5, -1),
        tac(1042, "Steel Chonchon", 2, -1, 1, 0, 1.0, -1),
        tac(1078, "Red Plant", 5, 0, 0, -1, 0.0, -1),
        tac(1079, "Blue Plant", 5, 0, 0, -1, 0.0, -1),
        tac(1080, "Green Plant", 5, 0, 0, -1, 0.0, -1),
        tac(1081, "Yellow Plant", 5, 0, 0, -1, 0.0, -1),
        tac(1082, "White Plant", 5, 0, 0, -1, 0.0, -1),
        tac(1083, "Shining Plant", 5, 0, 0, -1, 0.0, -1),
        tac(1084, "Red Mushroom", 5, 0, 0, -1, 0.0, -1),
        tac(1085, "Black Mushroom", 5, 0, 0, -1, 0.0, -1),
        tac(1095, "Deniro", 5, 0, 0, -1, 1.0, -1),
        tac(1105, "Pierre", 5, 0, 0, -1, 1.0, -1),
        tac(1111, "Drainliar", 11, 0, 2, 0, 1.0, -1),
        tac(1121, "Giearth", 5, 0, 0, -1, 1.0, -1),
        tac(1152, "Orc Skeleton", 11, -3, 2, -1, 1.0, -1),
        tac(1153, "Orc Zombie", 4, -3, 2, -1, 1.0, -1),
        tac(1160, "Andre", 5, 0, 0, -1, 1.0, -1),
        tac(1176, "Vitata", 5, 0, 0, -1, 1.0, -1),
        tac(1177, "Zenorc", 12, -4, 0, -1, 1.0, -1),
        tac(1189, "Orc Archer", 8, 100, 0, -1, 2.0, 0),
        tac(1555, "Parasite Summon", 5, 0, 0, -1, 1.0, -1),
        tac(1575, "Flora Summon", 5, 0, 0, -1, 0.6, -1),
        tac(1579, "Hydra Summon", 5, 0, 0, -1, 0.3, -1),
        tac(1589, "Mandragora Summon", 5, 0, 0, -1, 0.1, -1),
        tac(1590, "Geographer Summon", 5, 0, 0, -1, 1.0, -1),
        tac(2158, "Sera Legion (Hornet)", 5, 0, 0, -1, 0.5, -1),
        tac(2159, "Sera Legion (Giant)", 5, 0, 0, -1, 1.0, -1),
        tac(2160, "Sera Legion (Vespa)", 5, 0, 0, -1, 2.0, -1),
        tac(2379, "Event Monster", 5, 0, 0, -1, 0.0, -1),
        tac(2380, "Event Monster", 5, 0, 0, -1, 0.0, -1),
    ]
}

/// Mercenary per-mob default tactics, ported from the reference merc table.
pub fn default_merc_tactics() -> Vec<Tactic> {
    vec![
        // Default: pushback self, rescue retainer.
        Tactic {
            pushback: PushbackTactic::SelfOnly,
            rescue: RescueTactic::Retainer,
            ..tac(0, "Default", 3, 100, 0, -1, 1.0, -1)
        },
        Tactic {
            pushback: PushbackTactic::SelfOnly,
            ..tac(10, "Default Summon", 7, 100, 1, -1, 1.0, -1)
        },
        Tactic {
            cast: CastTactic::Passive,
            ..tac(11, "Autodetect Plant", 7, 0, 0, -1, 1.0, -1)
        },
    ]
}

fn pvp(key: i32, basic: i32, skill: i32, kite: i32, cast: i32, push: i32) -> PvpTactic {
    PvpTactic {
        key,
        basic: basic.into(),
        skill: skill.into(),
        kite: kite.into(),
        cast: cast.into(),
        pushback: push.into(),
        debuff: 0,
        skill_class: SkillClass::Both,
        rescue: RescueTactic::Never,
    }
}

/// PVP tactics keyed by friend class, ported from the reference PVP table.
pub fn default_pvp_tactics() -> Vec<PvpTactic> {
    vec![
        pvp(0, 5, 100, 0, 1, 0),
        pvp(12, 4, 100, 2, 1, 1), // KoS
        pvp(11, 8, 100, 2, 1, 1), // Enemy
        pvp(10, 7, 100, 0, 0, 0), // Neutral
        pvp(1, 0, 100, 0, 0, 0),  // Friend
        pvp(13, 0, 0, 0, 0, 0),   // Ally
        pvp(2, 0, 0, 0, 0, 0),    // Retainer
    ]
}
