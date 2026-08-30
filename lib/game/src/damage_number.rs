use crate::scheduled_hit::{DamageMessage, ScheduledHit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageNumberType {
    Normal,
    Critical,
    Enemy,
    Combo,
    ComboFinal,
    MultiHit,
    MultiHitTotal,
    Heal,
    /// Recovery rising animation recoloured by `DamageNumber::color_override`.
    EffectNumber,
    Miss,
    Lucky,
}

const DIGIT_SPACING: f32 = 8.0;
const SCREEN_X_FACTOR_PER_PIXEL: f32 = 1.0 / 640.0;
const AVG_PIXEL_RATIO_PER_PIXEL: f32 = 1.34 / 768.0;
const FRAME_MS: f32 = 24.0; // ms per animation tick
/// World units above the actor origin a number spawns at. Everything here is in
/// world units, never screen pixels: one sprite pixel is only `map_zoom / 75`
/// world units, so mixing the two shrinks the whole motion by 7.5x at zoom 10.
const LIFT: f32 = 12.0;
/// A running total spawns this much higher again.
const TOTAL_EXTRA_LIFT: f32 = 15.0;
/// Lucky is animated by the generic arm, which lifts further and never moves.
const LUCKY_LIFT: f32 = 20.0;
/// The arc adds a constant 8 and a fixed feedback term of 4/3 on top of `LIFT`.
const ARC_BIAS: f32 = 8.0;
const ARC_FEEDBACK: f32 = 4.0 / 3.0;
/// Rise per frame for the types that climb linearly instead of arcing.
const MISS_RISE: f32 = 0.54;
const TOTAL_RISE: f32 = 0.1;
const RECOVERY_RISE: f32 = 0.18;
/// World units per frame of sideways travel, before the `f/3 + 3` ramp.
const LATERAL_NORMAL: f32 = 0.8;
const LATERAL_CRITICAL: f32 = 0.5;
/// The critical plate is its own object with its own curves.
const CRIT_PLATE_ZOOM_START: f32 = 3.0;
const CRIT_PLATE_ZOOM_DECAY: f32 = 0.144;
const CRIT_PLATE_ZOOM_MIN: f32 = 1.0;
const CRIT_PLATE_FADE_PER_FRAME: f32 = 3.45;
/// A total holds full opacity this long before it starts to fade.
const TOTAL_HOLD_FRAMES: f32 = 90.0;
/// Every blow of a multi-hit animates as the running total: zoom units per frame².
const TOTAL_ZOOM_START: f32 = 0.5;
const TOTAL_ZOOM_ACCEL: f32 = 0.09;
const TOTAL_ZOOM_MAX: f32 = 3.0;
const TOTAL_FADE_PER_FRAME: f32 = 8.0;
const DAMAGE_FADE_PER_FRAME: f32 = 3.4;
const RECOVERY_FADE_PER_FRAME: f32 = 2.0;
const PLAYER_RED: [f32; 3] = [1.0, 0.0, 0.0];
/// Running multi-hit totals only. Every other blow, skill or critical, stays
/// white; anything landing on a player goes red.
const COMBO_YELLOW: [f32; 3] = [0.9, 0.9, 0.15];
/// Lucky plays `msg.act` action 3 once: 28 motions held 4 frames each. The word
/// itself only joins the backdrop from motion 2 on.
const LUCKY_MOTIONS: f32 = 28.0;
const LUCKY_FRAMES_PER_MOTION: f32 = 4.0;
const LUCKY_WORD_FROM_FRAME: f32 = 2.0 * LUCKY_FRAMES_PER_MOTION;
/// The critical backdrop under a multi-hit total, in zoom units per frame².
const TOTAL_CRIT_ZOOM_START: f32 = 0.5;
const TOTAL_CRIT_ZOOM_ACCEL: f32 = 0.06;
const TOTAL_CRIT_ZOOM_MAX: f32 = 2.0;
/// World units a multi-hit total's backdrop sits above its digits.
const TOTAL_CRIT_LIFT: f32 = 15.0;

impl DamageNumberType {
    pub fn color(&self) -> [f32; 3] {
        match self {
            Self::Enemy => PLAYER_RED,
            Self::Heal => [0.0, 1.0, 0.0],
            t if t.is_combo() || t.is_total() => COMBO_YELLOW,
            _ => [1.0, 1.0, 1.0],
        }
    }

    pub fn is_total(&self) -> bool {
        matches!(self, Self::ComboFinal | Self::MultiHitTotal)
    }

    pub fn is_combo(&self) -> bool {
        matches!(self, Self::Combo | Self::MultiHit)
    }

    pub fn uses_msg_sprite(&self) -> bool {
        matches!(self, Self::Miss | Self::Lucky)
    }

    pub fn is_combat_damage(&self) -> bool {
        matches!(
            self,
            Self::Normal
                | Self::Critical
                | Self::Enemy
                | Self::Combo
                | Self::ComboFinal
                | Self::MultiHit
                | Self::MultiHitTotal
        )
    }

    /// Animation frames the number stays alive for.
    fn life_frames(&self) -> f32 {
        match self {
            Self::Normal | Self::Critical | Self::Enemy => 70.0,
            Self::Miss => 80.0,
            Self::Lucky => LUCKY_MOTIONS * LUCKY_FRAMES_PER_MOTION,
            _ => 120.0,
        }
    }

    pub fn duration(&self) -> f32 {
        self.life_frames() * FRAME_MS / 1000.0
    }
}

/// Sign pair for the world XZ drift, from the actor's facing in degrees.
///
/// The thresholds do not wrap, so a facing sitting exactly on a full turn falls
/// through to the last bucket instead of folding back to the first. That is why
/// a spin stops at 360 rather than resetting to 0, and why the caller passes an
/// accumulated angle instead of an eight-way direction: a spinning target throws
/// its numbers to alternating sides only if the angle keeps its own history.
fn lateral_quadrant(facing_degrees: f32) -> (f32, f32) {
    let degrees = facing_degrees as i32;
    match degrees {
        0..90 => (-1.0, 1.0),
        90..180 => (-1.0, -1.0),
        180..270 => (1.0, -1.0),
        _ => (1.0, 1.0),
    }
}

pub struct DamageNumber {
    pub entity_id: u32,
    pub value: i32,
    pub number_type: DamageNumberType,
    pub elapsed: f32,
    pub facing_degrees: f32,
    pub last_screen_pos: Option<(f32, f32, f32)>,
    /// RGB override; falls back to `number_type.color()`.
    pub color_override: Option<[f32; 3]>,
    /// A multi-hit burst that landed a critical: the running total keeps its own
    /// size but gains the critical backdrop.
    pub has_critical: bool,
}

impl DamageNumber {
    pub fn new(
        entity_id: u32,
        value: i32,
        number_type: DamageNumberType,
        facing_degrees: f32,
    ) -> Self {
        Self {
            entity_id,
            value,
            number_type,
            elapsed: 0.0,
            facing_degrees,
            last_screen_pos: None,
            color_override: None,
            has_critical: false,
        }
    }

    pub fn effect_number(entity_id: u32, value: i32, color: [f32; 3], facing_degrees: f32) -> Self {
        Self {
            color_override: Some(color),
            ..Self::new(
                entity_id,
                value,
                DamageNumberType::EffectNumber,
                facing_degrees,
            )
        }
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.number_type.duration()
    }

    fn frame(&self) -> f32 {
        self.elapsed * 1000.0 / FRAME_MS
    }

    /// World units the number sits above the actor origin this frame.
    fn rise(&self) -> f32 {
        let f = self.frame();
        match self.number_type {
            t if t.is_combo() || t.is_total() => LIFT + TOTAL_EXTRA_LIFT + f * TOTAL_RISE,
            DamageNumberType::Miss => LIFT + f * MISS_RISE,
            DamageNumberType::Lucky => LUCKY_LIFT,
            DamageNumberType::Heal | DamageNumberType::EffectNumber => LIFT + f * RECOVERY_RISE,
            // Arcs up and falls back through its own start before it expires.
            _ => LIFT - ARC_BIAS - ARC_FEEDBACK + f * (2.0 - f / 30.0),
        }
    }

    /// Sideways travel in world XZ. Only the arcing types drift; everything else
    /// stays pinned over the actor.
    fn lateral(&self) -> (f32, f32) {
        let magnitude = match self.number_type {
            DamageNumberType::Critical => LATERAL_CRITICAL,
            DamageNumberType::Normal | DamageNumberType::Enemy => LATERAL_NORMAL,
            _ => return (0.0, 0.0),
        };
        let (sx, sz) = lateral_quadrant(self.facing_degrees);
        let travel = magnitude * (self.frame() / 3.0 + 3.0);
        (sx * travel, sz * travel)
    }

    /// Offset from the actor origin in world units, to be added to its world
    /// position before projecting. World Y is negative-up.
    pub fn world_offset(&self) -> [f32; 3] {
        let (dx, dz) = self.lateral();
        [dx, -self.rise(), dz]
    }

    /// The critical plate for a multi-hit total rides its own world position.
    pub fn backdrop_world_offset(&self) -> Option<[f32; 3]> {
        let b = self.critical_backdrop()?;
        if b.extra_lift == 0.0 {
            return None;
        }
        let [dx, dy, dz] = self.world_offset();
        Some([dx, dy - b.extra_lift, dz])
    }

    pub fn zoom(&self) -> f32 {
        let f = self.frame();
        match self.number_type {
            t if t.is_combo() || t.is_total() => {
                (TOTAL_ZOOM_START + TOTAL_ZOOM_ACCEL * f * f).min(TOTAL_ZOOM_MAX)
            }
            DamageNumberType::Critical => (5.0 - f * 0.24).max(1.3),
            DamageNumberType::Miss => 1.0,
            DamageNumberType::Lucky => 1.0,
            DamageNumberType::Heal | DamageNumberType::EffectNumber => (5.0 - f * 0.24).max(1.0),
            _ => (5.0 - f * 0.24).max(1.2),
        }
    }

    pub fn alpha(&self) -> f32 {
        let f = self.frame();
        let alpha_255 = match self.number_type {
            t if t.is_combo() || t.is_total() => {
                250.0 - (f - TOTAL_HOLD_FRAMES).max(0.0) * TOTAL_FADE_PER_FRAME
            }
            DamageNumberType::Miss => 250.0 - f * 3.0,
            DamageNumberType::Lucky => 255.0,
            DamageNumberType::Normal | DamageNumberType::Critical | DamageNumberType::Enemy => {
                250.0 - f * DAMAGE_FADE_PER_FRAME
            }
            _ => 250.0 - f * RECOVERY_FADE_PER_FRAME,
        };
        (alpha_255 / 255.0).clamp(0.0, 1.0)
    }

    fn critical_backdrop(&self) -> Option<CriticalBackdrop> {
        let f = self.frame();
        if self.number_type == DamageNumberType::Critical {
            return Some(CriticalBackdrop {
                zoom: (CRIT_PLATE_ZOOM_START - f * CRIT_PLATE_ZOOM_DECAY).max(CRIT_PLATE_ZOOM_MIN),
                alpha: ((255.0 - f * CRIT_PLATE_FADE_PER_FRAME) / 255.0).clamp(0.0, 1.0),
                extra_lift: 0.0,
            });
        }
        if !self.has_critical {
            return None;
        }
        Some(CriticalBackdrop {
            zoom: (TOTAL_CRIT_ZOOM_START + TOTAL_CRIT_ZOOM_ACCEL * f * f).min(TOTAL_CRIT_ZOOM_MAX),
            alpha: self.alpha(),
            extra_lift: TOTAL_CRIT_LIFT,
        })
    }

    pub fn digits(&self) -> Vec<u8> {
        if self.value == 0 {
            return vec![0];
        }
        let clamped = self.value.unsigned_abs().min(999999);
        let s = clamped.to_string();
        s.bytes().rev().map(|b| b - b'0').collect()
    }

    pub fn digit_x_offset(&self, i: usize, count: usize) -> f32 {
        let fi = i as f32;
        let fc = count as f32;
        -fi * DIGIT_SPACING + (DIGIT_SPACING / 2.0) * (fc - 1.0)
    }

    pub fn render_data(&self) -> Option<DamageNumberRenderData> {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return None;
        }
        let [cr, cg, cb] = self
            .color_override
            .unwrap_or_else(|| self.number_type.color());
        let digits = self.digits();
        let count = digits.len();
        let digit_x_offsets = (0..count).map(|i| self.digit_x_offset(i, count)).collect();
        let msg_frames = match self.number_type {
            DamageNumberType::Miss => vec![MSG_FRAME_MISS],
            DamageNumberType::Lucky if self.frame() < LUCKY_WORD_FROM_FRAME => {
                vec![MSG_FRAME_LUCKYBG]
            }
            DamageNumberType::Lucky => vec![MSG_FRAME_LUCKYBG, MSG_FRAME_LUCKY],
            _ => vec![],
        };
        Some(DamageNumberRenderData {
            digits,
            digit_x_offsets,
            color: [cr, cg, cb, alpha],
            zoom: self.zoom(),
            uses_msg_sprite: self.number_type.uses_msg_sprite(),
            msg_frames,
            critical_backdrop: self.critical_backdrop(),
        })
    }
}

