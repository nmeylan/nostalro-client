#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Stand,
    Move,
    Attack,
    Cast,
    Sit,
    Hurt,
    Skill,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiIntent {
    MoveTo {
        x: i32,
        y: i32,
    },
    MoveToOwner,
    Attack {
        target_gid: u32,
    },
    SkillObject {
        skill_id: u16,
        level: u8,
        target_gid: u32,
    },
    SkillGround {
        skill_id: u16,
        level: u8,
        x: i32,
        y: i32,
    },
    EmergencyDisconnect,
}

/// One actor visible to the AI's world scans.
#[derive(Debug, Clone, Copy)]
pub struct ActorView {
    pub gid: u32,
    pub x: i32,
    pub y: i32,
    pub is_monster: bool,
    pub is_player: bool,
    /// Mob class id for monsters (= `entity.job`), the tactics-table key.
    pub class_id: u16,
    pub motion: Motion,
    /// Who this actor is attacking (`None` when idle).
    pub target_gid: Option<u32>,
}

/// The subset of config fields the engine reads each tick, extracted from the
/// active companion's `HomunConfig`/`MercConfig` so the crate stays decoupled
/// from the full config layout.
#[derive(Debug, Clone, Copy)]
pub struct AiParams {
    pub aggro_hp: i32,
    pub aggro_sp: i32,
    pub super_passive: bool,
    pub opportunistic: bool,
    pub do_not_attack_moving: bool,
    pub attack_last_full_sp: bool,
    pub tank_monster_limit: i32,
    pub auto_mob_count: i32,
    pub stationary_aggro_dist: i32,
    pub mobile_aggro_dist: i32,
    pub stationary_move_bounds: i32,
    pub mobile_move_bounds: i32,
    pub do_not_chase: bool,
    pub chase_sp_pause: bool,
    pub chase_sp_pause_sp: i32,
    pub chase_sp_pause_time: i32,
    pub attack_skill_reserve_sp: i32,
    pub rescue_owner_low_hp: i32,
    pub use_attack_skill: bool,
    pub auto_skill_delay: i32,
    pub use_offensive_buff: i32,
    pub use_defensive_buff: i32,
    pub use_auto_heal: i32,
    pub heal_owner_hp: i32,
    pub heal_self_hp: i32,
    pub do_not_use_rest: bool,
    pub use_dance_attack: bool,
    pub dance_min_sp: i32,
    pub use_idle_walk: i32,
    pub idle_walk_sp: i32,
    pub use_berserk_mobbed: i32,
    pub use_avoid: bool,
    pub use_castle_route: bool,
    pub use_castle_defend: bool,
    pub castle_defend_threshold: i32,
    pub pvp_mode: bool,
}

impl Default for AiParams {
    fn default() -> Self {
        AiParams {
            aggro_hp: 60,
            aggro_sp: 0,
            super_passive: false,
            opportunistic: false,
            do_not_attack_moving: false,
            attack_last_full_sp: false,
            tank_monster_limit: 4,
            auto_mob_count: 2,
            stationary_aggro_dist: 12,
            mobile_aggro_dist: 7,
            stationary_move_bounds: 14,
            mobile_move_bounds: 9,
            do_not_chase: false,
            chase_sp_pause: false,
            chase_sp_pause_sp: 0,
            chase_sp_pause_time: 0,
            attack_skill_reserve_sp: 0,
            rescue_owner_low_hp: 0,
            use_attack_skill: true,
            auto_skill_delay: 400,
            use_offensive_buff: 1,
            use_defensive_buff: 1,
            use_auto_heal: 0,
            heal_owner_hp: 50,
            heal_self_hp: 50,
            do_not_use_rest: false,
            use_dance_attack: false,
            dance_min_sp: 0,
            use_idle_walk: 0,
            idle_walk_sp: 0,
            use_berserk_mobbed: 0,
            use_avoid: false,
            use_castle_route: false,
            use_castle_defend: false,
            castle_defend_threshold: 4,
            pvp_mode: false,
        }
    }
}

/// A companion skill known to the engine, from the live server skill list.
#[derive(Debug, Clone, Copy)]
pub struct CompanionSkill {
    pub id: u16,
    pub level: u8,
    pub sp_cost: u16,
    pub range: i32,
}

/// Borrowed world snapshot the caller assembles each tick.
pub struct AiContext<'a> {
    pub my_gid: u32,
    pub my_x: i32,
    pub my_y: i32,
    pub my_motion: Motion,
    pub my_hp: u32,
    pub my_max_hp: u32,
    pub my_sp: u32,
    pub my_max_sp: u32,
    pub attack_range: i32,
    pub aspd_ms: u32,
    /// Homunculus type (1..16) or mercenary type (1..30).
    pub companion_type: u16,
    pub owner_gid: u32,
    /// Owner cell, or `None` when the owner is unknown / off-screen.
    pub owner_pos: Option<(i32, i32)>,
    pub owner_motion: Motion,
    /// Owner HP percent (0..100), or `None` when unknown.
    pub owner_hp_pct: Option<i32>,
    pub spheres: u16,
    pub now_ms: u32,
    pub actors: &'a [ActorView],
    pub skills: &'a [CompanionSkill],
    pub skill_range: &'a dyn Fn(u16) -> i32,
    pub params: AiParams,
    pub tactics: &'a crate::tactics::TacticTable,
    pub pvp_tactics: &'a [crate::tactics::PvpTactic],
    pub friend_class: &'a dyn Fn(u32) -> crate::consts::FriendClass,
}

impl AiContext<'_> {
    pub fn hp_pct(&self) -> i32 {
        pct(self.my_hp, self.my_max_hp)
    }
    pub fn sp_pct(&self) -> i32 {
        pct(self.my_sp, self.my_max_sp)
    }
    pub fn owner_moving(&self) -> bool {
        self.owner_motion == Motion::Move
    }
    pub fn aggro_dist(&self) -> i32 {
        if self.owner_moving() {
            self.params.mobile_aggro_dist
        } else {
            self.params.stationary_aggro_dist
        }
    }
    pub fn move_bounds(&self) -> i32 {
        if self.owner_moving() {
            self.params.mobile_move_bounds
        } else {
            self.params.stationary_move_bounds
        }
    }

    /// The PVP tactic row for a friend class, falling back to the key-0 row.
    pub fn pvp_tactic(
        &self,
        class: crate::consts::FriendClass,
    ) -> Option<&crate::tactics::PvpTactic> {
        let key = i32::from(class);
        self.pvp_tactics
            .iter()
            .find(|t| t.key == key)
            .or_else(|| self.pvp_tactics.iter().find(|t| t.key == 0))
    }
}

fn pct(cur: u32, max: u32) -> i32 {
    if max == 0 {
        0
    } else {
        ((cur as f32 / max as f32) * 100.0).round() as i32
    }
}
