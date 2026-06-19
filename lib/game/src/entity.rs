use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::item::ItemType;
use models::enums::weapon::WeaponType;
use ragnarok_formats::act::SpriteAnimationState;

use crate::movement::MovementState;
use crate::scheduled_hit::ScheduledHitQueue;
use crate::sprite_path::weapon_view_id_to_type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    Npc,
    Monster,
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

/// A forced actor animation pushed by a body effect (e.g. Jumpkick's
/// forced kick pose). While set it overrides the
/// state-driven action and suppresses normal selection until the OneShot
/// finishes, then clears — mirroring the original game's forced animation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForcedAnimation {
    pub action: usize,
    pub start_frame: usize,
    pub duration_ms: f32,
    started: bool,
}

impl ForcedAnimation {
    pub fn new(action: usize, start_frame: usize, duration_ms: f32) -> Self {
        Self { action, start_frame, duration_ms, started: false }
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn mark_started(&mut self) {
        self.started = true;
    }
}

pub const DEATH_FADE_DURATION: f32 = 6.12; // 255 * 24ms, matching original client corpse fade
pub const VANISH_FADE_DURATION: f32 = 0.51; // 510ms, matching original client out-of-sight fade

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
    pub name_requested: bool,
    pub hp: Option<u32>,
    pub max_hp: Option<u32>,
    pub direction: u8,
    pub head_dir: u8,
    pub speed: u16,
    pub state: EntityState,
    pub state_timer: f32,
    pub cast_total_duration: f32,
    /// Animation duration override in seconds, applied once when the action changes.
    pub animation_duration: Option<f32>,
    /// Override start frame for the next animation play (consumed alongside animation_duration).
    pub animation_start_frame: Option<usize>,
    /// Last attack motion duration from server, used as ReadyFight timer after attack.
    attack_motion_duration: f32,
    pub movement: MovementState,
    pub animation: SpriteAnimationState,
    pub emotion: Option<EmotionState>,
    pub chat_bubble: Option<ChatBubbleState>,
    pub active_skill_id: Option<u16>,
    pub skill_hit_count: u16,
    pub scheduled_hits: ScheduledHitQueue,
    pub pending_attack_replays: Vec<(f32, u16)>,
    pub fade: Option<EntityFade>,
    /// True when the server sent a death event but we're waiting for
    /// all scheduled hits to finish their hurt animation first.
    pub pending_death: bool,
    pub just_spawned: bool,
    pub effect_state: i32,
    pub body_state: i16,
    pub health_state: i16,
    pub base_level: i16,
    pub is_boss: bool,
    /// Active forced animation from a body effect (Jumpkick), if any.
    pub forced_animation: Option<ForcedAnimation>,
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
        let weapon_type = if entity_type == EntityType::Player {
            weapon_view_id_to_type(weapon)
        } else {
            None
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
            name_requested: false,
            hp: None,
            max_hp: None,
            direction,
            head_dir: direction,
            speed,
            state: EntityState::Standing,
            state_timer: 0.0,
            cast_total_duration: 0.0,
            animation_duration: None,
            animation_start_frame: None,
            attack_motion_duration: 0.0,
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
            base_level: 0,
            is_boss: false,
            forced_animation: None,
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
        if self.state_timer > 0.0 {
            self.state_timer -= dt;
            if self.state_timer <= 0.0 {
                self.state_timer = 0.0;
                // If a death was requested while we were in a transient state,
                // transition to Dead now that the hurt animation has finished.
                if self.pending_death {
                    self.state = EntityState::Dead;
                    self.pending_death = false;
                    self.movement.stop();
                    return;
                }
                match self.state {
                    EntityState::Attacking if self.entity_type == EntityType::Player => {
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
            EntityState::Moving
        } else {
            EntityState::Standing
        };
    }

    pub fn enter_hurt(&mut self, duration_secs: f32) {
        if matches!(
            self.state,
            EntityState::Dead
                | EntityState::Attacking
                | EntityState::SkillExec
                | EntityState::Casting
        ) {
            return;
        }
        self.movement.stop();
        self.state = EntityState::Hurt;
        self.state_timer = duration_secs;
    }

    pub fn enter_attack(&mut self, duration_secs: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::Attacking;
        self.state_timer = duration_secs;
        self.attack_motion_duration = duration_secs;
        self.animation_duration = Some(duration_secs);
    }

    /// Caster attack replay for multi-hit skills (Sonic Blow, Chain Crush, Arrow Vulcan).
    /// Starts at frame 4 (weapon swing), matching the original game's attack motion
    /// at motion 4, motion speed 1.
    pub fn enter_attack_replay(&mut self, skill_id: u16) {
        if self.state == EntityState::Dead {
            return;
        }
        // Stay in SkillExec so action_index() uses skill_exec_action_index(),
        // which picks the correct action based on active_skill_id (e.g. Attack2
        // for Arrow Vulcan). Using Attacking state would call
        // attack_action_for_weapon() which may return a different action.
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
    }

    pub fn revive(&mut self) {
        self.state = EntityState::Standing;
        self.state_timer = 0.0;
        self.pending_death = false;
    }

    /// Request death but defer it until all scheduled hits complete.
    /// If there are no pending scheduled hits, transition to Dead immediately.
    pub fn request_pending_death(&mut self) {
        self.pending_death = true;
        if self.scheduled_hits.is_empty() {
            // No scheduled hits left, transition to Dead immediately
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

    pub fn wear_location_to_sprite_type(wear_location: u16) -> Option<u8> {
        Self::wear_location_to_sprite_type_for(wear_location, None)
    }

    pub fn wear_location_to_sprite_type_for(
        wear_location: u16,
        item_type: Option<ItemType>,
    ) -> Option<u8> {
        if wear_location & 256 != 0 {
            Some(4)
        }
        // HeadTop
        else if wear_location & 512 != 0 {
            Some(5)
        }
        // HeadMid
        else if wear_location & 1 != 0 {
            Some(3)
        }
        // HeadLow
        else if wear_location & 2 != 0 {
            Some(2)
        }
        // Weapon (HandRight, also two-handed)
        else if wear_location & 32 != 0 {
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
                EntityState::Attacking => self.attack_action_for_weapon(),
                EntityState::Hurt => 6,
                EntityState::Dead => 8,
                EntityState::Casting => 12,
                EntityState::SkillExec => self.skill_exec_action_index(),
            },
            EntityType::Monster | EntityType::Npc => match self.state {
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

    /// Returns the start frame for the current skill exec animation.
    /// SKILL action (index 12) starts at frame 1 (frame 0 is static pose),
    /// attack-type actions start at frame 0.
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
        }
    }

    /// Returns the attack action index based on job and equipped weapon.
    /// 5 = Attack1 (unarmed), 10 = Attack2 (primary weapon), 11 = Attack3 (alternate weapon)
    fn attack_action_for_weapon(&self) -> usize {
        let job = match JobName::try_from_value(self.job as usize) {
            Ok(j) => j,
            Err(_) => return 5,
        };
        let weapon = match self.weapon {
            Some(ref w) => w,
            None => {
                return match job {
                    // Monk unarmed uses alternate attack animation
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
    fn entity_starts_without_name() {
        let e = make_entity();
        assert!(e.name.is_none());
        assert!(!e.name_requested);
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
        assert_eq!(e.action_index(), 5); // Default unarmed → Attack1
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
    fn update_state_preserves_sitting() {
        let mut e = make_entity();
        e.state = EntityState::Sitting;
        e.update_state(0.016);
        assert_eq!(e.state, EntityState::Sitting);
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

        e.enter_hurt(0.5);
        assert_eq!(e.state, EntityState::Hurt);
        assert!(!e.movement.is_moving());

        // Still in hurt state after partial tick
        e.update_state(0.3);
        assert_eq!(e.state, EntityState::Hurt);

        // Timer expires, returns to standing
        e.update_state(0.3);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn dead_blocks_all_transitions() {
        let mut e = make_entity();
        e.enter_dead();
        assert_eq!(e.state, EntityState::Dead);

        e.enter_hurt(1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_attack(1.0);
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
    fn attacking_and_skill_exec_block_hurt() {
        let mut e = make_entity();
        e.enter_attack(1.0);
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
        e.apply_sprite_change(0, 4001); // job
        assert_eq!(e.job, 4001);
        e.apply_sprite_change(1, 5); // head
        assert_eq!(e.head, 5);
        e.apply_sprite_change(3, 10); // head_bottom (accessory)
        assert_eq!(e.head_bottom, 10);
        e.apply_sprite_change(4, 20); // head_top (accessory2)
        assert_eq!(e.head_top, 20);
        e.apply_sprite_change(5, 30); // head_mid (accessory3)
        assert_eq!(e.head_mid, 30);
        e.apply_sprite_change(6, 3); // hair_color
        assert_eq!(e.hair_color, 3);
        e.apply_sprite_change(8, 2); // shield
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
        e.enter_attack(0.5);
        assert_eq!(e.state, EntityState::Attacking);

        e.update_state(0.6);
        assert_eq!(e.state, EntityState::ReadyFight);
        assert_eq!(e.action_index(), 4);

        // ReadyFight duration matches attack_motion_duration (0.5s)
        e.update_state(0.4);
        assert_eq!(e.state, EntityState::ReadyFight);
        e.update_state(0.2);
        assert_eq!(e.state, EntityState::Standing);
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
        e.enter_attack(0.5);
        e.update_state(0.6);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn weapon_dependent_attack_action() {
        // Swordsman(1) with spear → Attack3 (alternate)
        let mut e = Entity::new_player(1, 1, 1, 1, 0, 4, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Swordsman(1) with sword → Attack2 (primary weapon)
        let mut e = Entity::new_player(1, 1, 1, 1, 0, 2, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 10);

        // Assassin(12) with katar → Attack3 (alternate)
        let mut e = Entity::new_player(1, 12, 1, 1, 0, 16, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Archer(3) with bow → Attack2 (primary weapon)
        let mut e = Entity::new_player(1, 3, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 10);

        // Archer(3) with dagger → Attack3 (alternate)
        let mut e = Entity::new_player(1, 3, 1, 1, 0, 1, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Hunter(11) with bow → Attack3 (alternate, different from Archer!)
        let mut e = Entity::new_player(1, 11, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Bard(19) with bow → Attack3 (alternate)
        let mut e = Entity::new_player(1, 19, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Monk(15) unarmed → Attack3 (fist fighting)
        let mut e = Entity::new_player(1, 15, 1, 1, 0, 0, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Monk(15) with knuckle → Attack3
        let mut e = Entity::new_player(1, 15, 1, 1, 0, 12, 0, 0, 0, 0, 100, 100, 0);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 11);

        // Monk(15) with mace → Attack2 (primary)
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
        let mut e = make_entity(); // Player type
        e.enter_dead();
        assert_eq!(e.state, EntityState::Dead);
        assert!(!e.is_fading());
        assert!((e.alpha() - 1.0).abs() < f32::EPSILON);
        assert!(!e.should_remove());
    }
}
