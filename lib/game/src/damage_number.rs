/// Floating damage numbers using GRF sprite textures (숫자.spr / msg.spr).
///
/// Each type maps to a sprite action index, color tint, and animation curve
/// matching the original game behavior.

use crate::scheduled_hit::{DamageMessage, ScheduledHit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageNumberType {
    /// Basic attack → action 1, white
    Normal,
    /// Ability damage → action 0, white
    Skill,
    /// Critical hit → action 0 (with crit overlay), yellow
    Critical,
    /// Player takes damage → action 2, red
    Enemy,
    /// Multi-hit running total (non-final) → action 0, yellow, short-lived
    Combo,
    /// Multi-hit final total → action 0, yellow, grow animation
    ComboFinal,
    /// Non-combo multi-hit running total (non-final) → action 0, white, short-lived
    MultiHit,
    /// Non-combo multi-hit total → action 0, white, grow animation
    MultiHitTotal,
    /// HP recovery → action 3, green
    Heal,
    /// Miss → msg.spr frame 0
    Miss,
    /// Lucky dodge → msg.spr frames 4+5
    Lucky,
}

const DIGIT_SPACING: f32 = 8.0;

/// One stateCnt tick = 24ms in the original game
const FRAME_MS: f32 = 24.0;

impl DamageNumberType {

    pub fn color(&self) -> [f32; 3] {
        match self {
            Self::Critical | Self::Combo | Self::ComboFinal => [0.9, 0.9, 0.15],
            Self::Enemy => [1.0, 0.0, 0.0],
            Self::Heal => [0.0, 1.0, 0.0],
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

    pub fn duration(&self) -> f32 {
        match self {
            Self::Combo | Self::MultiHit => 0.45,
            _ => 120.0 * FRAME_MS / 1000.0,                  // 2.88s
        }
    }
}

pub struct DamageNumber {
    pub entity_id: u32,
    pub value: i32,
    pub number_type: DamageNumberType,
    pub elapsed: f32,
    pub direction: u8,
    /// Cached screen position so numbers keep rendering after entity vanishes.
    pub last_screen_pos: Option<(f32, f32, f32)>,
}

impl DamageNumber {
    pub fn new(entity_id: u32, value: i32, number_type: DamageNumberType, direction: u8) -> Self {
        Self { entity_id, value, number_type, elapsed: 0.0, direction, last_screen_pos: None }
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.number_type.duration()
    }

    /// stateCnt equivalent: elapsed_ms / 24
    fn frame(&self) -> f32 {
        self.elapsed * 1000.0 / FRAME_MS
    }

    /// Y offset from origin (positive = upward in screen space)
    pub fn y_offset(&self) -> f32 {
        let f = self.frame();
        match self.number_type {
            DamageNumberType::ComboFinal | DamageNumberType::MultiHitTotal => {
                f * 0.1
            }
            DamageNumberType::Combo | DamageNumberType::MultiHit => {
                self.elapsed * 2.0
            }
            DamageNumberType::Miss => {
                f * 0.54
            }
            DamageNumberType::Lucky => {
                let perc = self.elapsed / self.number_type.duration();
                perc * 7.0
            }
            DamageNumberType::Heal => {
                let perc = self.elapsed / self.number_type.duration();
                if perc < 0.4 { 0.0 } else { (perc - 0.4) * 300.0 }
            }
            _ => {
                // Parabolic: rises fast then slows. Original: orgY+8 - cnt*(2.0 - cnt/30)
                -8.0 + f * (2.0 - f / 30.0)
            }
        }
    }

    /// X offset from origin (screen pixels, direction-based drift)
    pub fn x_offset(&self) -> f32 {
        let f = self.frame();
        let magnitude = match self.number_type {
            DamageNumberType::Critical => 0.5,
            DamageNumberType::Normal | DamageNumberType::Skill | DamageNumberType::Enemy => 0.8,
            _ => return 0.0,
        };
        // Map direction (0-7) to screen X factor
        // Directions 0-3 drift left, 4-7 drift right
        let dir_x: f32 = if self.direction % 8 < 4 { -1.0 } else { 1.0 };
        dir_x * magnitude * (f / 3.0 + 3.0)
    }

    /// Scale/zoom factor
    pub fn zoom(&self) -> f32 {
        let f = self.frame();
        match self.number_type {
            DamageNumberType::ComboFinal | DamageNumberType::MultiHitTotal => {
                // Original: m_zoom += stateCnt*0.18 (quadratic accumulation from 0.5)
                (0.5 + 0.09 * f * f).min(3.0)
            }
            DamageNumberType::Combo | DamageNumberType::MultiHit => {
                // quick grow to ~3.75 in first 150ms
                let growth = (self.elapsed / 0.15).min(1.0);
                0.1 + growth * 3.65
            }
            DamageNumberType::Critical => {
                (5.0 - f * 0.24).max(1.3)
            }
            DamageNumberType::Miss => 1.0,
            DamageNumberType::Lucky => 0.5,
            DamageNumberType::Heal => {
                let perc = self.elapsed / self.number_type.duration();
                ((1.0 - perc * 2.0) * 3.0).max(0.8)
            }
            _ => {
                (5.0 - f * 0.24).max(1.2)
            }
        }
    }

    /// Alpha 0.0–1.0
    pub fn alpha(&self) -> f32 {
        let f = self.frame();
        let alpha_255 = match self.number_type {
            DamageNumberType::ComboFinal | DamageNumberType::MultiHitTotal => {
                if f < 90.0 { 250.0 } else { 250.0 - (f - 90.0) * 8.0 }
            }
            DamageNumberType::Combo | DamageNumberType::MultiHit => {
                // alpha = 1.0 - (elapsed / 3.0)
                (1.0 - self.elapsed / 3.0) * 255.0
            }
            _ => 255.0 - f * 2.0,
        };
        (alpha_255 / 255.0).clamp(0.0, 1.0)
    }

    /// Digits of the value, left-to-right
    pub fn digits(&self) -> Vec<u8> {
        if self.value == 0 {
            return vec![0];
        }
        let clamped = self.value.unsigned_abs().min(999999);
        let s = clamped.to_string();
        s.bytes().rev().map(|b| b - b'0').collect()
    }

    /// X offset for digit at index i (0 = leftmost), centered around 0
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
        let [cr, cg, cb] = self.number_type.color();
        let digits = self.digits();
        let count = digits.len();
        let digit_x_offsets = (0..count).map(|i| self.digit_x_offset(i, count)).collect();
        let msg_frames = match self.number_type {
            DamageNumberType::Miss => vec![MSG_FRAME_MISS],
            DamageNumberType::Lucky => vec![MSG_FRAME_LUCKYBG, MSG_FRAME_LUCKY],
            _ => vec![],
        };
        Some(DamageNumberRenderData {
            digits,
            digit_x_offsets,
            color: [cr, cg, cb, alpha],
            zoom: self.zoom(),
            y_offset: self.y_offset(),
            x_offset: self.x_offset(),
            uses_msg_sprite: self.number_type.uses_msg_sprite(),
            msg_frames,
            is_critical: matches!(self.number_type, DamageNumberType::Critical),
        })
    }
}

pub struct DamageNumberRenderData {
    pub digits: Vec<u8>,
    pub digit_x_offsets: Vec<f32>,
    pub color: [f32; 4],
    pub zoom: f32,
    pub y_offset: f32,
    pub x_offset: f32,
    pub uses_msg_sprite: bool,
    pub msg_frames: Vec<usize>,
    pub is_critical: bool,
}

pub struct DamageNumberRenderEntry {
    pub entity_id: u32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub scale: f32,
    pub data: DamageNumberRenderData,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum TextureSource {
    Number,
    Message,
}

#[repr(C)]
pub struct DamageNumberQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
    pub source: TextureSource,
    pub tex_idx: usize,
}

pub fn build_damage_number_quads(
    entries: &[DamageNumberRenderEntry],
    num_act: &ragnarok_formats::act::ActFile,
    num_sizes: &[(u32, u32)],
    num_indexed_count: usize,
    msg_sizes: Option<&[(u32, u32)]>,
) -> Vec<DamageNumberQuad> {
    let mut quads = Vec::new();
    for entry in entries {
        let dmg = &entry.data;
        let s = entry.scale;
        let base_x = entry.screen_x + dmg.x_offset * s;
        let base_y = entry.screen_y - 10.0 * s - dmg.y_offset * s;
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
                    x, y, w: sw, h: sh,
                    color: [cr, cg, cb, alpha],
                    source: TextureSource::Message,
                    tex_idx: frame_idx,
                });
            }
            continue;
        }

        let action = &num_act.actions[0];

        // Critical: render critbg behind digits
        if dmg.is_critical {
            if let Some(msg_sz) = msg_sizes {
                if MSG_FRAME_CRITBG < msg_sz.len() {
                    let (tw, th) = msg_sz[MSG_FRAME_CRITBG];
                    let crit_zoom = 0.6 * zoom;
                    let sw = tw as f32 * crit_zoom;
                    let sh = th as f32 * crit_zoom;
                    let x = base_x - sw / 2.0;
                    let y = base_y - sh / 2.0 - 6.0 * s;
                    quads.push(DamageNumberQuad {
                        x, y, w: sw, h: sh,
                        color: [0.66, 0.66, 0.66, alpha],
                        source: TextureSource::Message,
                        tex_idx: MSG_FRAME_CRITBG,
                    });
                }
            }
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

            let x_offset = dmg.digit_x_offsets.get(i).copied().unwrap_or(0.0) * zoom;
            let x = base_x + x_offset - sw / 2.0;
            let y = base_y - sh / 2.0;

            quads.push(DamageNumberQuad {
                x, y, w: sw, h: sh,
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
}

impl DamageNumberManager {
    pub fn new() -> Self {
        Self { numbers: Vec::new() }
    }

    pub fn add(&mut self, number: DamageNumber) {
        // Combo/total types replace previous non-final combo numbers on the same entity
        let removes_combo = number.number_type.is_total() || number.number_type.is_combo();
        if removes_combo {
            self.numbers.retain(|n| {
                !(n.entity_id == number.entity_id && n.number_type.is_combo())
            });
        }
        self.numbers.push(number);
    }

    pub fn emit(&mut self, entity_id: u32, direction: u8, hit: &ScheduledHit, is_player_target: bool) {
        let is_multi_hit = matches!(hit.message, DamageMessage::AttackedMultiHit { .. });
        let is_skill = hit.skill_id > 0;

        if hit.damage == 0 && !is_multi_hit {
            self.add(DamageNumber::new(entity_id, 0, DamageNumberType::Miss, direction));
            return;
        }

        if is_multi_hit {
            self.add(DamageNumber::new(
                entity_id, hit.damage, DamageNumberType::Skill, direction,
            ));
            let running_total = hit.damage * (hit.hit_index as i32 + 1);
            let combo_type = if hit.is_last_hit {
                if is_skill { DamageNumberType::ComboFinal } else { DamageNumberType::MultiHitTotal }
            } else {
                if is_skill { DamageNumberType::Combo } else { DamageNumberType::MultiHit }
            };
            self.add(DamageNumber::new(entity_id, running_total, combo_type, direction));
        } else {
            let number_type = if hit.is_critical {
                DamageNumberType::Critical
            } else if hit.damage < 0 {
                DamageNumberType::Heal
            } else if is_skill {
                DamageNumberType::Skill
            } else if is_player_target {
                DamageNumberType::Enemy
            } else {
                DamageNumberType::Normal
            };
            self.add(DamageNumber::new(
                entity_id, hit.damage.abs(), number_type, direction,
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

/// msg.spr frame indices
pub const MSG_FRAME_MISS: usize = 0;
pub const MSG_FRAME_CRIT: usize = 2;
pub const MSG_FRAME_CRITBG: usize = 3;
pub const MSG_FRAME_LUCKYBG: usize = 4;
pub const MSG_FRAME_LUCKY: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_decomposition() {
        let d = DamageNumber::new(1, 12345, DamageNumberType::Normal, 0);
        // Reversed order: rightmost digit first, matching digit_x_offset layout
        assert_eq!(d.digits(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn digits_zero() {
        let d = DamageNumber::new(1, 0, DamageNumberType::Miss, 0);
        assert_eq!(d.digits(), vec![0]);
    }

    #[test]
    fn digits_clamped_to_999999() {
        let d = DamageNumber::new(1, 1_500_000, DamageNumberType::Normal, 0);
        assert_eq!(d.digits(), vec![9, 9, 9, 9, 9, 9]);
    }

    #[test]
    fn digit_x_offsets_symmetric_for_3_digits() {
        let d = DamageNumber::new(1, 123, DamageNumberType::Normal, 0);
        let offsets: Vec<f32> = (0..3).map(|i| d.digit_x_offset(i, 3)).collect();
        // Should be symmetric: [+8, 0, -8]
        assert_eq!(offsets, vec![8.0, 0.0, -8.0]);
    }

    #[test]
    fn normal_starts_large_shrinks() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::Normal, 0);
        let z0 = d.zoom();
        d.elapsed = 0.5;
        let z1 = d.zoom();
        assert!(z0 > z1, "zoom should decrease over time");
        assert!(z1 >= 1.2, "zoom should not go below 1.2");
    }

    #[test]
    fn total_starts_small_grows() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0);
        let z0 = d.zoom();
        d.elapsed = 0.5;
        let z1 = d.zoom();
        assert!(z0 < z1, "total zoom should increase over time");
        assert!(z0 >= 0.5, "total starts at 0.5");
    }

    #[test]
    fn total_alpha_holds_then_fades() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0);
        // frame 90 = 90 * 24ms = 2.16s
        d.elapsed = 2.0; // frame ~83 < 90
        assert!(d.alpha() > 0.95, "should be nearly opaque before frame 90");
        d.elapsed = 2.5; // frame ~104 > 90
        assert!(d.alpha() < 0.95, "should be fading after frame 90");
    }

    #[test]
    fn critical_color_is_yellow() {
        assert_eq!(DamageNumberType::Critical.color(), [0.9, 0.9, 0.15]);
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
    fn combo_replaces_previous_combo_on_same_entity() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 50, DamageNumberType::Combo, 0));
        assert_eq!(mgr.numbers.len(), 1);

        // New combo replaces old one
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Combo, 0));
        assert_eq!(mgr.numbers.len(), 1);
        assert_eq!(mgr.numbers[0].value, 100);
    }

    #[test]
    fn manager_removes_combo_on_total() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 50, DamageNumberType::Combo, 0));
        mgr.add(DamageNumber::new(2, 50, DamageNumberType::Combo, 0));
        assert_eq!(mgr.numbers.len(), 2);

        mgr.add(DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0));
        // Removed combo for entity 1, kept combo for entity 2, added the total
        assert_eq!(mgr.numbers.len(), 2);
        assert!(mgr.numbers.iter().any(|n| n.entity_id == 2));
        assert!(mgr.numbers.iter().any(|n| n.number_type == DamageNumberType::ComboFinal));
    }

    #[test]
    fn expired_numbers_removed_on_update() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Normal, 0));
        mgr.update(3.0); // well past 2.88s duration
        assert!(mgr.numbers.is_empty());
    }

    #[test]
    fn x_offset_direction_based() {
        let d_left = DamageNumber::new(1, 100, DamageNumberType::Normal, 1);
        let d_right = DamageNumber::new(1, 100, DamageNumberType::Normal, 5);
        // Direction 1 (dirs 0-3) drifts left, direction 5 (dirs 4-7) drifts right
        assert!(d_left.x_offset() < 0.0);
        assert!(d_right.x_offset() > 0.0);
    }

    #[test]
    fn combo_has_no_x_offset() {
        let d = DamageNumber::new(1, 100, DamageNumberType::Combo, 3);
        assert_eq!(d.x_offset(), 0.0);
    }
}
