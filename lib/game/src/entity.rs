use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::item::ItemType;
use models::enums::skill_enums::SkillEnum;
use models::enums::weapon::WeaponType;
use ragnarok_formats::act::{ActFile, SpriteActionType, SpriteAnimationState};

use crate::mob_info::MobInfo;
use crate::movement::MovementState;
use crate::scheduled_hit::ScheduledHitQueue;
use crate::sprite_path::weapon_view_id_to_type;

pub const AVG_ATTACKED_SPEED_SECS: f32 = 0.288;

const PICKUP_MOTION_FALLBACK_SECS: f32 = 0.5;

/// One swing of a multi-hit skill, replayed per hit. Longer than the 200 ms hit
/// spacing so consecutive hits run into each other instead of leaving the sprite
/// parked on the last frame.
const ATTACK_REPLAY_SECS: f32 = 0.3;

/// Attack-motion time (ms) that plays the swing at its native ACT frame delay.
/// Slower attacks scale up to a cap of 2×.
const AVG_ATTACK_MT_MS: f32 = 432.0;
const MAX_ATTACK_MT_MS: f32 = AVG_ATTACK_MT_MS * 2.0;

pub fn is_taekwon_job(job: u16) -> bool {
    (JobName::Taekwon.value()..=JobName::StarGladiatorUnion.value()).contains(&(job as usize))
}

/// Maps a server attack-motion time to the swing's animation speed factor.
/// A missing time (`<= 0`) plays at the native ACT speed. A bow is never
/// clamped.
pub fn attack_motion_factor(attack_mt_ms: i32, is_bow: bool) -> f32 {
    if attack_mt_ms <= 0 {
        return 1.0;
    }
    let mt = attack_mt_ms as f32;
    let clamped = if is_bow { mt } else { mt.min(MAX_ATTACK_MT_MS) };
    clamped / AVG_ATTACK_MT_MS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    Npc,
    Monster,
    Homunculus,
    Mercenary,
}

/// Actor class the server's job id puts an entity in. Selects the interaction
/// surface — name plate, HP/SP bar, hover cursor, click action, minimap marker —
/// where `EntityType` only selects the sprite path and action layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCategory {
    Player,
    Npc,
    WarpPoint,
    /// Ground unit spawned by a skill: Sanctuary, Fire Wall, traps, songs.
    Skill,
    Monster,
    Pet,
    Cart,
    Homunculus,
    Mercenary,
    Invisible,
}

impl EntityCategory {
    pub fn has_name_plate(self) -> bool {
        matches!(
            self,
            EntityCategory::Player
                | EntityCategory::Npc
                | EntityCategory::Monster
                | EntityCategory::Pet
                | EntityCategory::Homunculus
                | EntityCategory::Mercenary
        )
    }

    pub fn has_health_bar(self) -> bool {
        matches!(
            self,
            EntityCategory::Player
                | EntityCategory::Monster
                | EntityCategory::Pet
                | EntityCategory::Cart
                | EntityCategory::Homunculus
                | EntityCategory::Mercenary
        )
    }
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
    /// Play `start_frame..=end_frame` at native speed, then keep the end frame
    /// until `duration_ms` has elapsed.
    pub hold_last: bool,
    pub end_frame: Option<usize>,
    started: bool,
}

impl ForcedAnimation {
    pub fn new(action: usize, start_frame: usize, duration_ms: f32) -> Self {
        Self {
            action,
            start_frame,
            duration_ms,
            hold: false,
            hold_last: false,
            end_frame: None,
            started: false,
        }
    }

    pub fn play_then_hold(
        action: usize,
        start_frame: usize,
        end_frame: usize,
        duration_ms: f32,
    ) -> Self {
        Self {
            hold_last: true,
            end_frame: Some(end_frame),
            ..Self::new(action, start_frame, duration_ms)
        }
    }

    pub fn held(action: usize, frame: usize) -> Self {
        Self {
            action,
            start_frame: frame,
            duration_ms: 0.0,
            hold: true,
            hold_last: false,
            end_frame: None,
            started: false,
        }
    }

    /// A held frame that releases itself once `duration_ms` has elapsed.
    pub fn held_for(action: usize, frame: usize, duration_ms: f32) -> Self {
        Self {
            duration_ms,
            ..Self::held(action, frame)
        }
    }

    /// Counts a held frame down, reporting whether it still has time left.
    /// A hold set up with no duration never expires on its own.
    pub fn tick_hold(&mut self, dt: f32) -> bool {
        if self.duration_ms <= 0.0 {
            return true;
        }
        self.duration_ms -= dt * 1000.0;
        self.duration_ms > 0.0
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn mark_started(&mut self) {
        self.started = true;
    }
}

/// Attack action-group of a mercenary body (human 13-group layout). The
/// mercenary weapon sprites only carry frames in this group.
const MERCENARY_ATTACK_ACTION: usize = 10;

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

/// Degrees per step of the eight-way facing, and the offset the server's
/// direction sits at.
const DEGREES_PER_DIRECTION: f32 = 45.0;
const FACING_ORIGIN_DEGREES: f32 = 180.0;
const QUARTER_TURN_DEGREES: f32 = 90.0;
const FULL_TURN_DEGREES: f32 = 360.0;

/// Facing in degrees for an eight-way direction, normalised to `[0, 360)`.
///
/// The original reaches the same value two ways: straight from the direction
/// byte, which leaves it unwrapped, or from the angle between two positions,
/// which normalises. Anything that has moved or turned to face a target holds
/// the normalised form, and leaving it unwrapped puts every facing in the same
/// half of the drift buckets.
pub fn facing_degrees_for(direction: u8) -> f32 {
    (direction as f32 * DEGREES_PER_DIRECTION + FACING_ORIGIN_DEGREES) % FULL_TURN_DEGREES
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
    /// One pass of the emote's own action; see
    /// [`crate::emotion::emote_duration`].
    pub duration: f32,
}

impl EmotionState {
    /// Used when `emotion.act` cannot supply a duration.
    pub const FALLBACK_DURATION: f32 = 2.5;

