pub use ragnarok_ai as ai;
pub use ragnarok_ai::{
    ActorView, AiContext, AiIntent, AiState, CommandKind, CompanionAi, Motion, OwnerCommand,
};

use crate::event::SkillInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionKind {
    Homunculus,
    Mercenary,
}

pub struct HomunculusState {
    pub gid: u32,
    pub job: u16,
    pub name: String,
    pub renamed: bool,
    pub level: i16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub exp: i32,
    pub max_exp: i32,
    pub hunger: i16,
    pub intimacy: i16,
    pub accessory: u16,
    pub atk: i16,
    pub matk: i16,
    pub hit: i16,
    pub critical: i16,
    pub def: i16,
    pub mdef: i16,
    pub flee: i16,
    pub aspd: i16,
    pub atk_range: i16,
    pub skill_points: i16,
    pub skills: Vec<SkillInfo>,
    pub vaporized: bool,
    pub ai: CompanionAi,
}

impl HomunculusState {
    pub fn new(gid: u32) -> Self {
        Self {
            gid,
            job: 0,
            name: String::new(),
            renamed: false,
            level: 0,
            hp: 0,
            max_hp: 0,
            sp: 0,
            max_sp: 0,
            exp: 0,
            max_exp: 0,
            hunger: 0,
            intimacy: 0,
            accessory: 0,
            atk: 0,
            matk: 0,
            hit: 0,
            critical: 0,
            def: 0,
            mdef: 0,
            flee: 0,
            aspd: 0,
            atk_range: 0,
            skill_points: 0,
            skills: Vec::new(),
            vaporized: false,
            ai: CompanionAi::new(false),
        }
    }

    pub fn hp_percentage(&self) -> f32 {
        if self.max_hp > 0 {
            self.hp as f32 / self.max_hp as f32
        } else {
            0.0
        }
    }

    pub fn sp_percentage(&self) -> f32 {
        if self.max_sp > 0 {
            self.sp as f32 / self.max_sp as f32
        } else {
            0.0
        }
    }

    pub fn exp_percentage(&self) -> f32 {
        if self.max_exp > 0 {
            self.exp as f32 / self.max_exp as f32
        } else {
            0.0
        }
    }

    pub fn is_alive(&self) -> bool {
        !self.vaporized && self.hp > 0
    }
}

pub struct MercenaryState {
    pub gid: u32,
    pub job: u16,
    pub name: String,
    pub level: i16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub atk: i16,
    pub matk: i16,
    pub hit: i16,
    pub critical: i16,
    pub def: i16,
    pub mdef: i16,
    pub flee: i16,
    pub aspd: i16,
    pub atk_range: i16,
    pub faith: i16,
    pub expire_date: i32,
    pub calls: i32,
    pub kills: i32,
    pub skills: Vec<SkillInfo>,
    pub ai: CompanionAi,
}

impl MercenaryState {
    pub fn new(gid: u32) -> Self {
        Self {
            gid,
            job: 0,
            name: String::new(),
            level: 0,
            hp: 0,
            max_hp: 0,
            sp: 0,
            max_sp: 0,
            atk: 0,
            matk: 0,
            hit: 0,
            critical: 0,
            def: 0,
            mdef: 0,
            flee: 0,
            aspd: 0,
            atk_range: 0,
            faith: 0,
            expire_date: 0,
            calls: 0,
            kills: 0,
            skills: Vec::new(),
            ai: CompanionAi::new(true),
        }
    }

    pub fn hp_percentage(&self) -> f32 {
        if self.max_hp > 0 {
            self.hp as f32 / self.max_hp as f32
        } else {
            0.0
        }
    }

    pub fn sp_percentage(&self) -> f32 {
        if self.max_sp > 0 {
            self.sp as f32 / self.max_sp as f32
        } else {
            0.0
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}
