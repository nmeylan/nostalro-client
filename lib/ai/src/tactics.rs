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

/// One monster-class row (13 columns). `id` is the mob class id; row `0` is the
/// non-deletable default and row `13` the treasure-chest special.
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

pub fn default_tactics() -> Vec<Tactic> {
    vec![Tactic::default_row(), Tactic::treasure_row()]
}

pub fn default_pvp_tactics() -> Vec<PvpTactic> {
    vec![PvpTactic {
        key: 0,
        basic: BasicTactic::ReactLow,
        skill: SkillUse::Always,
        kite: KiteTactic::Never,
        cast: CastTactic::React,
        pushback: PushbackTactic::Never,
        debuff: 0,
        skill_class: SkillClass::Both,
        rescue: RescueTactic::Never,
    }]
}
