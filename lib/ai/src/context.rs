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
    MoveTo { x: i32, y: i32 },
    MoveToOwner,
    Attack { target_gid: u32 },
    SkillObject { skill_id: u16, level: u8, target_gid: u32 },
    SkillGround { skill_id: u16, level: u8, x: i32, y: i32 },
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
    pub spheres: u16,
    pub now_ms: u32,
    pub actors: &'a [ActorView],
    pub skill_range: &'a dyn Fn(u16) -> i32,
}