/// The `msg.spr` critical plate drawn behind a number, sized and faded
/// independently of the digits it backs.
pub struct CriticalBackdrop {
    pub zoom: f32,
    pub alpha: f32,
    /// World units above the digits. Non-zero means it needs its own projection.
    pub extra_lift: f32,
}

pub struct DamageNumberRenderData {
    pub digits: Vec<u8>,
    pub digit_x_offsets: Vec<f32>,
    pub color: [f32; 4],
    pub zoom: f32,
    pub uses_msg_sprite: bool,
    pub msg_frames: Vec<usize>,
    pub critical_backdrop: Option<CriticalBackdrop>,
}

pub struct DamageNumberRenderEntry {
    pub entity_id: u32,
    /// Projection of the actor origin plus `DamageNumber::world_offset`.
    pub screen_x: f32,
    pub screen_y: f32,
    pub scale: f32,
    /// Projection of `backdrop_world_offset`, when the plate rides higher than
    /// the digits and so needs its own.
    pub backdrop_screen: Option<(f32, f32, f32)>,
    pub data: DamageNumberRenderData,
}

pub use ragnarok_formats::damage_number::{DamageNumberQuad, TextureSource};

/// Every retail map ships a GND zoom of 10.0.
pub const STANDARD_MAP_ZOOM: f32 = 10.0;

