use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::item::ItemType;
use models::enums::weapon::WeaponType;
use ragnarok_formats::act::{ActFile, SpriteActionType, SpriteAnimationState};

use crate::mob_info::MobInfo;
use crate::movement::MovementState;
use crate::scheduled_hit::ScheduledHitQueue;
use crate::sprite_path::weapon_view_id_to_type;

pub const AVG_ATTACKED_SPEED_SECS: f32 = 0.288;

/// Attack-motion time (ms) that plays the swing at its native ACT frame delay.
/// Slower attacks scale up to a cap of 2×, matching the original client.
const AVG_ATTACK_MT_MS: f32 = 432.0;
const MAX_ATTACK_MT_MS: f32 = AVG_ATTACK_MT_MS * 2.0;

/// Maps a server attack-motion time to the swing's animation speed factor.
/// A missing time (`<= 0`) plays at the native ACT speed.
pub fn attack_motion_factor(attack_mt_ms: i32) -> f32 {
    if attack_mt_ms <= 0 {
        return 1.0;
    }
    (attack_mt_ms as f32).min(MAX_ATTACK_MT_MS) / AVG_ATTACK_MT_MS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    Npc,
    Monster,
    Homunculus,
    Mercenary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    Standing,
    Moving,
    Sitting,
    Attacking,
    Casting,
    SkillExec,
    ReadyFight,
    Hurt,
    Dead,
    Pickup,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForcedAnimation {
    pub action: usize,
    pub start_frame: usize,
    pub duration_ms: f32,
    /// Hold a single static frame until cleared externally, instead of playing
    /// the action once and auto-clearing (used by Blade Stop's grip pose).
    pub hold: bool,
    started: bool,
}

impl ForcedAnimation {
    pub fn new(action: usize, start_frame: usize, duration_ms: f32) -> Self {
        Self {
            action,
            start_frame,
            duration_ms,
            hold: false,
            started: false,
        }
    }

    pub fn held(action: usize, frame: usize) -> Self {
        Self {
            action,
            start_frame: frame,
            duration_ms: 0.0,
            hold: true,
            started: false,
        }
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn mark_started(&mut self) {
        self.started = true;
    }
}

/// Attack action-group for a mercenary body (human 13-group layout), by merc
/// class: archers/swordsmen use ATTACK2 (10), lancers use ATTACK3 (11), matching
/// the player attack action for a bow / sword vs. spear.
fn mercenary_attack_action(job: u16) -> usize {
    match job {
        6027..=6036 => 11, // lancer (spear)
        _ => 10,           // archer (bow) / sword
    }
}

/// Mercenary bodies carry no weapon view id, so the weapon type is inferred from
/// the merc class range. This drives swing/hit sounds and the ranged-arrow gate.
pub fn mercenary_weapon(job: u16) -> Option<WeaponType> {
    match job {
        6017..=6026 => Some(WeaponType::Bow),
        6027..=6036 => Some(WeaponType::Spear1H),
        6037..=6046 => Some(WeaponType::Sword1H),
        _ => None,
    }
}

pub const DEATH_FADE_DURATION: f32 = 6.12; // 255 × 24 ms
pub const VANISH_FADE_DURATION: f32 = 0.51; // 510 ms

pub struct EntityFade {
    pub elapsed: f32,
    pub duration: f32,
}

impl EntityFade {
    pub fn alpha(&self) -> f32 {
        (1.0 - self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.duration
    }
}

pub struct EmotionState {
    pub emotion_type: u8,
    pub elapsed: f32,
}

impl EmotionState {
    pub const DISPLAY_DURATION: f32 = 2.5;

    pub fn new(emotion_type: u8) -> Self {
        Self {
            emotion_type,
            elapsed: 0.0,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= Self::DISPLAY_DURATION
    }
}

pub struct ChatBubbleState {
    pub message: String,
    pub elapsed: f32,
}

impl ChatBubbleState {
    pub const DISPLAY_DURATION: f32 = 5.0;

    pub fn new(message: String) -> Self {
        Self {
            message,
            elapsed: 0.0,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= Self::DISPLAY_DURATION
    }
}

pub struct Entity {
    pub id: u32,
    pub entity_type: EntityType,
    pub job: u16,
    pub sex: u8,
    pub head: u16,
    pub hair_color: u16,
    pub cloth_color: u16,
    pub weapon: Option<WeaponType>,
    pub head_top: u16,
    pub head_mid: u16,
    pub head_bottom: u16,
    pub shield: u16,
    pub name: Option<String>,
    pub guild_name: Option<String>,
    pub guild_id: u32,
    pub guild_emblem_version: i32,
    pub position_name: Option<String>,
    pub name_requested: bool,
    pub hp: Option<u32>,
    pub max_hp: Option<u32>,
    pub mob_info: Option<MobInfo>,
    pub direction: u8,
    pub head_dir: u8,
    pub speed: u16,
    pub state: EntityState,
    pub state_timer: f32,
    pub cast_total_duration: f32,
    pub animation_duration: Option<f32>,
    pub animation_start_frame: Option<usize>,
    attack_motion_duration: f32,
    pub attack_motion_factor: f32,
    pub movement: MovementState,
    pub animation: SpriteAnimationState,
    pub emotion: Option<EmotionState>,
    pub chat_bubble: Option<ChatBubbleState>,
    pub active_skill_id: Option<u16>,
    pub skill_hit_count: u16,
    pub scheduled_hits: ScheduledHitQueue,
    pub pending_attack_replays: Vec<(f32, u16)>,
    pub fade: Option<EntityFade>,
    pub pending_death: bool,
    pub just_spawned: bool,
    pub effect_state: i32,
    pub body_state: i16,
    pub health_state: i16,
    /// Blade Stop / Root: darkens the body and freezes motion until released.
    pub rooted: bool,
    pub base_level: i16,
    pub is_boss: bool,
    pub pk_rank: i32,
    pub pk_total: i32,
    pub forced_animation: Option<ForcedAnimation>,
    pub cart_type: Option<u8>,
    /// Vendor shop-name board; `Some` marks this actor as an open vend shop.
    pub vending_board: Option<String>,
    pub anim_last_pos: (f32, f32),
    pub is_running: bool,
    pub footstep_timer: f32,
    pub footstep_left: bool,
    /// Who this actor is currently attacking, inferred from attack events. Backs
    /// the companion AI's "who is targeting my owner / me" scans. Cleared on stop/death.
    pub target_gid: Option<u32>,
    /// Monster actor spawned with head==100 is a pet; drives the accessory ACT
    /// swap and performance actions.
    pub is_pet: bool,
    /// Equipped pet accessory view id (0 = none); selects the accessory ACT variant.
    pub pet_accessory: u16,
    /// Account is listed as a GM: Operator body sprite and yellow name/guild/chat.
    pub is_gm: bool,
}

impl Entity {
    pub fn new(
        id: u32,
        entity_type: EntityType,
        job: u16,
        sex: u8,
        head: u16,
        hair_color: u16,
        weapon: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        shield: u16,
        x: u16,
        y: u16,
        direction: u8,
        speed: u16,
    ) -> Self {
        let weapon_type = match entity_type {
            EntityType::Player => weapon_view_id_to_type(weapon),
            EntityType::Mercenary => mercenary_weapon(job),
            _ => None,
        };
        let mut movement = MovementState::new(x, y);
        movement.set_speed(speed);
        Self {
            id,
            entity_type,
            job,
            sex,
            head,
            hair_color,
            cloth_color: 0,
            weapon: weapon_type,
            head_top,
            head_mid,
            head_bottom,
            shield,
            name: None,
            guild_name: None,
            guild_id: 0,
            guild_emblem_version: 0,
            position_name: None,
            name_requested: false,
            hp: None,
            max_hp: None,
            mob_info: None,
            direction,
            head_dir: 0,
            speed,
            state: EntityState::Standing,
            state_timer: 0.0,
            cast_total_duration: 0.0,
            animation_duration: None,
            animation_start_frame: None,
            attack_motion_duration: 0.0,
            attack_motion_factor: 1.0,
            movement,
            animation: SpriteAnimationState::new(direction),
            emotion: None,
            chat_bubble: None,
            active_skill_id: None,
            skill_hit_count: 0,
            scheduled_hits: ScheduledHitQueue::new(),
            pending_attack_replays: Vec::new(),
            fade: None,
            pending_death: false,
            just_spawned: true,
            effect_state: 0,
            body_state: 0,
            health_state: 0,
            rooted: false,
            base_level: 0,
            is_boss: false,
            pk_rank: 0,
            pk_total: 0,
            forced_animation: None,
            cart_type: None,
            vending_board: None,
            anim_last_pos: (x as f32, y as f32),
            is_running: false,
            footstep_timer: 0.0,
            footstep_left: false,
            target_gid: None,
            is_pet: false,
            pet_accessory: 0,
            is_gm: false,
        }
    }

    pub fn new_player(
        id: u32,
        job: u16,
        sex: u8,
        head: u16,
        hair_color: u16,
        weapon: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        shield: u16,
        x: u16,
        y: u16,
        direction: u8,
    ) -> Self {
        Self::new(
            id,
            EntityType::Player,
            job,
            sex,
            head,
            hair_color,
            weapon,
            head_top,
            head_mid,
            head_bottom,
            shield,
            x,
            y,
            direction,
            150,
        )
    }

    pub fn update_state(&mut self, dt: f32) {
        if let Some(emo) = &mut self.emotion {
            emo.elapsed += dt;
            if emo.is_expired() {
                self.emotion = None;
            }
        }

        if let Some(bubble) = &mut self.chat_bubble {
            bubble.elapsed += dt;
            if bubble.is_expired() {
                self.chat_bubble = None;
            }
        }

        if self.state == EntityState::Dead {
            return;
        }
        if self.pending_death && self.scheduled_hits.is_empty() {
            self.enter_dead();
            return;
        }
        if self.state_timer > 0.0 {
            self.state_timer -= dt;
            if self.state_timer <= 0.0 {
                self.state_timer = 0.0;
                match self.state {
                    EntityState::Attacking
                        if matches!(
                            self.entity_type,
                            EntityType::Player | EntityType::Mercenary
                        ) =>
                    {
                        self.state = EntityState::ReadyFight;
                        self.state_timer = self.attack_motion_duration;
                    }
                    _ => {
                        self.state = EntityState::Standing;
                        self.active_skill_id = None;
                    }
                }
            }
            return;
        }
        if self.state == EntityState::Sitting {
            return;
        }
        self.state = if self.movement.is_moving() {
            self.head_dir = 0;
            EntityState::Moving
        } else {
            EntityState::Standing
        };
    }

    pub fn is_move_locked(&self) -> bool {
        matches!(
            self.state,
            EntityState::Hurt | EntityState::Pickup | EntityState::Attacking
        )
    }

    pub fn begin_move(&mut self, path: Vec<crate::path::PathNode>, now: f32) {
        self.movement.start_move(path, now);
        self.state_timer = 0.0;
    }

    pub fn enter_hurt(&mut self, damage_motion_secs: f32) {
        if matches!(
            self.state,
            EntityState::Dead
                | EntityState::Attacking
                | EntityState::SkillExec
                | EntityState::Casting
        ) {
            return;
        }
        if damage_motion_secs <= AVG_ATTACKED_SPEED_SECS {
            return;
        }
        self.movement.stop();
        self.state = EntityState::Hurt;
        self.state_timer = damage_motion_secs;
        self.animation_duration = Some(damage_motion_secs);
    }

    pub fn enter_attack(&mut self, duration_secs: f32, motion_factor: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::Attacking;
        self.state_timer = duration_secs;
        self.attack_motion_duration = duration_secs;
        self.attack_motion_factor = motion_factor;
        self.animation_duration = Some(duration_secs);
    }

    pub fn enter_attack_replay(&mut self, skill_id: u16) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::SkillExec;
        self.state_timer = 0.2;
        self.active_skill_id = Some(skill_id);
        self.animation_duration = Some(0.3);
        self.animation_start_frame = Some(4);
    }

    pub fn enter_casting(&mut self, duration_secs: f32, skill_id: u16) {
        if self.state == EntityState::Dead {
            return;
        }
        self.movement.stop();
        self.state = EntityState::Casting;
        self.state_timer = duration_secs;
        self.cast_total_duration = duration_secs;
        self.active_skill_id = Some(skill_id);
    }

    pub fn enter_skill_exec(&mut self, duration_secs: f32, skill_id: u16, hit_count: u16) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::SkillExec;
        self.state_timer = duration_secs;
        self.animation_duration = Some(duration_secs);
        self.active_skill_id = Some(skill_id);
        self.skill_hit_count = hit_count;
    }

    pub fn enter_dead(&mut self) {
        self.state = EntityState::Dead;
        self.state_timer = 0.0;
        self.movement.stop();
        self.pending_death = false;
        self.target_gid = None;
    }

    pub fn revive(&mut self) {
        self.state = EntityState::Standing;
        self.state_timer = 0.0;
        self.pending_death = false;
    }

    pub fn request_pending_death(&mut self) {
        self.pending_death = true;
        if self.scheduled_hits.is_empty() {
            self.enter_dead();
        }
    }

    pub fn start_vanish_fade(&mut self) {
        self.fade = Some(EntityFade {
            elapsed: 0.0,
            duration: VANISH_FADE_DURATION,
        });
    }

    pub fn alpha(&self) -> f32 {
        self.fade.as_ref().map_or(1.0, |f| f.alpha())
    }

    pub fn is_fading(&self) -> bool {
        self.fade.is_some()
    }

    pub fn is_alive(&self) -> bool {
        self.state != EntityState::Dead && !self.pending_death && self.fade.is_none()
    }

    pub fn should_remove(&self) -> bool {
        self.fade.as_ref().is_some_and(|f| f.is_expired())
    }

    pub fn enter_pickup(&mut self, duration_secs: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::Pickup;
        self.state_timer = duration_secs;
    }

    pub fn apply_sprite_change(&mut self, sprite_type: u8, value: u16) {
        match sprite_type {
            0 => self.job = value,
            1 => self.head = value,
            2 => self.weapon = weapon_view_id_to_type(value),
            3 => self.head_bottom = value,
            4 => self.head_top = value,
            5 => self.head_mid = value,
            6 => self.hair_color = value,
            7 => self.cloth_color = value,
            8 => self.shield = value,
            _ => {}
        }
    }

    pub fn react_to_status(&mut self, icon: ClientEffectIcon, active: bool) {
        match icon {
            ClientEffectIcon::Run => {
                self.is_running = active;
                self.footstep_timer = 0.0;
            }
            ClientEffectIcon::Ting if active => {
                self.is_running = false;
                self.footstep_timer = 0.0;
            }
            _ => {}
        }
    }

    pub fn wear_location_to_sprite_type(wear_location: u16) -> Option<u8> {
        Self::wear_location_to_sprite_type_for(wear_location, None)
    }

    pub fn wear_location_to_sprite_type_for(
        wear_location: u16,
        item_type: Option<ItemType>,
    ) -> Option<u8> {
        if wear_location & 256 != 0 {
            Some(4)
        } else if wear_location & 512 != 0 {
            Some(5)
        } else if wear_location & 1 != 0 {
            Some(3)
        } else if wear_location & 2 != 0 {
            Some(2)
        } else if wear_location & 32 != 0 {
            if item_type == Some(ItemType::Weapon) {
                Some(2)
            } else {
                Some(8)
            }
        } else {
            None
        }
    }

    pub fn hp_percentage(&self) -> Option<f32> {
        match (self.hp, self.max_hp) {
            (Some(hp), Some(max_hp)) if max_hp > 0 => Some(hp as f32 / max_hp as f32),
            _ => None,
        }
    }

    pub fn action_index(&self) -> usize {
        match self.entity_type {
            EntityType::Player => match self.state {
                EntityState::Standing => 0,
                EntityState::Moving => 1,
                EntityState::Sitting => 2,
                EntityState::Pickup => 3,
                EntityState::ReadyFight => 4,
                EntityState::Attacking => self.attack_action_index(),
                EntityState::Hurt => 6,
                EntityState::Dead => 8,
                EntityState::Casting => 12,
                EntityState::SkillExec => self.skill_exec_action_index(),
            },
            // Mercenaries use human bodies (the full player action layout), not
            // the 5-group monster layout.
            EntityType::Mercenary => match self.state {
                EntityState::Standing => 0,
                EntityState::Moving => 1,
                EntityState::Sitting => 2,
                EntityState::Pickup => 3,
                EntityState::ReadyFight => 4,
                EntityState::Attacking | EntityState::SkillExec => {
                    mercenary_attack_action(self.job)
                }
                EntityState::Hurt => 6,
                EntityState::Dead => 8,
                EntityState::Casting => 12,
            },
            EntityType::Monster | EntityType::Npc | EntityType::Homunculus => match self.state {
                EntityState::Standing
                | EntityState::Sitting
                | EntityState::Pickup
                | EntityState::ReadyFight => 0,
                EntityState::Moving => 1,
                EntityState::Attacking | EntityState::Casting | EntityState::SkillExec => 2,
                EntityState::Hurt => 3,
                EntityState::Dead => 4,
            },
        }
    }

    fn redirect_filler_attack1(&self, group: usize, body_act: &ActFile) -> usize {
        if self.entity_type == EntityType::Player
            && group == SpriteActionType::Attack1 as usize
            && body_act.action_group_is_static(group)
        {
            return SpriteActionType::Attack2 as usize;
        }
        group
    }

    pub fn resolved_action_index(&self, body_act: &ActFile) -> usize {
        self.redirect_filler_attack1(self.action_index(), body_act)
    }

    pub fn resolved_attack_action_index(&self, body_act: &ActFile) -> usize {
        self.redirect_filler_attack1(self.attack_action_index(), body_act)
    }

    /// Action group the entity swings with, whatever its current state.
    pub fn attack_action_index(&self) -> usize {
        match self.entity_type {
            EntityType::Player => self.attack_action_for_weapon(),
            EntityType::Mercenary => mercenary_attack_action(self.job),
            EntityType::Monster | EntityType::Npc | EntityType::Homunculus => 2,
        }
    }

    pub fn skill_exec_start_frame(&self) -> usize {
        use crate::skill_action::{SkillMotionType, skill_motion_type};
        match self.active_skill_id {
            Some(id) => match skill_motion_type(id) {
                SkillMotionType::Skill | SkillMotionType::Sing | SkillMotionType::Dance => 1,
                _ => 0,
            },
            None => 1,
        }
    }

    fn skill_exec_action_index(&self) -> usize {
        use crate::skill_action::{SkillMotionType, skill_motion_type};
        let skill_id = match self.active_skill_id {
            Some(id) => id,
            None => return 12,
        };
        match skill_motion_type(skill_id) {
            SkillMotionType::Attack => self.attack_action_for_weapon(),
            SkillMotionType::Throw => 5,
            SkillMotionType::Attack2 => 10,
            SkillMotionType::Pickup => 3,
            SkillMotionType::Sing | SkillMotionType::Dance | SkillMotionType::Skill => 12,
            SkillMotionType::Stand => 0,
            SkillMotionType::Walk => 1,
        }
    }

    fn attack_action_for_weapon(&self) -> usize {
        let job = match JobName::try_from_value(self.job as usize) {
            Ok(j) => j,
            Err(_) => return 5,
        };
        let weapon = match self.weapon {
            Some(ref w) => w,
            None => {
                return match job {
                    JobName::Monk | JobName::Champion | JobName::BabyMonk => 11,
                    _ => 5,
                };
            }
        };
        let is_female = self.sex == 0;
        match job {
            JobName::Novice
            | JobName::NoviceHigh
            | JobName::BabyNovice
            | JobName::SuperNovice
            | JobName::SuperBaby => {
                if is_female {
                    match weapon {
                        WeaponType::Dagger => 11,
                        _ => 10,
                    }
                } else {
                    match weapon {
                        WeaponType::Dagger => 10,
                        _ => 11,
                    }
                }
            }
            JobName::Swordsman | JobName::SwordsmanHigh | JobName::BabySwordsman => match weapon {
                WeaponType::Spear1H | WeaponType::Spear2H => 11,
                _ => 10,
            },
            JobName::Mage | JobName::MageHigh | JobName::BabyMage => match weapon {
                WeaponType::Dagger => 11,
                _ => 10,
            },
            JobName::Archer | JobName::ArcherHigh | JobName::BabyArcher => match weapon {
                WeaponType::Bow => 10,
                _ => 11,
            },
            JobName::Acolyte | JobName::AcolyteHigh | JobName::BabyAcolyte => 10,
            JobName::Merchant | JobName::MerchantHigh | JobName::BabyMerchant => match weapon {
                WeaponType::Dagger => 11,
                _ => 10,
            },
            JobName::Thief | JobName::ThiefHigh | JobName::BabyThief => match weapon {
                WeaponType::Bow => 11,
                _ => 10,
            },
            JobName::Knight | JobName::LordKnight | JobName::BabyKnight => match weapon {
                WeaponType::Spear1H | WeaponType::Spear2H => 11,
                _ => 10,
            },
            JobName::Priest | JobName::HighPriest | JobName::BabyPriest => match weapon {
                WeaponType::Book => 11,
                _ => 10,
            },
            JobName::Wizard | JobName::HighWizard | JobName::BabyWizard => {
                if is_female {
                    match weapon {
                        WeaponType::Staff | WeaponType::Staff2H => 11,
                        _ => 10,
                    }
                } else {
                    match weapon {
                        WeaponType::Dagger => 11,
                        _ => 10,
                    }
                }
            }
            JobName::Blacksmith | JobName::Whitesmith | JobName::BabyBlacksmith => match weapon {
                WeaponType::Sword1H | WeaponType::Axe1H | WeaponType::Axe2H | WeaponType::Mace => {
                    11
                }
                _ => 10,
            },
            JobName::Hunter | JobName::Sniper | JobName::BabyHunter => match weapon {
                WeaponType::Bow => 11,
                _ => 10,
            },
            JobName::Assassin | JobName::AssassinCross | JobName::BabyAssassin => match weapon {
                WeaponType::Katar
                | WeaponType::DoubleDd
                | WeaponType::DoubleSs
                | WeaponType::DoubleAa
                | WeaponType::DoubleDs
                | WeaponType::DoubleDa
                | WeaponType::DoubleSa => 11,
                _ => 10,
            },
            JobName::Crusader | JobName::Paladin | JobName::BabyCrusader => match weapon {
                WeaponType::Spear1H | WeaponType::Spear2H => 11,
                _ => 10,
            },
            JobName::Monk | JobName::Champion | JobName::BabyMonk => match weapon {
                WeaponType::Knuckle => 11,
                _ => 10,
            },
            JobName::Sage | JobName::Professor | JobName::BabySage => match weapon {
                WeaponType::Book
                | WeaponType::Staff
                | WeaponType::Staff2H
                | WeaponType::Spear2H => 11,
                _ => 10,
            },
            JobName::Rogue | JobName::Stalker | JobName::BabyRogue => match weapon {
                WeaponType::Bow => 11,
                _ => 10,
            },
            JobName::Alchemist | JobName::Creator | JobName::BabyAlchemist => match weapon {
                WeaponType::Sword1H | WeaponType::Axe1H | WeaponType::Axe2H | WeaponType::Mace => {
                    11
                }
                _ => 10,
            },
            JobName::Bard | JobName::Clown | JobName::BabyBard => match weapon {
                WeaponType::Bow => 11,
                _ => 10,
            },
            JobName::Dancer | JobName::Gypsy | JobName::BabyDancer => match weapon {
                WeaponType::Bow => 11,
                _ => 10,
            },
            JobName::SoulLinker => {
                if is_female {
                    match weapon {
                        WeaponType::Staff | WeaponType::Staff2H => 11,
                        _ => 10,
                    }
                } else {
                    match weapon {
                        WeaponType::Dagger => 11,
                        _ => 10,
                    }
                }
            }
            JobName::Gunslinger => match weapon {
                WeaponType::Rifle | WeaponType::Gatling | WeaponType::Grenade => 11,
                _ => 10,
            },
            JobName::Ninja => match weapon {
                WeaponType::Shuriken => 11,
                _ => 10,
            },
            JobName::Taekwon | JobName::StarGladiator => 10,
            _ => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathNode;

    fn make_entity() -> Entity {
        Entity::new_player(1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 100, 100, 0)
    }

    fn make_body_act(frames: usize, filler: &[usize]) -> ActFile {
        use ragnarok_formats::act::{Action, Motion, SpriteFrame};
        let frame = |spr: i32| SpriteFrame {
            x: 0,
            y: 0,
            sprite_index: spr,
            mirror: 0,
            color: [255; 4],
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0,
            sprite_type: 0,
            width: None,
            height: None,
        };
        let actions = (0..13 * 8)
            .map(|flat| {
                let is_filler = filler.contains(&(flat / 8));
                Action {
                    motions: (0..frames)
                        .map(|f| Motion {
                            range1: [0; 4],
                            range2: [0; 4],
                            clips: vec![frame(if is_filler { 0 } else { f as i32 })],
                            event_id: -1,
                            attach_points: Vec::new(),
                        })
                        .collect(),
                }
            })
            .collect();
        ActFile {
            version: (2, 5),
            actions,
            events: Vec::new(),
            delays: vec![4.0; 13 * 8],
        }
    }

    fn make_path_node(x: u16, y: u16, is_diagonal: bool) -> PathNode {
        PathNode {
            id: 0,
            parent_id: 0,
            x,
            y,
            g_cost: 0,
            f_cost: 0,
            is_open: false,
            is_diagonal,
        }
    }

    #[test]
    fn mercenary_entity_infers_weapon_from_class() {
        let merc = |job| {
            Entity::new(
                1,
                EntityType::Mercenary,
                job,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                100,
                100,
                0,
                150,
            )
            .weapon
        };
        assert_eq!(merc(6017), Some(WeaponType::Bow)); // archer
        assert_eq!(merc(6027), Some(WeaponType::Spear1H)); // lancer
        assert_eq!(merc(6037), Some(WeaponType::Sword1H)); // swordman
    }

    #[test]
    fn entity_starts_without_name() {
        let e = make_entity();
        assert!(e.name.is_none());
        assert!(!e.name_requested);
    }

    #[test]
    fn running_status_mirrors_active_and_ting_force_stops() {
        let mut e = make_entity();
        e.react_to_status(ClientEffectIcon::Run, true);
        assert!(e.is_running);
        e.react_to_status(ClientEffectIcon::Ting, true);
        assert!(!e.is_running);
        e.is_running = true;
        e.react_to_status(ClientEffectIcon::Run, false);
        assert!(!e.is_running);
    }

    #[test]
    fn action_index_maps_states_to_player_sprite_actions() {
        let mut e = make_entity();
        assert_eq!(e.action_index(), 0);
        e.state = EntityState::Moving;
        assert_eq!(e.action_index(), 1);
        e.state = EntityState::Sitting;
        assert_eq!(e.action_index(), 2);
        e.state = EntityState::Pickup;
        assert_eq!(e.action_index(), 3);
        e.state = EntityState::ReadyFight;
        assert_eq!(e.action_index(), 4);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 5);
        e.state = EntityState::Hurt;
        assert_eq!(e.action_index(), 6);
        e.state = EntityState::Dead;
        assert_eq!(e.action_index(), 8);
        e.state = EntityState::Casting;
        assert_eq!(e.action_index(), 12);
        e.state = EntityState::SkillExec;
        assert_eq!(e.action_index(), 12);
    }

    #[test]
    fn action_index_maps_states_to_monster_sprite_actions() {
        let mut e = Entity::new(
            2,
            EntityType::Monster,
            1002,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            100,
            100,
            0,
            200,
        );
        assert_eq!(e.action_index(), 0);
        e.state = EntityState::Moving;
        assert_eq!(e.action_index(), 1);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 2);
        e.state = EntityState::SkillExec;
        assert_eq!(e.action_index(), 2);
        e.state = EntityState::Casting;
        assert_eq!(e.action_index(), 2);
        e.state = EntityState::Hurt;
        assert_eq!(e.action_index(), 3);
        e.state = EntityState::Dead;
        assert_eq!(e.action_index(), 4);
    }

    #[test]
    fn pending_death_resolves_when_a_delayed_hit_lands_even_at_rest() {
        use crate::scheduled_hit::ScheduledHit;
        let mut e = Entity::new(
            2,
            EntityType::Monster,
            1002,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            100,
            100,
            0,
            200,
        );
        e.state = EntityState::Standing;
        let mut hit = ScheduledHit::single(50, 17, false);
        hit.fire_at = 10.0;
        e.scheduled_hits.push(hit);

        e.request_pending_death();
        e.update_state(0.1);
        assert_eq!(
            e.state,
            EntityState::Standing,
            "stays alive while the bolt flies"
        );
        assert!(e.pending_death);

        e.scheduled_hits.drain_ready(10.0);
        e.update_state(0.1);
        assert_eq!(
            e.state,
            EntityState::Dead,
            "dies once the delayed hit has landed"
        );
        assert!(!e.pending_death);
    }

    #[test]
    fn update_state_preserves_sitting() {
        let mut e = make_entity();
        e.state = EntityState::Sitting;
        e.update_state(0.016);
        assert_eq!(e.state, EntityState::Sitting);
    }

    #[test]
    fn walking_resets_turned_head_to_default() {
        let mut e = make_entity();
        e.head_dir = 2;
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        e.movement.start_move(path, 0.0);

        e.update_state(0.016);

        assert_eq!(e.state, EntityState::Moving);
        assert_eq!(e.head_dir, 0);
    }

    #[test]
    fn hurt_cancels_movement_and_recovers_to_standing() {
        let mut e = make_entity();
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        e.movement.start_move(path, 0.0);
        assert!(e.movement.is_moving());

        assert!(!e.is_move_locked());

        e.enter_hurt(0.5);
        assert_eq!(e.state, EntityState::Hurt);
        assert!(!e.movement.is_moving());
        assert!(e.is_move_locked());

        e.update_state(0.3);
        assert_eq!(e.state, EntityState::Hurt);
        assert!(e.is_move_locked());

        e.update_state(0.3);
        assert_eq!(e.state, EntityState::Standing);
        assert!(!e.is_move_locked());
    }

    #[test]
    fn pickup_motion_locks_movement_until_it_ends() {
        let mut e = make_entity();
        e.enter_pickup(0.5);
        assert_eq!(e.state, EntityState::Pickup);
        assert!(e.is_move_locked());

        e.update_state(0.6);
        assert_eq!(e.state, EntityState::Standing);
        assert!(!e.is_move_locked());
    }

    #[test]
    fn light_hit_below_threshold_keeps_walking() {
        let mut e = make_entity();
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        e.movement.start_move(path, 0.0);

        e.enter_hurt(0.2);
        e.update_state(0.016);
        assert_eq!(e.state, EntityState::Moving);
        assert!(e.movement.is_moving());
        assert!(!e.is_move_locked());
    }

    #[test]
    fn dead_blocks_all_transitions() {
        let mut e = make_entity();
        e.enter_dead();
        assert_eq!(e.state, EntityState::Dead);

        e.enter_hurt(1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_attack(1.0, 1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_skill_exec(1.0, 0, 1);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_pickup(1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_casting(1.0, 0);
        assert_eq!(e.state, EntityState::Dead);

        e.update_state(1.0);
        assert_eq!(e.state, EntityState::Dead);
    }

    #[test]
    fn attack_motion_factor_is_native_at_average_and_capped_when_slow() {
        assert_eq!(attack_motion_factor(432), 1.0);
        assert_eq!(attack_motion_factor(216), 0.5);
        assert_eq!(
            attack_motion_factor(5000),
            2.0,
            "slow attacks cap at 2x native"
        );
        assert_eq!(
            attack_motion_factor(0),
            1.0,
            "missing time plays at native speed"
        );
    }

    #[test]
    fn attacking_and_skill_exec_block_hurt() {
        let mut e = make_entity();
        e.enter_attack(1.0, 1.0);
        assert_eq!(e.state, EntityState::Attacking);
        e.enter_hurt(0.5);
        assert_eq!(e.state, EntityState::Attacking);

        let mut e2 = make_entity();
        e2.enter_skill_exec(1.0, 0, 1);
        assert_eq!(e2.state, EntityState::SkillExec);
        e2.enter_hurt(0.5);
        assert_eq!(e2.state, EntityState::SkillExec);
    }

    #[test]
    fn apply_sprite_change_updates_entity_fields() {
        let mut e = make_entity();
        e.apply_sprite_change(0, 4001);
        assert_eq!(e.job, 4001);
        e.apply_sprite_change(1, 5);
        assert_eq!(e.head, 5);
        e.apply_sprite_change(3, 10);
        assert_eq!(e.head_bottom, 10);
        e.apply_sprite_change(4, 20);
        assert_eq!(e.head_top, 20);
        e.apply_sprite_change(5, 30);
        assert_eq!(e.head_mid, 30);
        e.apply_sprite_change(6, 3);
        assert_eq!(e.hair_color, 3);
        e.apply_sprite_change(8, 2);
        assert_eq!(e.shield, 2);
    }

    #[test]
    fn hp_percentage_returns_ratio_or_none() {
        let mut e = make_entity();
        assert!(e.hp_percentage().is_none());

        e.hp = Some(75);
        e.max_hp = Some(100);
        assert!((e.hp_percentage().unwrap() - 0.75).abs() < f32::EPSILON);

        e.max_hp = Some(0);
        assert!(e.hp_percentage().is_none());
    }

    #[test]
    fn chat_bubble_expires_after_duration() {
        let mut e = make_entity();
        e.chat_bubble = Some(ChatBubbleState::new("Hello!".to_string()));
        assert!(e.chat_bubble.is_some());

        e.update_state(3.0);
        assert!(e.chat_bubble.is_some());
        assert_eq!(e.chat_bubble.as_ref().unwrap().message, "Hello!");

        e.update_state(2.1);
        assert!(e.chat_bubble.is_none());
    }

    #[test]
    fn emotion_expires_after_duration() {
        let mut e = make_entity();
        e.emotion = Some(super::EmotionState::new(0));
        assert!(e.emotion.is_some());

        e.update_state(1.0);
        assert!(e.emotion.is_some());

        e.update_state(1.6);
        assert!(e.emotion.is_none());
    }

    #[test]
    fn casting_uses_action_index_12_for_player() {
        let mut e = make_entity();
        e.enter_casting(2.0, 0);
        assert_eq!(e.state, EntityState::Casting);
        assert_eq!(e.action_index(), 12);
    }

    #[test]
    fn casting_blocks_hurt() {
        let mut e = make_entity();
        e.enter_casting(2.0, 0);
        assert_eq!(e.state, EntityState::Casting);

        e.enter_hurt(0.5);
        assert_eq!(e.state, EntityState::Casting);
    }

    #[test]
    fn casting_counts_down_to_standing() {
        let mut e = make_entity();
        e.enter_casting(1.0, 0);
        assert_eq!(e.state, EntityState::Casting);

        e.update_state(0.5);
        assert_eq!(e.state, EntityState::Casting);

        e.update_state(0.6);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn enter_casting_stops_movement() {
        let mut e = make_entity();
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        e.movement.start_move(path, 0.0);
        assert!(e.movement.is_moving());

        e.enter_casting(2.0, 0);
        assert!(!e.movement.is_moving());
        assert_eq!(e.cast_total_duration, 2.0);
    }

    #[test]
    fn attack_expires_to_readyfight_for_player() {
        let mut e = make_entity();
        e.enter_attack(0.5, 1.0);
        assert_eq!(e.state, EntityState::Attacking);
        assert!(e.is_move_locked());

        e.update_state(0.6);
        assert_eq!(e.state, EntityState::ReadyFight);
        assert_eq!(e.action_index(), 4);
        assert!(!e.is_move_locked());

        e.update_state(0.4);
        assert_eq!(e.state, EntityState::ReadyFight);
        e.update_state(0.2);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn barehand_swing_avoids_a_filler_attack1_group() {
        let mut e = make_entity();
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 5, "barehand picks ATTACK1");

        let with_attack1 = make_body_act(5, &[]);
        assert_eq!(e.resolved_action_index(&with_attack1), 5);
        assert_eq!(e.resolved_attack_action_index(&with_attack1), 5);

        let filler_attack1 = make_body_act(5, &[5]);
        assert_eq!(e.resolved_action_index(&filler_attack1), 10);
        assert_eq!(e.resolved_attack_action_index(&filler_attack1), 10);
    }

    #[test]
    fn move_during_standby_switches_to_walking() {
        let mut e = make_entity();
        e.enter_attack(0.9, 1.0);
        e.update_state(1.0);
        assert_eq!(e.state, EntityState::ReadyFight);

        e.begin_move(
            vec![
                make_path_node(101, 100, false),
                make_path_node(102, 100, false),
            ],
            0.0,
        );
        e.update_state(0.016);
        assert_eq!(e.state, EntityState::Moving);
        assert_eq!(e.action_index(), 1);
    }

    #[test]
    fn skill_exec_expires_to_standing_for_player() {
        let mut e = make_entity();
        e.enter_skill_exec(0.5, 0, 1);
        assert_eq!(e.state, EntityState::SkillExec);
        assert_eq!(e.action_index(), 12);

        e.update_state(0.6);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn attack_expires_to_standing_for_monster() {
        let mut e = Entity::new(
            2,
            EntityType::Monster,
            1002,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            100,
            100,
            0,
            200,
        );
        e.enter_attack(0.5, 1.0);
        e.update_state(0.6);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn weapon_dependent_attack_action() {
        let mut e = Entity::new_player(1, 1, 1, 1, 0, 4, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 1, 1, 1, 0, 2, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 10);

        let mut e = Entity::new_player(1, 12, 1, 1, 0, 16, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 3, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 10);

        let mut e = Entity::new_player(1, 3, 1, 1, 0, 1, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 11, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 19, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 15, 1, 1, 0, 0, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 15, 1, 1, 0, 12, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 15, 1, 1, 0, 8, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 10);
    }

    #[test]
    fn death_fade_alpha_decreases_linearly() {
        let mut e = Entity::new(
            1,
            EntityType::Monster,
            1002,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            100,
            100,
            0,
            200,
        );
        assert!((e.alpha() - 1.0).abs() < f32::EPSILON);

        e.fade = Some(super::EntityFade {
            elapsed: 0.0,
            duration: DEATH_FADE_DURATION,
        });
        assert!((e.alpha() - 1.0).abs() < f32::EPSILON);

        e.fade.as_mut().unwrap().elapsed = 3.06;
        assert!((e.alpha() - 0.5).abs() < 0.01);

        e.fade.as_mut().unwrap().elapsed = 6.12;
        assert!((e.alpha() - 0.0).abs() < f32::EPSILON);
        assert!(e.should_remove());
    }

    #[test]
    fn vanish_fade_expires_at_510ms() {
        let mut e = make_entity();
        e.start_vanish_fade();
        assert!(e.is_fading());
        assert!(!e.should_remove());

        e.fade.as_mut().unwrap().elapsed = 0.51;
        assert!(e.should_remove());
    }

    #[test]
    fn player_death_no_fade_by_default() {
        let mut e = make_entity();
        e.enter_dead();
        assert_eq!(e.state, EntityState::Dead);
        assert!(!e.is_fading());
        assert!((e.alpha() - 1.0).abs() < f32::EPSILON);
        assert!(!e.should_remove());
    }
}