    pub fn new(emotion_type: u8, duration: f32) -> Self {
        Self {
            emotion_type,
            elapsed: 0.0,
            duration,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.duration
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
    /// Raw right-hand look the server sent, kept because the weapon art is named
    /// after the item id when the archive has such a file.
    pub weapon_look: u16,
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
    /// Facing in degrees. A quarter-turn spin accumulates here rather than in
    /// `direction`, because the value is read by consumers whose thresholds do
    /// not wrap: past 360 means something different from the same angle modulo
    /// 360, and `direction` alone cannot express it. Write it through
    /// `set_facing` / `spin_quarter_turn`, never on its own.
    pub facing_degrees: f32,
    pub head_dir: u8,
    pub speed: u16,
    state: EntityState,
    /// Counts state entries. The sprite animation restarts whenever it differs
    /// from the entry it last started on, so re-entering the same state
    /// (a second swing) rewinds too.
    motion_epoch: u32,
    /// Expires a transient state only while the entity has no sprite to end it
    /// on the animation. A cast never expires on it unless armed by a local
    /// channel.
    pub state_timer: f32,
    /// Cast countdown, tracked apart from `state_timer` because a cast outlives
    /// the casting pose: Free Cast walks the caster away while the bar keeps
    /// filling.
    pub cast_remaining: f32,
    pub cast_total_duration: f32,
    /// Actor the running cast is aimed at; the caster turns to it periodically.
    pub cast_target_gid: Option<u32>,
    cast_reface_timer: f32,
    pub animation_duration: Option<f32>,
    /// Extra time the last frame of the queued one-shot is held before it
    /// reports finished.
    pub animation_hold_secs: f32,
    pub animation_start_frame: Option<usize>,
    pub attack_motion_factor: f32,
    /// A TaeKwon swing kicks with group 11 or 10, rolled once per swing.
    second_attack: bool,
    pub movement: MovementState,
    pub animation: SpriteAnimationState,
    pub emotion: Option<EmotionState>,
    pub chat_bubble: Option<ChatBubbleState>,
    pub active_skill: Option<SkillEnum>,
    pub skill_hit_count: u16,
    pub scheduled_hits: ScheduledHitQueue,
    pub pending_attack_replays: Vec<(f32, SkillEnum)>,
    pub fade: Option<EntityFade>,
    pub pending_death: bool,
    pub just_spawned: bool,
    pub effect_state: i32,
    pub body_state: i16,
    pub health_state: i16,
    /// Blade Stop / Root: darkens the body and freezes motion until released.
    pub rooted: bool,
    /// opt3 status bits, the only channel that carries the body-buff auras.
    pub opt3: i32,
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
    hover: HoverState,
}

/// The Star Gladiator Union hover: a slow bob whose height chases a target set
/// by the pose. See [`Entity::tick_hover`] and [`Entity::hover_lift_px`].
#[derive(Default, Clone, Copy)]
struct HoverState {
    /// Bob phase in degrees, stepping 2 per frame and wrapping at 360.
    move_deg: u16,
    /// Current height, stepping 1 every 15 frames toward the pose's target.
    now: i16,
    frame_accum: f32,
}

pub const JOB_STAR_GLADIATOR_UNION: u16 = 4048;

impl HoverState {
    fn tick_frame(&mut self, target: Option<i16>) {
        self.move_deg = (self.move_deg + 2) % 360;
        match target {
            Some(target) => {
                if self.move_deg % 30 == 0 {
                    self.now += (target - self.now).signum();
                }
            }
            None => self.now = (self.now - 1).max(0),
        }
    }

    fn lift_px(&self, camera_distance: f32) -> f32 {
        let bob = (self.now as f32) + (self.move_deg as f32).to_radians().sin() * 4.0;
        (bob * 400.0 / camera_distance).max(0.0)
    }
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
            weapon_look: weapon,
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
            facing_degrees: facing_degrees_for(direction),
            head_dir: 0,
            speed,
            state: EntityState::Standing,
            motion_epoch: 0,
            state_timer: 0.0,
            cast_remaining: 0.0,
            cast_total_duration: 0.0,
            cast_target_gid: None,
            cast_reface_timer: 0.0,
            animation_duration: None,
            animation_hold_secs: 0.0,
            animation_start_frame: None,
            attack_motion_factor: 1.0,
            second_attack: false,
            movement,
            animation: SpriteAnimationState::new(direction),
            emotion: None,
            chat_bubble: None,
            active_skill: None,
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
            opt3: 0,
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
            hover: HoverState::default(),
        }
    }

    /// Advances the Union hover. `warm` marks a red body-hit flash, which the
    /// original counts as an upright pose. Other jobs never leave the ground, so
    /// a Star Gladiator who transforms back sinks instead of snapping down.
    pub fn tick_hover(&mut self, delta: f32, warm: bool) {
        if self.job != JOB_STAR_GLADIATOR_UNION && self.hover.now == 0 {
            return;
        }
        let target = if self.job != JOB_STAR_GLADIATOR_UNION {
            None
        } else {
            match self.state {
                EntityState::Standing | EntityState::Moving => Some(20),
                EntityState::Sitting => Some(100),
                _ if warm => Some(20),
                _ => None,
            }
        };
        self.hover.frame_accum += delta * 60.0;
        while self.hover.frame_accum >= 1.0 {
            self.hover.frame_accum -= 1.0;
            self.hover.tick_frame(target);
        }
    }