/// Sprite pixels of digit pitch per unit of `digit_x_offset`, for a viewport of
/// `screen_w` x `screen_h`. Widescreen spaces the digits further apart.
pub fn digit_pitch_scale(screen_w: f32, screen_h: f32) -> f32 {
    if screen_h <= 0.0 {
        return 1.0;
    }
    ((screen_w * SCREEN_X_FACTOR_PER_PIXEL) / (screen_h * AVG_PIXEL_RATIO_PER_PIXEL))
        // Setting min and max otherwise on high resolution spacing is too important and on very low, digit are render over each other
        .min(1.3)
        .max(0.9)
}

/// Pixels per world unit implied by a sprite scale, since `sprite_scale` is
/// `pixels_per_world_unit * map_zoom / 75`.
pub fn pixels_per_world_unit(sprite_scale: f32, map_zoom: f32) -> f32 {
    sprite_scale * 75.0 / map_zoom
}

/// Screen position for a world offset, for callers holding a sprite scale but no
/// camera. It drops the offset straight onto the screen axes, so the sideways
/// drift will not swing with camera yaw the way the projected path does. The
/// game projects properly; this is for the 2D viewers.
pub fn flat_screen_offset(
    anchor: (f32, f32),
    world_offset: [f32; 3],
    pixels_per_world_unit: f32,
) -> (f32, f32) {
    (
        anchor.0 + world_offset[0] * pixels_per_world_unit,
        anchor.1 + world_offset[1] * pixels_per_world_unit,
    )
}

/// Turn live numbers into render entries.
///
/// `project` maps an entity and a world offset onto a screen position and sprite
/// scale, returning `None` when the entity cannot be placed. It is a callback
/// because projecting needs the camera, which lives in the renderer crate and is
/// not something this crate can reach.
pub fn build_damage_number_entries<F>(
    numbers: &mut [DamageNumber],
    mut project: F,
) -> Vec<DamageNumberRenderEntry>
where
    F: FnMut(u32, [f32; 3]) -> Option<(f32, f32, f32)>,
{
    numbers
        .iter_mut()
        .filter_map(|dmg| {
            // A number outlives the entity that spawned it, so it falls back to
            // wherever it was last drawn.
            let placed = match project(dmg.entity_id, dmg.world_offset()) {
                Some(pos) => {
                    dmg.last_screen_pos = Some(pos);
                    pos
                }
                None => dmg.last_screen_pos?,
            };
            let backdrop_screen = dmg
                .backdrop_world_offset()
                .and_then(|offset| project(dmg.entity_id, offset));
            Some(DamageNumberRenderEntry {
                entity_id: dmg.entity_id,
                screen_x: placed.0,
                screen_y: placed.1,
                scale: placed.2,
                backdrop_screen,
                data: dmg.render_data()?,
            })
        })
        .collect()
}

pub fn build_damage_number_quads(
    entries: &[DamageNumberRenderEntry],
    num_act: &ragnarok_formats::act::ActFile,
    num_sizes: &[(u32, u32)],
    num_indexed_count: usize,
    msg_sizes: Option<&[(u32, u32)]>,
    viewport: (f32, f32),
) -> Vec<DamageNumberQuad> {
    let mut quads = Vec::new();
    let pitch_scale = digit_pitch_scale(viewport.0, viewport.1);
    for entry in entries {
        let dmg = &entry.data;
        let s = entry.scale;
        let base_x = entry.screen_x;
        let base_y = entry.screen_y;
        let [cr, cg, cb, alpha] = dmg.color;
        let zoom = dmg.zoom * s;

        if dmg.uses_msg_sprite {
            let msg_sz = match msg_sizes {
                Some(s) => s,
                None => continue,
            };
            for &frame_idx in &dmg.msg_frames {
                if frame_idx >= msg_sz.len() {
                    continue;
                }
                let (tw, th) = msg_sz[frame_idx];
                let sw = tw as f32 * zoom;
                let sh = th as f32 * zoom;
                let x = base_x - sw / 2.0;
                let y = base_y - sh / 2.0;
                quads.push(DamageNumberQuad {
                    x,
                    y,
                    w: sw,
                    h: sh,
                    color: [cr, cg, cb, alpha],
                    source: TextureSource::Message,
                    tex_idx: frame_idx,
                });
            }
            continue;
        }

        let action = &num_act.actions[0];

        if let Some(backdrop) = &dmg.critical_backdrop
            && let Some(msg_sz) = msg_sizes
            && MSG_FRAME_CRITBG < msg_sz.len()
        {
            let (tw, th) = msg_sz[MSG_FRAME_CRITBG];
            let (plate_x, plate_y, plate_s) = entry.backdrop_screen.unwrap_or((base_x, base_y, s));
            let crit_zoom = backdrop.zoom * plate_s;
            let sw = tw as f32 * crit_zoom;
            let sh = th as f32 * crit_zoom;
            let x = plate_x - sw / 2.0;
            let y = plate_y - sh / 2.0;
            quads.push(DamageNumberQuad {
                x,
                y,
                w: sw,
                h: sh,
                color: [0.66, 0.66, 0.66, backdrop.alpha],
                source: TextureSource::Message,
                tex_idx: MSG_FRAME_CRITBG,
            });
        }

        for (i, &digit) in dmg.digits.iter().enumerate() {
            let motion_idx = digit as usize;
            if motion_idx >= action.motions.len() {
                continue;
            }
            let motion = &action.motions[motion_idx];
            if motion.clips.is_empty() {
                continue;
            }
            let clip = &motion.clips[0];
            if clip.sprite_index < 0 {
                continue;
            }

            let tex_idx = if clip.sprite_type == 0 {
                clip.sprite_index as usize
            } else {
                num_indexed_count + clip.sprite_index as usize
            };
            if tex_idx >= num_sizes.len() {
                continue;
            }

            let (tw, th) = num_sizes[tex_idx];
            let sw = tw as f32 * zoom;
            let sh = th as f32 * zoom;

            println!("{}",pitch_scale);
            let x_offset = dmg.digit_x_offsets.get(i).copied().unwrap_or(0.0) * pitch_scale * zoom;
            let x = base_x + x_offset - sw / 2.0;
            let y = base_y - sh / 2.0;

            quads.push(DamageNumberQuad {
                x,
                y,
                w: sw,
                h: sh,
                color: [cr, cg, cb, alpha],
                source: TextureSource::Number,
                tex_idx,
            });
        }
    }
    quads
}

pub struct DamageNumberManager {
    pub numbers: Vec<DamageNumber>,
    pub combat_hidden: bool,
}

impl Default for DamageNumberManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DamageNumberManager {
    pub fn new() -> Self {
        Self {
            numbers: Vec::new(),
            combat_hidden: false,
        }
    }