    pub fn hover_lift_px(&self, camera_distance: f32) -> f32 {
        if self.job != JOB_STAR_GLADIATOR_UNION {
            return 0.0;
        }
        self.hover.lift_px(camera_distance)
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

    /// Face an eight-way direction, resetting any accumulated spin.
    pub fn set_facing(&mut self, direction: u8) {
        self.direction = direction;
        self.facing_degrees = facing_degrees_for(direction);
    }

    /// Face the cell direction `target`, letting the head cover the last
    /// eighth: the body never ends more than one step off `target`, and a
    /// second look to the same side straightens the head and turns the body.
    pub fn look_at_direction(&mut self, target: u8) {
        match (self.direction + 8 - target) % 8 {
            0 | 4 => {
                self.set_facing(target);
                self.head_dir = 0;
            }
            1 if self.head_dir != 1 => self.head_dir = 1,
            7 if self.head_dir != 2 => self.head_dir = 2,
            1 | 7 => {
                self.set_facing(target);
                self.head_dir = 0;
            }
            2 | 3 => {
                self.set_facing((target + 1) % 8);
                self.head_dir = 1;
            }
            _ => {
                self.set_facing((target + 7) % 8);
                self.head_dir = 2;
            }
        }
    }

    /// Turn a quarter clockwise, as a multi-hit skill does to its target on
    /// every blow. The angle wraps only once it passes a full turn, so a facing
    /// that lands exactly on 360 keeps that value rather than folding to zero.
    pub fn spin_quarter_turn(&mut self) {
        self.facing_degrees += QUARTER_TURN_DEGREES;
        if self.facing_degrees > FULL_TURN_DEGREES {
            self.facing_degrees -= FULL_TURN_DEGREES;
        }
        let steps = (QUARTER_TURN_DEGREES / DEGREES_PER_DIRECTION) as u8;
        self.direction = (self.direction + steps) % 8;
    }

    pub fn state(&self) -> EntityState {
        self.state
    }

    pub fn motion_epoch(&self) -> u32 {
        self.motion_epoch
    }

    /// Changes state, restarting the animation only when the state differs.
    pub fn set_state(&mut self, state: EntityState) {
        if self.state != state {
            self.enter(state);
        }
    }

    fn enter(&mut self, state: EntityState) {
        self.state = state;
        self.motion_epoch = self.motion_epoch.wrapping_add(1);
    }

    fn tick_state_timer(&mut self, dt: f32) -> bool {
        self.state_timer -= dt;
        if self.state_timer <= 0.0 {
            self.state_timer = 0.0;
            return true;
        }
        false
    }

    /// Whether the sprite animation started for the current state entry has
    /// played through.
    fn animation_played_through(&self) -> bool {
        self.animation.entry() == self.motion_epoch && self.animation.is_finished()
    }

    /// A composite actor's static groups are never ticked, so a transient state
    /// resolving to one would never report finished.
    fn motion_never_advances(&self) -> bool {
        matches!(self.entity_type, EntityType::Player | EntityType::Mercenary)
            && SpriteActionType::from_index(self.action_index()).is_some_and(|a| !a.is_animated())
    }

    fn end_transient(&mut self) {
        self.state_timer = 0.0;
        self.active_skill = None;
        let combat_actor = matches!(self.entity_type, EntityType::Player | EntityType::Mercenary);
        let ready = combat_actor
            && match self.state {
                EntityState::Attacking => !self.is_second_attack(),
                EntityState::Hurt | EntityState::Casting => true,
                _ => false,
            };
        self.set_state(if ready {
            EntityState::ReadyFight
        } else {
            EntityState::Standing
        });
    }

    /// `sprite_loaded` hands the transient states to the animation: with a
    /// sprite they end when it reports finished, without one they expire on
    /// `state_timer`.
    pub fn update_state(&mut self, dt: f32, sprite_loaded: bool) {
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

        if self.cast_remaining > 0.0 {
            self.cast_remaining -= dt;
            if self.cast_remaining <= 0.0 {
                self.clear_cast();
            }
        }

        if self.state == EntityState::Dead {
            return;
        }
        if self.pending_death && self.scheduled_hits.is_empty() {
            self.enter_dead();
            return;
        }
        match self.state {
            EntityState::Attacking
            | EntityState::SkillExec
            | EntityState::Hurt
            | EntityState::Pickup => {
                let ended = if sprite_loaded {
                    self.animation_played_through() || self.motion_never_advances()
                } else {
                    self.tick_state_timer(dt)
                };
                if ended {
                    self.end_transient();
                }
                return;
            }
            EntityState::Casting => {
                if self.state_timer > 0.0 && self.tick_state_timer(dt) {
                    self.end_transient();
                    return;
                }
                if !self.movement.is_moving() {
                    return;
                }
            }
            EntityState::Sitting => return,
            _ => {}
        }
        let next = if self.movement.is_moving() {
            self.head_dir = 0;
            EntityState::Moving
        } else if self.state == EntityState::ReadyFight {
            EntityState::ReadyFight
        } else {
            EntityState::Standing
        };
        self.set_state(next);
    }

    pub fn is_move_locked(&self) -> bool {
        matches!(self.state, EntityState::Pickup | EntityState::Attacking)
    }

    pub fn begin_move(&mut self, path: Vec<crate::path::PathNode>, now: f32) {
        self.movement.start_move(path, now);
        self.state_timer = 0.0;
        if matches!(
            self.state,
            EntityState::Attacking
                | EntityState::SkillExec
                | EntityState::Hurt
                | EntityState::Pickup
        ) {
            self.set_state(EntityState::Moving);
        }
    }

    /// Action group the damage motion plays from.
    pub fn hurt_action_group(&self) -> usize {
        match self.entity_type {
            EntityType::Player | EntityType::Mercenary => SpriteActionType::Hurt as usize,
            EntityType::Monster | EntityType::Npc | EntityType::Homunculus => 3,
        }
    }

    /// `natural_secs` is the damage action's own duration. The server's damage
    /// motion time scales it: below one average the motion is played faster,
    /// above it the motion plays once and the last frame is held for the rest.
    /// Only a player keeps swinging through a blow; everything else flinches
    /// mid-attack.
    pub fn enter_hurt(&mut self, damage_motion_secs: f32, natural_secs: Option<f32>) {
        if self.state == EntityState::Dead
            || (self.state == EntityState::Attacking && self.entity_type == EntityType::Player)
        {
            return;
        }
        if damage_motion_secs <= 0.0 {
            return;
        }
        let natural = natural_secs
            .filter(|s| *s > 0.0)
            .unwrap_or(AVG_ATTACKED_SPEED_SECS);
        let factor = damage_motion_secs / AVG_ATTACKED_SPEED_SECS;
        self.movement.stop();
        self.enter(EntityState::Hurt);
        self.state_timer = natural * factor;
        self.animation_duration = Some(natural * factor.min(1.0));
        self.animation_hold_secs = natural * (factor - 1.0).max(0.0);
    }

    /// Picks which attack group the next swing uses. Must run before the swing's
    /// duration is measured, because that reads the group.
    pub fn roll_attack_variant(&mut self, rand: u32) {
        if is_taekwon_job(self.job) {
            self.second_attack = rand % 10 < 7;
        }
    }

    /// Whether the player swings with the second attack group (11).
    pub fn is_second_attack(&self) -> bool {
        self.entity_type == EntityType::Player && self.attack_action_for_weapon() == 11
    }

    /// Frame of the player's swing at which the blow connects.
    pub fn player_attack_keyframe(&self) -> f32 {
        let job = JobName::try_from_value(self.job as usize).ok();
        if self.is_second_attack() {
            let dual_wield = matches!(
                self.weapon,
                Some(
                    WeaponType::Katar
                        | WeaponType::DoubleDd
                        | WeaponType::DoubleSs
                        | WeaponType::DoubleAa
                        | WeaponType::DoubleDs
                        | WeaponType::DoubleDa
                        | WeaponType::DoubleSa
                )
            );
            return match job {
                Some(JobName::Novice | JobName::SuperNovice | JobName::SuperBaby)
                    if self.sex == 1 =>
                {
                    5.85
                }
                Some(JobName::Assassin | JobName::AssassinCross | JobName::BabyAssassin)
                    if dual_wield =>
                {
                    3.0
                }
                _ => 6.0,
            };
        }
        match job {
            Some(JobName::Merchant) => 5.85,
            Some(JobName::Thief) => 5.75,
            _ => 6.0,
        }
    }

    pub fn enter_attack(&mut self, duration_secs: f32, motion_factor: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        self.enter(EntityState::Attacking);
        self.state_timer = duration_secs;
        self.attack_motion_factor = motion_factor;
        self.animation_duration = Some(duration_secs);
    }

    pub fn enter_attack_replay(&mut self, skill: SkillEnum) {
        if self.state == EntityState::Dead {
            return;
        }
        self.enter(EntityState::SkillExec);
        self.state_timer = ATTACK_REPLAY_SECS;
        self.active_skill = Some(skill);
        self.animation_duration = Some(ATTACK_REPLAY_SECS);
    }

    /// A transient one-shot (hurt, death, skill, pickup) holds its last frame
    /// once it has played through — unless a fresh motion is already queued, as
    /// a multi-hit skill queues one per hit. `pose_action` is the action group
    /// the state resolves to, which is what `anim_action` was started from.
    pub fn holds_last_frame(
        &self,
        pose_action: usize,
        anim_action: usize,
        anim_finished: bool,
    ) -> bool {
        self.animation_duration.is_none()
            && matches!(
                self.state,
                EntityState::Dead
                    | EntityState::Hurt
                    | EntityState::SkillExec
                    | EntityState::Pickup
            )
            && anim_action == pose_action
            && anim_finished
    }

    pub fn enter_casting(&mut self, duration_secs: f32, skill: SkillEnum) {
        if self.state == EntityState::Dead {
            return;
        }
        self.movement.stop();
        self.enter(EntityState::Casting);
        self.state_timer = 0.0;
        self.cast_target_gid = None;
        self.cast_reface_timer = 0.0;
        self.cast_remaining = duration_secs;
        self.cast_total_duration = duration_secs;
        self.active_skill = Some(skill);
    }

    /// The target to turn toward, once every 34 frame units of the cast.
    pub fn cast_reface_due(&mut self, dt: f32) -> Option<u32> {
        const REFACE_SECS: f32 = 34.0 * ragnarok_formats::act::FRAME_DELAY_UNIT_MS / 1000.0;
        if self.state != EntityState::Casting {
            return None;
        }
        let target = self.cast_target_gid?;
        self.cast_reface_timer += dt;
        if self.cast_reface_timer < REFACE_SECS {
            return None;
        }
        self.cast_reface_timer -= REFACE_SECS;
        Some(target)
    }

    pub fn clear_cast(&mut self) {
        self.cast_remaining = 0.0;
        self.cast_total_duration = 0.0;
    }

    pub fn cast_progress(&self) -> Option<f32> {
        (self.cast_total_duration > 0.0)
            .then(|| 1.0 - (self.cast_remaining / self.cast_total_duration))
    }

    pub fn enter_skill_exec(&mut self, duration_secs: f32, skill: SkillEnum, hit_count: u16) {
        if self.state == EntityState::Dead {
            return;
        }
        self.enter(EntityState::SkillExec);
        self.state_timer = duration_secs;
        self.animation_duration = Some(duration_secs);
        self.active_skill = Some(skill);
        self.skill_hit_count = hit_count;
        if crate::skill_action::skill_motion_type(skill)
            == crate::skill_action::SkillMotionType::Skill
            && let Some(overlay) = self.skill_exec_overlay()
        {
            self.forced_animation = Some(overlay);
        }
    }

    /// Action group a skill's casting motion plays from.
    fn skill_exec_group(&self) -> usize {
        match JobName::try_from_value(self.job as usize) {
            Ok(
                JobName::Bard
                | JobName::Dancer
                | JobName::Crusader
                | JobName::BabyBard
                | JobName::BabyDancer
                | JobName::BabyCrusader
                | JobName::Taekwon
                | JobName::StarGladiator
                | JobName::StarGladiatorUnion
                | JobName::Gunslinger,
            ) => SpriteActionType::ReadyFight as usize,
            Ok(JobName::Monk | JobName::Champion | JobName::BabyMonk) if self.sex == 0 => {
                SpriteActionType::ReadyFight as usize
            }
            _ => SpriteActionType::Skill as usize,
        }
    }

    /// Frames of the skill group some jobs show over their casting motion.
    fn skill_exec_overlay(&self) -> Option<ForcedAnimation> {
        let skill = SpriteActionType::Skill as usize;
        match JobName::try_from_value(self.job as usize) {
            Ok(JobName::Gunslinger) => Some(ForcedAnimation::play_then_hold(skill, 0, 3, 1000.0)),
            Ok(JobName::Ninja) => Some(ForcedAnimation::play_then_hold(skill, 2, 5, 1000.0)),
            Ok(JobName::Clown | JobName::Gypsy | JobName::Paladin) => {
                Some(ForcedAnimation::held_for(skill, 0, 400.0))
            }
            Ok(JobName::Monk | JobName::Champion | JobName::BabyMonk) if self.sex != 0 => {
                Some(ForcedAnimation::held_for(skill, 0, 400.0))
            }
            _ => None,
        }
    }

    pub fn enter_dead(&mut self) {
        self.enter(EntityState::Dead);
        self.state_timer = 0.0;
        self.clear_cast();
        self.forced_animation = None;
        self.movement.stop();
        self.pending_death = false;
        self.target_gid = None;
    }

    pub fn revive(&mut self) {
        self.set_state(EntityState::Standing);
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

    pub fn enter_pickup(&mut self, motion_secs: Option<f32>) {
        if self.state == EntityState::Dead {
            return;
        }
        self.enter(EntityState::Pickup);
        self.state_timer = motion_secs.unwrap_or(PICKUP_MOTION_FALLBACK_SECS);
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
            // An off-hand weapon has no slot of its own: the combined dual-wield
            // look needs both hands at once, which only the server's LOOK_WEAPON
            // change carries. Routing it to the weapon slot here would blank the
            // main hand.
            if item_type == Some(ItemType::Weapon) {
                None
            } else {
                Some(8)
            }
        } else {
            None
        }
    }

    pub fn category(&self) -> EntityCategory {
        let category = crate::sprite_path::entity_category_from_job(self.job);
        if self.is_pet && category == EntityCategory::Monster {
            EntityCategory::Pet
        } else {
            category
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
                EntityState::Attacking | EntityState::SkillExec => MERCENARY_ATTACK_ACTION,
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
            EntityType::Mercenary => MERCENARY_ATTACK_ACTION,
            EntityType::Monster | EntityType::Npc | EntityType::Homunculus => 2,
        }
    }

    pub fn skill_exec_start_frame(&self) -> usize {
        use crate::skill_action::{SkillMotionType, skill_motion_type, skill_pose};
        match self.active_skill {
            Some(skill) => match skill_motion_type(skill) {
                SkillMotionType::Skill | SkillMotionType::Sing | SkillMotionType::Dance => 1,
                SkillMotionType::Pose => skill_pose(skill).map_or(0, |p| p.frame),
                _ => 0,
            },
            None => 1,
        }
    }

    fn skill_exec_action_index(&self) -> usize {
        use crate::skill_action::{SkillMotionType, skill_motion_type, skill_pose};
        let skill = match self.active_skill {
            Some(skill) => skill,
            None => return 12,
        };
        match skill_motion_type(skill) {
            SkillMotionType::Pose => skill_pose(skill).map_or(0, |p| p.action),
            SkillMotionType::Attack => self.attack_action_for_weapon(),
            SkillMotionType::Throw => 5,
            SkillMotionType::Attack2 => 10,
            SkillMotionType::Pickup => 3,
            SkillMotionType::Skill => self.skill_exec_group(),
            SkillMotionType::Sing | SkillMotionType::Dance => 12,
            SkillMotionType::Stand => 0,
            SkillMotionType::Walk => 1,
        }
    }

    fn attack_action_for_weapon(&self) -> usize {
        let job = match JobName::try_from_value(self.job as usize) {
            Ok(j) => j,
            Err(_) => return 5,
        };
        if is_taekwon_job(self.job) {
            return if self.second_attack { 11 } else { 10 };
        }
        let weapon = match self.weapon {
            Some(ref w) => w,
            None => {
                return match job {
                    JobName::Monk
                    | JobName::Champion
                    | JobName::BabyMonk
                    | JobName::Archer
                    | JobName::ArcherHigh
                    | JobName::BabyArcher => 11,
                    _ => 10,
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
                WeaponType::Rifle
                | WeaponType::Gatling
                | WeaponType::Shotgun
                | WeaponType::Grenade => 11,
                _ => 10,
            },
            JobName::Ninja => match weapon {
                WeaponType::Shuriken => 11,
                _ => 10,
            },
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

    #[test]
    fn union_star_gladiator_floats_higher_while_seated_and_only_that_job_leaves_the_ground() {
        let mut e = make_entity();
        e.job = JOB_STAR_GLADIATOR_UNION;
        assert_eq!(e.hover_lift_px(200.0), 0.0, "starts on the ground");

        e.set_state(EntityState::Standing);
        e.tick_hover(10.0, false);
        let standing = e.hover_lift_px(200.0);
        assert!(standing > 0.0);

        e.set_state(EntityState::Sitting);
        e.tick_hover(20.0, false);
        assert!(e.hover_lift_px(200.0) > standing, "sitting floats higher");

        // Zooming out shrinks the on-screen lift.
        assert!(e.hover_lift_px(800.0) < e.hover_lift_px(200.0));

        let mut plain = make_entity();
        plain.set_state(EntityState::Standing);
        plain.tick_hover(10.0, false);
        assert_eq!(plain.hover_lift_px(200.0), 0.0);
    }

    #[test]
    fn hover_decays_when_the_pose_is_neither_upright_nor_seated_unless_flashing_red() {
        let mut e = make_entity();
        e.job = JOB_STAR_GLADIATOR_UNION;
        e.set_state(EntityState::Standing);
        e.tick_hover(10.0, false);
        let airborne = e.hover_lift_px(200.0);

        let mut warm = make_entity();
        warm.job = e.job;
        warm.set_state(EntityState::Standing);
        warm.tick_hover(10.0, false);

        e.set_state(EntityState::Dead);
        warm.set_state(EntityState::Dead);
        e.tick_hover(2.0, false);
        warm.tick_hover(2.0, true);
        assert!(e.hover_lift_px(200.0) < airborne, "sinks once knocked down");
        assert!(
            warm.hover_lift_px(200.0) > e.hover_lift_px(200.0),
            "a red hit holds it up"
        );
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
        };
        assert_eq!(merc(6017).weapon, Some(WeaponType::Bow)); // archer
        assert_eq!(merc(6027).weapon, Some(WeaponType::Spear1H)); // lancer
        assert_eq!(merc(6037).weapon, Some(WeaponType::Sword1H)); // swordman

        for job in [6017, 6027, 6037] {
            assert_eq!(merc(job).attack_action_index(), 10, "merc {job}");
        }
    }

    #[test]
    fn gunslinger_swings_with_the_group_its_gun_sprite_is_drawn_in() {
        let mut e = make_entity();
        e.job = JobName::Gunslinger.value() as u16;

        e.weapon = Some(WeaponType::Revolver);
        assert_eq!(e.attack_action_index(), 10);

        for gun in [
            WeaponType::Rifle,
            WeaponType::Gatling,
            WeaponType::Shotgun,
            WeaponType::Grenade,
        ] {
            e.weapon = Some(gun);
            assert_eq!(e.attack_action_index(), 11, "{gun:?}");
        }
    }

    #[test]
    fn taekwon_swings_pick_a_kick_per_swing_and_hold_it_through_the_swing() {
        assert!(
            JobName::try_from_value(JOB_STAR_GLADIATOR_UNION as usize).is_ok(),
            "the server crate must know job 4048"
        );
        let mut groups = std::collections::HashSet::new();
        for job in [
            JobName::Taekwon,
            JobName::StarGladiator,
            JobName::StarGladiatorUnion,
        ] {
            let mut e = make_entity();
            e.job = job.value() as u16;
            for roll in 0..20u32 {
                e.roll_attack_variant(roll);
                e.enter_attack(0.5, 1.0);
                let first = e.action_index();
                for _ in 0..5 {
                    assert_eq!(e.action_index(), first, "{job:?} roll {roll}");
                }
                groups.insert(first);
                e.update_state(1.0, false);
            }
        }
        assert_eq!(groups, [10, 11].into_iter().collect());

        let mut knight = make_entity();
        knight.job = JobName::Knight.value() as u16;
        knight.roll_attack_variant(0);
        knight.enter_attack(0.5, 1.0);
        assert_eq!(knight.action_index(), 10, "the roll is TaeKwon-only");
    }

    #[test]
    fn skill_motion_group_and_overlay_follow_job_and_sex() {
        let exec = |job: JobName, sex: u8| {
            let mut e = make_entity();
            e.job = job.value() as u16;
            e.sex = sex;
            e.enter_skill_exec(0.5, SkillEnum::AlHeal, 1);
            (e.action_index(), e.forced_animation)
        };
        let skill = SpriteActionType::Skill as usize;

        assert_eq!(exec(JobName::Knight, 1), (skill, None));
        assert_eq!(exec(JobName::Bard, 1).0, 4);
        assert_eq!(exec(JobName::BabyCrusader, 0).0, 4);
        assert_eq!(exec(JobName::StarGladiatorUnion, 1).0, 4);
        assert_eq!(exec(JobName::Monk, 0), (4, None), "female monk");
        assert_eq!(
            exec(JobName::Monk, 1),
            (skill, Some(ForcedAnimation::held_for(skill, 0, 400.0))),
            "male monk"
        );
        assert_eq!(
            exec(JobName::Paladin, 1).0,
            skill,
            "only the baby forms join group 4"
        );
        assert_eq!(
            exec(JobName::Gunslinger, 1),
            (
                4,
                Some(ForcedAnimation::play_then_hold(skill, 0, 3, 1000.0))
            )
        );
        assert_eq!(
            exec(JobName::Ninja, 1),
            (
                skill,
                Some(ForcedAnimation::play_then_hold(skill, 2, 5, 1000.0))
            )
        );

        let mut song = make_entity();
        song.job = JobName::Bard.value() as u16;
        song.enter_skill_exec(0.5, SkillEnum::BaPoembragi, 1);
        assert_eq!(song.action_index(), skill, "a song is not a skill motion");
        assert!(song.forced_animation.is_none());
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
        e.set_state(EntityState::Moving);
        assert_eq!(e.action_index(), 1);
        e.set_state(EntityState::Sitting);
        assert_eq!(e.action_index(), 2);
        e.set_state(EntityState::Pickup);
        assert_eq!(e.action_index(), 3);
        e.set_state(EntityState::ReadyFight);
        assert_eq!(e.action_index(), 4);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 10);
        e.set_state(EntityState::Hurt);
        assert_eq!(e.action_index(), 6);
        e.set_state(EntityState::Dead);
        assert_eq!(e.action_index(), 8);
        e.set_state(EntityState::Casting);
        assert_eq!(e.action_index(), 12);
        e.set_state(EntityState::SkillExec);
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
        e.set_state(EntityState::Moving);
        assert_eq!(e.action_index(), 1);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 2);
        e.set_state(EntityState::SkillExec);
        assert_eq!(e.action_index(), 2);
        e.set_state(EntityState::Casting);
        assert_eq!(e.action_index(), 2);
        e.set_state(EntityState::Hurt);
        assert_eq!(e.action_index(), 3);
        e.set_state(EntityState::Dead);
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
        e.set_state(EntityState::Standing);
        let mut hit = ScheduledHit::single(50, Some(SkillEnum::MgFirebolt), false);
        hit.fire_at = 10.0;
        e.scheduled_hits.push(hit);

        e.request_pending_death();
        e.update_state(0.1, false);
        assert_eq!(
            e.state,
            EntityState::Standing,
            "stays alive while the bolt flies"
        );
        assert!(e.pending_death);

        e.scheduled_hits.drain_ready(10.0);
        e.update_state(0.1, false);
        assert_eq!(
            e.state,
            EntityState::Dead,
            "dies once the delayed hit has landed"
        );
        assert!(!e.pending_death);
    }

    #[test]
    fn looking_at_a_cell_turns_the_head_before_the_body() {
        let mut e = make_entity();
        e.set_state(EntityState::Sitting);
        e.set_facing(2);

        e.look_at_direction(1);
        assert_eq!((e.direction, e.head_dir), (2, 1));

        e.look_at_direction(1);
        assert_eq!((e.direction, e.head_dir), (1, 0));

        e.look_at_direction(4);
        assert_eq!((e.direction, e.head_dir), (3, 2));

        e.look_at_direction(4);
        assert_eq!((e.direction, e.head_dir), (4, 0));
    }

    #[test]
    fn update_state_preserves_sitting() {
        let mut e = make_entity();
        e.set_state(EntityState::Sitting);
        e.update_state(0.016, false);
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

        e.update_state(0.016, false);

        assert_eq!(e.state, EntityState::Moving);
        assert_eq!(e.head_dir, 0);
    }

    #[test]
    fn hurt_cancels_movement_without_locking_it_and_recovers_to_the_combat_stance() {
        let mut e = make_entity();
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        e.movement.start_move(path, 0.0);
        assert!(e.movement.is_moving());

        e.enter_hurt(0.5, None);
        assert_eq!(e.state, EntityState::Hurt);
        assert!(!e.movement.is_moving());
        assert!(
            !e.is_move_locked(),
            "the damage motion never swallows a move request"
        );

        e.update_state(0.3, false);
        assert_eq!(e.state, EntityState::Hurt);

        e.update_state(0.3, false);
        assert_eq!(e.state, EntityState::ReadyFight);
        assert_eq!(e.action_index(), 4);
    }

    #[test]
    fn pickup_motion_locks_movement_until_it_ends() {
        let mut e = make_entity();
        e.enter_pickup(Some(0.5));
        assert_eq!(e.state, EntityState::Pickup);
        assert!(e.is_move_locked());

        e.update_state(0.6, false);
        assert_eq!(e.state, EntityState::Standing);
        assert!(!e.is_move_locked());
    }

    /// One frame of the client's animation pass for a one-shot state.
    fn start_one_shot(e: &mut Entity) {
        use ragnarok_formats::act::MotionType;
        let new_entry = e.animation.mark_entry(e.motion_epoch());
        e.animation
            .set_action(e.action_index(), MotionType::OneShot);
        if new_entry {
            e.animation.restart_motion();
        }
    }

    #[test]
    fn each_pickup_replays_the_motion_and_the_pose_ends_with_it() {
        let act = make_body_act(3, &[]);
        let play_out = |e: &mut Entity| {
            start_one_shot(e);
            for _ in 0..10 {
                e.animation.update(0.05, &act, 0);
            }
        };
        let mut e = make_entity();

        e.enter_pickup(Some(0.3));
        play_out(&mut e);
        assert!(e.animation.is_finished());

        e.enter_pickup(Some(0.3));
        start_one_shot(&mut e);
        assert_eq!(e.animation.motion_index(), 0);
        e.update_state(0.016, true);
        assert_eq!(e.state, EntityState::Pickup);
        assert!(e.is_move_locked());

        play_out(&mut e);
        e.update_state(0.016, true);
        assert_eq!(e.state, EntityState::Standing);
        assert!(!e.is_move_locked());
    }

    #[test]
    fn a_new_cast_rewinds_the_skill_group_left_on_its_last_frame() {
        use ragnarok_formats::act::MotionType;
        let act = make_body_act(4, &[]);
        let mut e = make_entity();

        e.enter_skill_exec(0.2, SkillEnum::AlHeal, 1);
        assert!(e.animation.mark_entry(e.motion_epoch()));
        e.animation.play(
            e.action_index(),
            e.animation_duration.take().unwrap() * 1000.0,
            1,
        );
        for _ in 0..10 {
            e.animation.update(0.05, &act, 0);
        }
        assert!(e.animation.is_finished());
        assert_eq!(e.animation.motion_index(), 3);
        e.update_state(0.016, true);
        assert_eq!(e.state, EntityState::Standing);

        e.enter_casting(2.0, SkillEnum::MgFirebolt);
        assert_eq!(e.action_index(), 12, "same group as the skill it follows");
        assert!(e.animation.mark_entry(e.motion_epoch()));
        e.animation.set_action(12, MotionType::Static);
        e.animation.restart_motion();
        assert_eq!(e.animation.motion_index(), 0);

        for _ in 0..10 {
            e.update_state(1.0, true);
        }
        assert_eq!(
            e.state,
            EntityState::Casting,
            "a cast only ends on a server packet"
        );
        assert!(
            !e.animation.mark_entry(e.motion_epoch()),
            "re-deriving the same state must not restart it"
        );
    }

    #[test]
    fn a_held_stance_ends_its_state_when_the_hold_expires_with_nothing_stale_after() {
        let act = make_body_act(3, &[]);
        let mut e = make_entity();
        e.enter_skill_exec(2.0, SkillEnum::TkReadystorm, 1);
        e.animation.mark_entry(e.motion_epoch());
        e.update_state(3.0, true);
        assert_eq!(
            e.state,
            EntityState::SkillExec,
            "no timer while a sprite is loaded"
        );

        e.animation.finish();
        e.update_state(0.016, true);
        assert_eq!(e.state, EntityState::Standing);

        start_one_shot(&mut e);
        e.animation.update(0.05, &act, 0);
        assert_eq!(e.animation.action(), 0);
        assert!(!e.animation.is_finished());
    }

    #[test]
    fn a_stand_motion_skill_does_not_latch_the_actor_in_skill_exec() {
        let mut e = make_entity();
        e.enter_skill_exec(1.0, SkillEnum::BdAdaptation, 1);
        assert_eq!(e.action_index(), SpriteActionType::Idle as usize);

        e.animation.mark_entry(e.motion_epoch());
        e.update_state(0.016, true);

        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn a_swing_ends_on_its_animation_with_a_sprite_and_on_the_timer_without() {
        let act = make_body_act(3, &[]);
        let mut with_sprite = make_entity();
        with_sprite.enter_attack(5.0, 1.0);
        with_sprite.animation.mark_entry(with_sprite.motion_epoch());
        with_sprite
            .animation
            .play_attack(with_sprite.action_index(), 1.0, 0);
        with_sprite.update_state(1.0, true);
        assert_eq!(with_sprite.state, EntityState::Attacking);
        for _ in 0..10 {
            with_sprite.animation.update(0.05, &act, 0);
        }
        with_sprite.update_state(0.016, true);
        assert_eq!(with_sprite.state, EntityState::ReadyFight);

        let mut without = make_entity();
        without.enter_attack(0.5, 1.0);
        without.update_state(0.3, false);
        assert_eq!(without.state, EntityState::Attacking);
        without.update_state(0.3, false);
        assert_eq!(without.state, EntityState::ReadyFight);

        let mut stale = make_entity();
        stale.enter_hurt(0.2, Some(0.2));
        stale.animation.mark_entry(stale.motion_epoch());
        stale.animation.play(stale.action_index(), 200.0, 0);
        for _ in 0..10 {
            stale.animation.update(0.05, &act, 0);
        }
        stale.enter_attack(0.5, 1.0);
        stale.update_state(0.016, true);
        assert_eq!(
            stale.state,
            EntityState::Attacking,
            "the previous entry's finished flag must not end the new state"
        );
    }

    #[test]
    fn damage_motion_time_scales_the_action_and_a_slow_blow_holds_the_pose() {
        let natural = Some(0.4);

        let mut fast = make_entity();
        fast.enter_hurt(AVG_ATTACKED_SPEED_SECS / 2.0, natural);
        assert_eq!(fast.state, EntityState::Hurt);
        assert_eq!(fast.state_timer, 0.2);
        assert_eq!(
            fast.animation_duration,
            Some(0.2),
            "a quick blow plays the action faster"
        );

        let mut slow = make_entity();
        slow.enter_hurt(AVG_ATTACKED_SPEED_SECS * 2.0, natural);
        assert_eq!(slow.state_timer, 0.8);
        assert_eq!(
            slow.animation_duration,
            Some(0.4),
            "a slow blow plays the action once and holds the last frame"
        );
    }

    #[test]
    fn dead_blocks_all_transitions() {
        let mut e = make_entity();
        e.enter_dead();
        assert_eq!(e.state, EntityState::Dead);

        e.enter_hurt(1.0, None);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_attack(1.0, 1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_skill_exec(1.0, SkillEnum::SmBash, 1);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_pickup(Some(1.0));
        assert_eq!(e.state, EntityState::Dead);

        e.enter_casting(1.0, SkillEnum::SmBash);
        assert_eq!(e.state, EntityState::Dead);

        e.update_state(1.0, false);
        assert_eq!(e.state, EntityState::Dead);
    }

    #[test]
    fn attack_motion_factor_is_native_at_average_and_capped_when_slow_except_for_a_bow() {
        assert_eq!(attack_motion_factor(432, false), 1.0);
        assert_eq!(attack_motion_factor(216, false), 0.5);
        assert_eq!(
            attack_motion_factor(5000, false),
            2.0,
            "slow attacks cap at 2x native"
        );
        assert_eq!(
            attack_motion_factor(1296, true),
            3.0,
            "a bow is never clamped"
        );
        assert_eq!(
            attack_motion_factor(0, false),
            1.0,
            "missing time plays at native speed"
        );
    }

    #[test]
    fn player_hit_keyframe_is_gated_on_the_second_attack_first() {
        let keyframe = |job: JobName, sex: u8, weapon: Option<WeaponType>| {
            let mut e = make_entity();
            e.job = job.value() as u16;
            e.sex = sex;
            e.weapon = weapon;
            e.player_attack_keyframe()
        };
        assert_eq!(keyframe(JobName::Thief, 1, Some(WeaponType::Dagger)), 5.75);
        assert_eq!(
            keyframe(JobName::Merchant, 1, Some(WeaponType::Axe1H)),
            5.85
        );
        assert_eq!(
            keyframe(JobName::Thief, 1, Some(WeaponType::Bow)),
            6.0,
            "a second attack skips the first-attack rows"
        );
        assert_eq!(keyframe(JobName::Assassin, 1, Some(WeaponType::Katar)), 3.0);
        assert_eq!(
            keyframe(JobName::Assassin, 1, Some(WeaponType::DoubleDd)),
            3.0
        );
        assert_eq!(
            keyframe(JobName::Assassin, 1, Some(WeaponType::Dagger)),
            6.0
        );
        assert_eq!(
            keyframe(JobName::Novice, 1, Some(WeaponType::Sword1H)),
            5.85
        );
        assert_eq!(keyframe(JobName::Novice, 0, Some(WeaponType::Dagger)), 6.0);
        assert_eq!(keyframe(JobName::Knight, 1, Some(WeaponType::Spear1H)), 6.0);
    }

    #[test]
    fn only_a_player_swings_through_a_blow() {
        let mut player = make_entity();
        player.enter_attack(1.0, 1.0);
        player.enter_hurt(0.5, None);
        assert_eq!(player.state, EntityState::Attacking);

        let mut monster = make_entity();
        monster.entity_type = EntityType::Monster;
        monster.enter_attack(1.0, 1.0);
        monster.enter_hurt(0.5, None);
        assert_eq!(monster.state, EntityState::Hurt);

        let mut casting = make_entity();
        casting.enter_casting(2.0, SkillEnum::SmBash);
        casting.enter_hurt(0.5, None);
        assert_eq!(casting.state, EntityState::Hurt);

        let mut skilling = make_entity();
        skilling.enter_skill_exec(1.0, SkillEnum::SmBash, 1);
        skilling.enter_hurt(0.5, None);
        assert_eq!(skilling.state, EntityState::Hurt);
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
    fn taking_off_an_off_hand_weapon_keeps_the_main_hand_swinging() {
        use models::enums::EnumWithMaskValueU64;
        use models::enums::item::EquipmentLocation;
        let (right, left) = (
            EquipmentLocation::HandRight.as_flag() as u16,
            EquipmentLocation::HandLeft.as_flag() as u16,
        );
        const WEAPON_SLOT: u8 = 2;
        const SHIELD_SLOT: u8 = 8;

        assert_eq!(
            Entity::wear_location_to_sprite_type_for(right, Some(ItemType::Weapon)),
            Some(WEAPON_SLOT)
        );
        assert_eq!(
            Entity::wear_location_to_sprite_type_for(left, Some(ItemType::Armor)),
            Some(SHIELD_SLOT)
        );
        assert_eq!(
            Entity::wear_location_to_sprite_type_for(left, Some(ItemType::Weapon)),
            None
        );

        let mut e = make_entity();
        e.weapon = Some(WeaponType::DoubleDd);
        let armed = e.attack_action_index();
        e.apply_sprite_change(WEAPON_SLOT, 0);
        assert_ne!(
            e.attack_action_index(),
            armed,
            "blanking the weapon slot drops the player to the bare-hand swing"
        );
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

        e.update_state(3.0, false);
        assert!(e.chat_bubble.is_some());
        assert_eq!(e.chat_bubble.as_ref().unwrap().message, "Hello!");

        e.update_state(2.1, false);
        assert!(e.chat_bubble.is_none());
    }

    #[test]
    fn emotion_expires_after_one_pass_of_its_action() {
        let mut e = make_entity();
        e.emotion = Some(super::EmotionState::new(0, 1.4));

        e.update_state(1.3, false);
        assert!(e.emotion.is_some());

        e.update_state(0.2, false);
        assert!(
            e.emotion.is_none(),
            "gone at the action's length, not at the 2.5 s fallback"
        );
    }

    #[test]
    fn casting_uses_action_index_12_for_player() {
        let mut e = make_entity();
        e.enter_casting(2.0, SkillEnum::SmBash);
        assert_eq!(e.state, EntityState::Casting);
        assert_eq!(e.action_index(), 12);
    }

    #[test]
    fn a_locally_timed_channel_counts_down_to_the_combat_stance() {
        let mut e = make_entity();
        e.enter_casting(1.0, SkillEnum::KnAutocounter);
        e.state_timer = 1.0;
        assert_eq!(e.state, EntityState::Casting);

        e.update_state(0.5, false);
        assert_eq!(e.state, EntityState::Casting);

        e.update_state(0.6, false);
        assert_eq!(e.state, EntityState::ReadyFight);
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

        e.enter_casting(2.0, SkillEnum::SmBash);
        assert!(!e.movement.is_moving());
        assert_eq!(e.cast_total_duration, 2.0);
    }

    #[test]
    fn walking_away_mid_cast_keeps_the_cast_running() {
        let mut e = make_entity();
        e.enter_casting(1.0, SkillEnum::SmBash);

        e.begin_move(
            vec![
                make_path_node(101, 100, false),
                make_path_node(102, 100, false),
            ],
            0.0,
        );
        e.update_state(0.5, false);
        assert_eq!(e.state, EntityState::Moving);
        assert_eq!(e.cast_progress(), Some(0.5));

        e.update_state(0.6, false);
        assert_eq!(e.state, EntityState::Moving);
        assert_eq!(e.cast_progress(), None);
    }

    #[test]
    fn attack_expires_to_readyfight_for_player() {
        let mut e = make_entity();
        e.enter_attack(0.5, 1.0);
        assert_eq!(e.state, EntityState::Attacking);
        assert!(e.is_move_locked());

        e.update_state(0.6, false);
        assert_eq!(e.state, EntityState::ReadyFight);
        assert_eq!(e.action_index(), 4);
        assert!(!e.is_move_locked());

        // The stance holds until something else moves the actor out of it.
        e.update_state(5.0, false);
        assert_eq!(e.state, EntityState::ReadyFight);
    }

    #[test]
    fn a_second_attack_swing_ends_standing_not_in_the_combat_stance() {
        let mut archer = make_entity();
        archer.job = JobName::Archer.value() as u16;
        archer.weapon = Some(WeaponType::Bow);
        archer.enter_attack(0.5, 1.0);
        assert_eq!(archer.action_index(), 10);
        archer.update_state(0.6, false);
        assert_eq!(archer.state, EntityState::ReadyFight);

        archer.weapon = None;
        archer.enter_attack(0.5, 1.0);
        assert_eq!(archer.action_index(), 11);
        archer.update_state(0.6, false);
        assert_eq!(archer.state, EntityState::Standing);
    }

    #[test]
    fn a_caster_turns_to_its_target_every_34_frame_units() {
        let mut e = make_entity();
        e.enter_casting(5.0, SkillEnum::MgFirebolt);
        assert_eq!(e.cast_reface_due(1.0), None, "no target, no turn");

        e.cast_target_gid = Some(7);
        assert_eq!(e.cast_reface_due(0.5), None);
        assert_eq!(e.cast_reface_due(0.4), Some(7));
        assert_eq!(e.cast_reface_due(0.4), None);
        assert_eq!(e.cast_reface_due(0.5), Some(7));

        e.set_state(EntityState::Standing);
        assert_eq!(e.cast_reface_due(2.0), None);
    }

    #[test]
    fn throw_pose_avoids_a_filler_attack1_group() {
        use models::enums::skill_enums::SkillEnum;
        let mut e = make_entity();
        e.enter_skill_exec(0.5, SkillEnum::TfThrowstone, 1);
        assert_eq!(e.action_index(), 5, "a throw picks ATTACK1");

        let with_attack1 = make_body_act(5, &[]);
        assert_eq!(e.resolved_action_index(&with_attack1), 5);

        let filler_attack1 = make_body_act(5, &[5]);
        assert_eq!(e.resolved_action_index(&filler_attack1), 10);
    }

    #[test]
    fn bare_hands_swing_with_a_group_the_body_sprite_actually_fills() {
        let mut e = make_entity();
        for (job, group) in [
            (JobName::Gunslinger, 10),
            (JobName::Novice, 10),
            (JobName::Archer, 11),
            (JobName::Monk, 11),
        ] {
            e.job = job.value() as u16;
            assert_eq!(e.attack_action_index(), group, "{job:?}");
        }
    }

    #[test]
    fn move_during_standby_switches_to_walking() {
        let mut e = make_entity();
        e.enter_attack(0.9, 1.0);
        e.update_state(1.0, false);
        assert_eq!(e.state, EntityState::ReadyFight);

        e.begin_move(
            vec![
                make_path_node(101, 100, false),
                make_path_node(102, 100, false),
            ],
            0.0,
        );
        e.update_state(0.016, false);
        assert_eq!(e.state, EntityState::Moving);
        assert_eq!(e.action_index(), 1);
    }

    #[test]
    fn a_multi_hit_replay_restarts_the_swing_parked_on_its_last_frame() {
        use models::enums::skill_enums::SkillEnum;
        let sonic_blow = SkillEnum::AsSonicblow;
        let mut e = make_entity();

        e.enter_skill_exec(0.5, sonic_blow, 8);
        let swing = e.action_index();
        assert_eq!(e.animation_duration.take(), Some(0.5));
        assert!(!e.holds_last_frame(swing, swing, false), "still swinging");
        assert!(
            e.holds_last_frame(swing, swing, true),
            "played through, parked"
        );

        e.enter_attack_replay(sonic_blow);
        assert!(
            !e.holds_last_frame(swing, swing, true),
            "the next hit must replay the swing"
        );
        assert_eq!(e.animation_duration, Some(ATTACK_REPLAY_SECS));
        assert_eq!(
            e.state_timer, ATTACK_REPLAY_SECS,
            "the state must outlast the motion it plays"
        );

        let filler_attack1 = make_body_act(5, &[5]);
        let mut throwing = make_entity();
        throwing.enter_skill_exec(0.5, SkillEnum::TfThrowstone, 1);
        throwing.animation_duration.take();
        let pose = throwing.resolved_action_index(&filler_attack1);
        assert_eq!((throwing.action_index(), pose), (5, 10));
        assert!(
            throwing.holds_last_frame(pose, pose, true),
            "a redirected group parks too, it does not restart"
        );
    }

    #[test]
    fn skill_exec_expires_to_standing_for_player() {
        let mut e = make_entity();
        e.enter_skill_exec(0.5, SkillEnum::AlHeal, 1);
        assert_eq!(e.state, EntityState::SkillExec);
        assert_eq!(e.action_index(), 12);

        e.update_state(0.6, false);
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
        e.update_state(0.6, false);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn weapon_dependent_attack_action() {
        let mut e = Entity::new_player(1, 1, 1, 1, 0, 4, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 1, 1, 1, 0, 2, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 10);

        let mut e = Entity::new_player(1, 12, 1, 1, 0, 16, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 3, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 10);

        let mut e = Entity::new_player(1, 3, 1, 1, 0, 1, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 11, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 19, 1, 1, 0, 11, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 15, 1, 1, 0, 0, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 15, 1, 1, 0, 12, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
        assert_eq!(e.action_index(), 11);

        let mut e = Entity::new_player(1, 15, 1, 1, 0, 8, 0, 0, 0, 0, 100, 100, 0);
        e.set_state(EntityState::Attacking);
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