    pub fn clear(&mut self) {
        self.numbers.clear();
    }

    pub fn add(&mut self, number: DamageNumber) {
        if self.combat_hidden && number.number_type.is_combat_damage() {
            return;
        }
        let removes_combo = number.number_type.is_total() || number.number_type.is_combo();
        if removes_combo {
            self.numbers
                .retain(|n| !(n.entity_id == number.entity_id && n.number_type.is_combo()));
        }
        self.numbers.push(number);
    }

    pub fn emit(
        &mut self,
        entity_id: u32,
        facing_degrees: f32,
        hit: &ScheduledHit,
        is_player_target: bool,
        attacker_is_player: bool,
    ) {
        let is_multi_hit = matches!(hit.message, DamageMessage::AttackedMultiHit { .. });
        let is_skill = hit.skill.is_some();

        if hit.damage < 0 {
            return;
        }

        let total_zero = match hit.message {
            DamageMessage::AttackedMultiHit { total_damage } => total_damage == 0,
            _ => hit.damage == 0,
        };
        if total_zero {
            if hit.hit_index == 0 {
                let mut miss =
                    DamageNumber::new(entity_id, 0, DamageNumberType::Miss, facing_degrees);
                if attacker_is_player {
                    miss.color_override = Some(PLAYER_RED);
                }
                self.add(miss);
            }
            return;
        }

        if is_multi_hit {
            // Every blow shows its own damage as a plain hit: never the skill
            // colour, and a critical marks only the total, never these.
            let blow_type = if is_player_target {
                DamageNumberType::Enemy
            } else {
                DamageNumberType::Normal
            };
            self.add(DamageNumber::new(
                entity_id,
                hit.damage.abs(),
                blow_type,
                facing_degrees,
            ));
            // Riding above them, the running total in the combo colour. The
            // left-hand hit of a dual wield breaks the even split, so the last
            // one reads the burst total instead of extrapolating.
            let running_total = match hit.message {
                DamageMessage::AttackedMultiHit { total_damage } if hit.is_last_hit => total_damage,
                _ => hit.damage * (hit.hit_index as i32 + 1),
            };
            let combo_type = if hit.is_last_hit {
                if is_skill {
                    DamageNumberType::ComboFinal
                } else {
                    DamageNumberType::MultiHitTotal
                }
            } else if is_skill {
                DamageNumberType::Combo
            } else {
                DamageNumberType::MultiHit
            };
            let mut number =
                DamageNumber::new(entity_id, running_total, combo_type, facing_degrees);
            number.has_critical = hit.is_critical;
            self.add(number);
        } else {
            let number_type = if hit.is_critical {
                DamageNumberType::Critical
            } else if is_player_target {
                DamageNumberType::Enemy
            } else {
                DamageNumberType::Normal
            };
            self.add(DamageNumber::new(
                entity_id,
                hit.damage.abs(),
                number_type,
                facing_degrees,
            ));
        }
    }

    pub fn update(&mut self, dt: f32) {
        for n in &mut self.numbers {
            n.elapsed += dt;
        }
        self.numbers.retain(|n| !n.is_expired());
    }
}

pub const MSG_FRAME_MISS: usize = 0;
pub const MSG_FRAME_CRIT: usize = 2;
pub const MSG_FRAME_CRITBG: usize = 3;
pub const MSG_FRAME_LUCKYBG: usize = 4;
pub const MSG_FRAME_LUCKY: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::skill_enums::SkillEnum;

    #[test]
    fn digits_decomposition() {
        let d = DamageNumber::new(1, 12345, DamageNumberType::Normal, 0.0);
        assert_eq!(d.digits(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn digits_zero() {
        let d = DamageNumber::new(1, 0, DamageNumberType::Miss, 0.0);
        assert_eq!(d.digits(), vec![0]);
    }

    #[test]
    fn digits_clamped_to_999999() {
        let d = DamageNumber::new(1, 1_500_000, DamageNumberType::Normal, 0.0);
        assert_eq!(d.digits(), vec![9, 9, 9, 9, 9, 9]);
    }

    #[test]
    fn digit_x_offsets_symmetric_for_3_digits() {
        let d = DamageNumber::new(1, 123, DamageNumberType::Normal, 0.0);
        let offsets: Vec<f32> = (0..3).map(|i| d.digit_x_offset(i, 3)).collect();
        assert_eq!(offsets, vec![8.0, 0.0, -8.0]);
    }

    #[test]
    fn digit_pitch_widens_with_the_viewport_aspect() {
        let d = DamageNumber::new(1, 123, DamageNumberType::Normal, 0.0);
        let pitch = |w: f32, h: f32| {
            (d.digit_x_offset(0, 3) - d.digit_x_offset(1, 3)) * digit_pitch_scale(w, h)
        };
        assert!((pitch(640.0, 480.0) - pitch(1024.0, 768.0)).abs() < 1e-4);
        assert!(pitch(1920.0, 1080.0) > pitch(1024.0, 768.0));
        // The widest glyph in the number sprite is 10px, so a pitch below that
        // overlaps its neighbour.
        assert!(pitch(1920.0, 1080.0) > 10.0);
    }

    #[test]
    fn normal_starts_large_shrinks() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0);
        let z0 = d.zoom();
        d.elapsed = 0.5;
        let z1 = d.zoom();
        assert!(z0 > z1);
        assert!(z1 >= 1.2);
    }

    #[test]
    fn total_starts_small_grows() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0.0);
        let z0 = d.zoom();
        d.elapsed = 0.5;
        let z1 = d.zoom();
        assert!(z0 < z1);
        assert!(z0 >= 0.5);
    }

    #[test]
    fn a_critical_reads_white_and_is_marked_by_its_plate_alone() {
        let crit = DamageNumber::new(1, 100, DamageNumberType::Critical, 0.0);
        assert_eq!(crit.number_type.color(), [1.0, 1.0, 1.0]);
        let plate = crit.render_data().unwrap().critical_backdrop;
        assert!(plate.is_some());
    }

    #[test]
    fn a_double_attack_reads_like_a_skill_combo_not_like_plain_damage() {
        assert_eq!(DamageNumberType::MultiHit.color(), COMBO_YELLOW);
        assert_eq!(
            DamageNumberType::MultiHitTotal.color(),
            DamageNumberType::ComboFinal.color()
        );
        assert_ne!(
            DamageNumberType::MultiHit.color(),
            DamageNumberType::Normal.color()
        );

        let mut plain = DamageNumber::new(1, 120, DamageNumberType::Normal, 0.0);
        let mut by_skill = DamageNumber::new(1, 120, DamageNumberType::Combo, 0.0);
        let mut by_weapon = DamageNumber::new(1, 120, DamageNumberType::MultiHit, 0.0);
        assert_eq!(by_weapon.zoom(), TOTAL_ZOOM_START);

        for n in [&mut plain, &mut by_skill, &mut by_weapon] {
            n.elapsed = 5.0 * FRAME_MS / 1000.0;
        }
        // A multi-hit swells and climbs on the total's curve whatever caused it,
        // while plain damage shrinks from its opening flash.
        assert_eq!(by_weapon.zoom(), by_skill.zoom());
        assert!(by_weapon.zoom() > TOTAL_ZOOM_START);
        assert!(plain.zoom() < 5.0);
        assert_eq!(by_weapon.world_offset(), by_skill.world_offset());
        // World Y is negative-up, so a climbing number has a falling Y.
        assert!(by_weapon.world_offset()[1] < 0.0);
    }

    #[test]
    fn enemy_color_is_red() {
        assert_eq!(DamageNumberType::Enemy.color(), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn heal_color_is_green() {
        assert_eq!(DamageNumberType::Heal.color(), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_single_skill_hit_is_white_on_a_monster_and_red_on_a_player() {
        let hit = ScheduledHit::single(100, Some(SkillEnum::MgFirebolt), false);

        let mut player = DamageNumberManager::new();
        player.emit(1, 0.0, &hit, true, false);
        assert_eq!(
            player.numbers.last().unwrap().number_type,
            DamageNumberType::Enemy
        );

        let mut monster = DamageNumberManager::new();
        monster.emit(1, 0.0, &hit, false, false);
        let on_monster = monster.numbers.last().unwrap();
        assert_eq!(on_monster.number_type, DamageNumberType::Normal);
        assert_eq!(
            on_monster.number_type.color(),
            [1.0, 1.0, 1.0],
            "only criticals and multi-hit totals are yellow"
        );

        let weapon_hit = ScheduledHit::single(100, None, false);
        let mut plain = DamageNumberManager::new();
        plain.emit(1, 0.0, &weapon_hit, false, false);
        let blow = plain.numbers.last().unwrap();
        assert_eq!(blow.number_type, DamageNumberType::Normal);
        assert_eq!(blow.number_type.color(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn fully_missed_multi_hit_shows_single_miss() {
        let mut mgr = DamageNumberManager::new();
        mgr.emit(
            1,
            0.0,
            &ScheduledHit::multi_hit(0, 0, Some(SkillEnum::MgSight), 0, false),
            false,
            false,
        );
        mgr.emit(
            1,
            0.0,
            &ScheduledHit::multi_hit(0, 0, Some(SkillEnum::MgSight), 1, true),
            false,
            false,
        );
        assert_eq!(mgr.numbers.len(), 1);
        assert_eq!(mgr.numbers[0].number_type, DamageNumberType::Miss);
    }

    #[test]
    fn a_players_miss_is_red_a_monsters_stays_white() {
        let miss = ScheduledHit::single(0, None, false);

        let mut by_player = DamageNumberManager::new();
        by_player.emit(1, 0.0, &miss, false, true);
        assert_eq!(
            by_player.numbers[0].render_data().unwrap().color[..3],
            PLAYER_RED
        );

        let mut by_monster = DamageNumberManager::new();
        by_monster.emit(1, 0.0, &miss, true, false);
        assert_eq!(
            by_monster.numbers[0].render_data().unwrap().color[..3],
            [1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn combo_replaces_previous_combo_on_same_entity() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 50, DamageNumberType::Combo, 0.0));
        assert_eq!(mgr.numbers.len(), 1);

        // New combo replaces old one
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Combo, 0.0));
        assert_eq!(mgr.numbers.len(), 1);
        assert_eq!(mgr.numbers[0].value, 100);
    }

    #[test]
    fn manager_removes_combo_on_total() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 50, DamageNumberType::Combo, 0.0));
        mgr.add(DamageNumber::new(2, 50, DamageNumberType::Combo, 0.0));
        assert_eq!(mgr.numbers.len(), 2);

        mgr.add(DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0.0));
        assert_eq!(mgr.numbers.len(), 2);
        assert!(mgr.numbers.iter().any(|n| n.entity_id == 2));
        assert!(
            mgr.numbers
                .iter()
                .any(|n| n.number_type == DamageNumberType::ComboFinal)
        );
    }

    #[test]
    fn clear_drops_all_numbers() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0));
        mgr.add(DamageNumber::new(2, 50, DamageNumberType::Combo, 0.0));
        mgr.clear();
        assert!(mgr.numbers.is_empty());
    }

    #[test]
    fn expired_numbers_removed_on_update() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0));
        mgr.update(3.0);
        assert!(mgr.numbers.is_empty());
    }

    #[test]
    fn facing_decides_which_side_the_number_drifts_to() {
        // Half the facings throw left and half right. Leaving the angle
        // unwrapped collapses all eight onto one side.
        let x_of = |dir: u8| {
            DamageNumber::new(
                1,
                100,
                DamageNumberType::Normal,
                crate::entity::facing_degrees_for(dir),
            )
            .world_offset()[0]
        };
        for dir in 0..4 {
            assert!(x_of(dir) > 0.0, "dir {dir} should drift right");
        }
        for dir in 4..8 {
            assert!(x_of(dir) < 0.0, "dir {dir} should drift left");
        }
    }

    #[test]
    fn a_spinning_target_throws_its_numbers_to_alternating_sides() {
        // A multi-hit skill turns its target a quarter turn per blow, and the
        // buckets are a quarter turn wide, so each blow drifts to the next one.
        let mut facing = crate::entity::facing_degrees_for(3);
        let mut xs = Vec::new();
        for _ in 0..4 {
            facing += 90.0;
            if facing > 360.0 {
                facing -= 360.0;
            }
            xs.push(DamageNumber::new(1, 100, DamageNumberType::Normal, facing).world_offset()[0]);
        }
        assert!(xs[0] < 0.0 && xs[1] < 0.0, "{xs:?}");
        assert!(xs[2] > 0.0 && xs[3] > 0.0, "{xs:?}");
    }

    #[test]
    fn offsets_are_world_units_not_sprite_pixels() {
        // Spawn sits LIFT above the actor origin, less the arc's own bias. A
        // sprite-pixel reading of these numbers would be 7.5x too small.
        let d = DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0);
        assert_eq!(d.world_offset()[1], -(LIFT - ARC_BIAS - ARC_FEEDBACK));

        // The arc peaks near frame 30 and falls back through its start by 60.
        let mut peak = DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0);
        peak.elapsed = 30.0 * FRAME_MS / 1000.0;
        let mut back = DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0);
        back.elapsed = 60.0 * FRAME_MS / 1000.0;
        assert!(peak.world_offset()[1] < -30.0);
        assert!((back.world_offset()[1] - d.world_offset()[1]).abs() < 1e-3);

        // A running total spawns higher again and only climbs.
        let total = DamageNumber::new(1, 100, DamageNumberType::MultiHitTotal, 0.0);
        assert_eq!(total.world_offset()[1], -(LIFT + TOTAL_EXTRA_LIFT));
    }

    #[test]
    fn a_recovery_number_climbs_and_shrinks_on_its_own_curve() {
        let mut heal = DamageNumber::new(1, 42, DamageNumberType::Heal, 0.0);
        assert_eq!(heal.world_offset()[1], -LIFT);
        assert_eq!(heal.zoom(), 5.0);

        heal.elapsed = 10.0 * FRAME_MS / 1000.0;
        assert_eq!(heal.world_offset()[1], -(LIFT + 10.0 * RECOVERY_RISE));
        // Shares the digit decay but floors lower than a damage number does.
        assert_eq!(heal.zoom(), 5.0 - 10.0 * 0.24);
        let mut settled = DamageNumber::new(1, 42, DamageNumberType::Heal, 0.0);
        settled.elapsed = 30.0 * FRAME_MS / 1000.0;
        assert_eq!(settled.zoom(), 1.0);
        // It never drifts sideways.
        assert_eq!(heal.world_offset()[0], 0.0);
    }

    #[test]
    fn combat_hidden_suppresses_damage_but_keeps_miss_and_heal() {
        let mut mgr = DamageNumberManager::new();
        mgr.combat_hidden = true;

        mgr.emit(
            1,
            0.0,
            &ScheduledHit::single(100, None, false),
            false,
            false,
        );
        assert!(mgr.numbers.is_empty());

        mgr.emit(1, 0.0, &ScheduledHit::single(0, None, false), false, false);
        assert_eq!(
            mgr.numbers.last().unwrap().number_type,
            DamageNumberType::Miss
        );

        mgr.add(DamageNumber::new(1, 42, DamageNumberType::Heal, 0.0));
        assert!(
            mgr.numbers
                .iter()
                .any(|n| n.number_type == DamageNumberType::Heal)
        );

        mgr.combat_hidden = false;
        let before = mgr.numbers.len();
        mgr.emit(
            1,
            0.0,
            &ScheduledHit::single(100, None, false),
            false,
            false,
        );
        assert_eq!(mgr.numbers.len(), before + 1);
    }

    #[test]
    fn a_multi_hit_keeps_every_blow_and_one_running_total() {
        let mut mgr = DamageNumberManager::new();
        for (index, last) in [(0u16, false), (1, false), (2, true)] {
            mgr.emit(
                1,
                0.0,
                &ScheduledHit::multi_hit(30, 90, Some(SkillEnum::MgFirebolt), index, last),
                false,
                false,
            );
        }

        // Three blows at their own damage, plus a single total that replaced
        // itself each hit.
        let blows: Vec<&DamageNumber> = mgr
            .numbers
            .iter()
            .filter(|n| !n.number_type.is_combo() && !n.number_type.is_total())
            .collect();
        assert_eq!(blows.len(), 3);
        assert!(blows.iter().all(|n| n.value == 30));
        // Plain white even though a skill drove the burst — only the total
        // carries the skill colour.
        assert_eq!(blows[0].number_type, DamageNumberType::Normal);
        assert_eq!(blows[0].number_type.color(), [1.0, 1.0, 1.0]);

        let totals: Vec<&DamageNumber> = mgr
            .numbers
            .iter()
            .filter(|n| n.number_type.is_combo() || n.number_type.is_total())
            .collect();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].number_type, DamageNumberType::ComboFinal);
        assert_eq!(totals[0].value, 90);
        assert_eq!(totals[0].number_type.color(), COMBO_YELLOW);
    }

    #[test]
    fn damage_fades_out_faster_and_earlier_than_a_total() {
        let mut damage = DamageNumber::new(1, 100, DamageNumberType::Normal, 0.0);
        let mut total = DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0.0);

        // Frame 60: damage is most of the way out, the total has not begun to fade.
        damage.elapsed = 60.0 * FRAME_MS / 1000.0;
        total.elapsed = damage.elapsed;
        assert!(damage.alpha() < total.alpha());
        assert_eq!(total.alpha(), 250.0 / 255.0);

        damage.elapsed = 71.0 * FRAME_MS / 1000.0;
        total.elapsed = damage.elapsed;
        assert!(damage.is_expired() && !total.is_expired());

        total.elapsed = 121.0 * FRAME_MS / 1000.0;
        assert!(total.is_expired());
    }

    #[test]
    fn a_critical_burst_backs_its_total_with_a_smaller_plate() {
        let mut mgr = DamageNumberManager::new();
        let mut crit = ScheduledHit::multi_hit(50, 100, None, 1, true);
        crit.is_critical = true;
        mgr.emit(1, 0.0, &crit, false, false);
        mgr.emit(
            2,
            0.0,
            &ScheduledHit::multi_hit(50, 100, None, 1, true),
            false,
            false,
        );

        let total = |entity_id: u32| {
            mgr.numbers
                .iter()
                .find(|n| n.entity_id == entity_id && n.number_type.is_total())
                .unwrap()
        };
        let (with_crit, plain) = (total(1), total(2));
        assert_eq!(with_crit.number_type, plain.number_type);
        assert_eq!(with_crit.zoom(), plain.zoom());
        assert!(plain.render_data().unwrap().critical_backdrop.is_none());

        // The plate grows on its own, slower curve and stays under the digits.
        let backdrop = with_crit.render_data().unwrap().critical_backdrop.unwrap();
        assert_eq!(backdrop.zoom, TOTAL_CRIT_ZOOM_START);
        assert_eq!(backdrop.extra_lift, TOTAL_CRIT_LIFT);
        // Riding higher than the digits, it needs its own projected position.
        let plate = with_crit.backdrop_world_offset().unwrap();
        assert_eq!(plate[1], with_crit.world_offset()[1] - TOTAL_CRIT_LIFT);

        let mut grown = DamageNumber::new(1, 100, DamageNumberType::MultiHitTotal, 0.0);
        grown.has_critical = true;
        grown.elapsed = 20.0 * FRAME_MS / 1000.0;
        let grown_zoom = grown.render_data().unwrap().critical_backdrop.unwrap().zoom;
        assert!(grown_zoom > backdrop.zoom && grown_zoom <= TOTAL_CRIT_ZOOM_MAX);
        assert!(grown_zoom < grown.zoom());
    }

    #[test]
    fn lucky_animates_in_place_at_full_opacity() {
        let mut lucky = DamageNumber::new(1, 0, DamageNumberType::Lucky, 0.0);
        assert_eq!(lucky.world_offset(), [0.0, -LUCKY_LIFT, 0.0]);
        assert_eq!(lucky.zoom(), 1.0);
        // The word only joins the backdrop once the action reaches its motion 2.
        assert_eq!(
            lucky.render_data().unwrap().msg_frames,
            vec![MSG_FRAME_LUCKYBG]
        );

        lucky.elapsed = 60.0 * FRAME_MS / 1000.0;
        let data = lucky.render_data().unwrap();
        assert_eq!(data.msg_frames, vec![MSG_FRAME_LUCKYBG, MSG_FRAME_LUCKY]);
        assert_eq!(data.color[3], 1.0);
        // It never moves off its spawn point.
        assert_eq!(lucky.world_offset(), [0.0, -LUCKY_LIFT, 0.0]);
        assert!(!lucky.is_expired());

        lucky.elapsed = 113.0 * FRAME_MS / 1000.0;
        assert!(lucky.is_expired());
    }

    #[test]
    fn combo_has_no_lateral_drift() {
        let d = DamageNumber::new(1, 100, DamageNumberType::Combo, 3.0);
        assert_eq!(d.world_offset()[0], 0.0);
        assert_eq!(d.world_offset()[2], 0.0);
    }
}
