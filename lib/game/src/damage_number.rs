use crate::scheduled_hit::{DamageMessage, ScheduledHit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageNumberType {
    Normal,
    Skill,
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
const FRAME_MS: f32 = 24.0; // ms per animation tick
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
/// Skill damage, criticals and running multi-hit totals share one colour; a
/// plain weapon blow stays white and anything landing on a player goes red.
const SKILL_YELLOW: [f32; 3] = [0.9, 0.9, 0.15];
/// Lucky plays `msg.act` action 3 once: 28 motions held 4 frames each. The word
/// itself only joins the backdrop from motion 2 on.
const LUCKY_MOTIONS: f32 = 28.0;
const LUCKY_FRAMES_PER_MOTION: f32 = 4.0;
const LUCKY_WORD_FROM_FRAME: f32 = 2.0 * LUCKY_FRAMES_PER_MOTION;
/// The critical backdrop under a multi-hit total, in zoom units per frame².
const TOTAL_CRIT_ZOOM_START: f32 = 0.5;
const TOTAL_CRIT_ZOOM_ACCEL: f32 = 0.06;
const TOTAL_CRIT_ZOOM_MAX: f32 = 2.0;
/// Screen units a critical backdrop sits above its digits, before entity scale.
const TOTAL_CRIT_LIFT: f32 = 15.0;
const CRIT_LIFT: f32 = 6.0;
/// The single-hit critical backdrop tracks its digits' zoom at this fraction.
const CRIT_ZOOM_RATIO: f32 = 0.6;

impl DamageNumberType {
    pub fn color(&self) -> [f32; 3] {
        match self {
            Self::Enemy => PLAYER_RED,
            Self::Heal => [0.0, 1.0, 0.0],
            Self::Skill | Self::Critical => SKILL_YELLOW,
            t if t.is_combo() || t.is_total() => SKILL_YELLOW,
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
                | Self::Skill
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
            Self::Normal | Self::Skill | Self::Critical | Self::Enemy => 70.0,
            Self::Miss => 80.0,
            Self::Lucky => LUCKY_MOTIONS * LUCKY_FRAMES_PER_MOTION,
            _ => 120.0,
        }
    }

    pub fn duration(&self) -> f32 {
        self.life_frames() * FRAME_MS / 1000.0
    }
}

pub struct DamageNumber {
    pub entity_id: u32,
    pub value: i32,
    pub number_type: DamageNumberType,
    pub elapsed: f32,
    pub direction: u8,
    pub last_screen_pos: Option<(f32, f32, f32)>,
    /// RGB override; falls back to `number_type.color()`.
    pub color_override: Option<[f32; 3]>,
    /// A multi-hit burst that landed a critical: the running total keeps its own
    /// size but gains the critical backdrop.
    pub has_critical: bool,
}

impl DamageNumber {
    pub fn new(entity_id: u32, value: i32, number_type: DamageNumberType, direction: u8) -> Self {
        Self {
            entity_id,
            value,
            number_type,
            elapsed: 0.0,
            direction,
            last_screen_pos: None,
            color_override: None,
            has_critical: false,
        }
    }

    pub fn effect_number(entity_id: u32, value: i32, color: [f32; 3], direction: u8) -> Self {
        Self {
            color_override: Some(color),
            ..Self::new(entity_id, value, DamageNumberType::EffectNumber, direction)
        }
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.number_type.duration()
    }

    fn frame(&self) -> f32 {
        self.elapsed * 1000.0 / FRAME_MS
    }

    pub fn y_offset(&self) -> f32 {
        let f = self.frame();
        match self.number_type {
            t if t.is_combo() || t.is_total() => f * 0.1,
            DamageNumberType::Miss => f * 0.54,
            DamageNumberType::Lucky => 0.0,
            DamageNumberType::Heal | DamageNumberType::EffectNumber => {
                let perc = self.elapsed / self.number_type.duration();
                if perc < 0.4 {
                    0.0
                } else {
                    (perc - 0.4) * 300.0
                }
            }
            _ => -8.0 + f * (2.0 - f / 30.0),
        }
    }

    pub fn x_offset(&self) -> f32 {
        let f = self.frame();
        let magnitude = match self.number_type {
            DamageNumberType::Critical => 0.5,
            DamageNumberType::Normal | DamageNumberType::Skill | DamageNumberType::Enemy => 0.8,
            _ => return 0.0,
        };
        let dir_x: f32 = if self.direction % 8 < 4 { -1.0 } else { 1.0 };
        dir_x * magnitude * (f / 3.0 + 3.0)
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
            DamageNumberType::Heal | DamageNumberType::EffectNumber => {
                let perc = self.elapsed / self.number_type.duration();
                ((1.0 - perc * 2.0) * 3.0).max(0.8)
            }
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
            DamageNumberType::Normal
            | DamageNumberType::Skill
            | DamageNumberType::Critical
            | DamageNumberType::Enemy => 250.0 - f * DAMAGE_FADE_PER_FRAME,
            _ => 250.0 - f * RECOVERY_FADE_PER_FRAME,
        };
        (alpha_255 / 255.0).clamp(0.0, 1.0)
    }

    fn critical_backdrop(&self) -> Option<CriticalBackdrop> {
        if self.number_type == DamageNumberType::Critical {
            return Some(CriticalBackdrop {
                zoom: CRIT_ZOOM_RATIO * self.zoom(),
                lift: CRIT_LIFT,
            });
        }
        if !self.has_critical {
            return None;
        }
        let f = self.frame();
        Some(CriticalBackdrop {
            zoom: (TOTAL_CRIT_ZOOM_START + TOTAL_CRIT_ZOOM_ACCEL * f * f).min(TOTAL_CRIT_ZOOM_MAX),
            lift: TOTAL_CRIT_LIFT,
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
            y_offset: self.y_offset(),
            x_offset: self.x_offset(),
            uses_msg_sprite: self.number_type.uses_msg_sprite(),
            msg_frames,
            critical_backdrop: self.critical_backdrop(),
        })
    }
}

/// The `msg.spr` critical plate drawn behind a number, sized and lifted
/// independently of the digits it backs.
pub struct CriticalBackdrop {
    pub zoom: f32,
    pub lift: f32,
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
    pub critical_backdrop: Option<CriticalBackdrop>,
}

pub struct DamageNumberRenderEntry {
    pub entity_id: u32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub scale: f32,
    pub data: DamageNumberRenderData,
}

pub use ragnarok_formats::damage_number::{DamageNumberQuad, TextureSource};

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
            let crit_zoom = backdrop.zoom * s;
            let sw = tw as f32 * crit_zoom;
            let sh = th as f32 * crit_zoom;
            let x = base_x - sw / 2.0;
            let y = base_y - sh / 2.0 - backdrop.lift * s;
            quads.push(DamageNumberQuad {
                x,
                y,
                w: sw,
                h: sh,
                color: [0.66, 0.66, 0.66, alpha],
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

            let x_offset = dmg.digit_x_offsets.get(i).copied().unwrap_or(0.0) * zoom;
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
        direction: u8,
        hit: &ScheduledHit,
        is_player_target: bool,
        attacker_is_player: bool,
    ) {
        let is_multi_hit = matches!(hit.message, DamageMessage::AttackedMultiHit { .. });
        let is_skill = hit.skill_id > 0;

        if hit.damage < 0 {
            return;
        }

        let total_zero = match hit.message {
            DamageMessage::AttackedMultiHit { total_damage } => total_damage == 0,
            _ => hit.damage == 0,
        };
        if total_zero {
            if hit.hit_index == 0 {
                let mut miss = DamageNumber::new(entity_id, 0, DamageNumberType::Miss, direction);
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
                direction,
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
            let mut number = DamageNumber::new(entity_id, running_total, combo_type, direction);
            number.has_critical = hit.is_critical;
            self.add(number);
        } else {
            let number_type = if hit.is_critical {
                DamageNumberType::Critical
            } else if is_player_target {
                DamageNumberType::Enemy
            } else if is_skill {
                DamageNumberType::Skill
            } else {
                DamageNumberType::Normal
            };
            self.add(DamageNumber::new(
                entity_id,
                hit.damage.abs(),
                number_type,
                direction,
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

    #[test]
    fn digits_decomposition() {
        let d = DamageNumber::new(1, 12345, DamageNumberType::Normal, 0);
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
        assert_eq!(offsets, vec![8.0, 0.0, -8.0]);
    }

    #[test]
    fn normal_starts_large_shrinks() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::Normal, 0);
        let z0 = d.zoom();
        d.elapsed = 0.5;
        let z1 = d.zoom();
        assert!(z0 > z1);
        assert!(z1 >= 1.2);
    }

    #[test]
    fn total_starts_small_grows() {
        let mut d = DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0);
        let z0 = d.zoom();
        d.elapsed = 0.5;
        let z1 = d.zoom();
        assert!(z0 < z1);
        assert!(z0 >= 0.5);
    }

    #[test]
    fn critical_color_is_yellow() {
        assert_eq!(DamageNumberType::Critical.color(), SKILL_YELLOW);
    }

    #[test]
    fn a_double_attack_reads_like_a_skill_combo_not_like_plain_damage() {
        assert_eq!(DamageNumberType::MultiHit.color(), SKILL_YELLOW);
        assert_eq!(
            DamageNumberType::MultiHitTotal.color(),
            DamageNumberType::ComboFinal.color()
        );
        assert_ne!(
            DamageNumberType::MultiHit.color(),
            DamageNumberType::Normal.color()
        );

        let mut plain = DamageNumber::new(1, 120, DamageNumberType::Normal, 0);
        let mut by_skill = DamageNumber::new(1, 120, DamageNumberType::Combo, 0);
        let mut by_weapon = DamageNumber::new(1, 120, DamageNumberType::MultiHit, 0);
        assert_eq!(by_weapon.zoom(), TOTAL_ZOOM_START);

        for n in [&mut plain, &mut by_skill, &mut by_weapon] {
            n.elapsed = 5.0 * FRAME_MS / 1000.0;
        }
        // A multi-hit swells and climbs on the total's curve whatever caused it,
        // while plain damage shrinks from its opening flash.
        assert_eq!(by_weapon.zoom(), by_skill.zoom());
        assert!(by_weapon.zoom() > TOTAL_ZOOM_START);
        assert!(plain.zoom() < 5.0);
        assert_eq!(by_weapon.y_offset(), by_skill.y_offset());
        assert!(by_weapon.y_offset() > 0.0);
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
    fn skill_hit_on_player_is_red_but_on_monster_is_skill() {
        let hit = ScheduledHit::single(100, 17, false);

        let mut player = DamageNumberManager::new();
        player.emit(1, 0, &hit, true, false);
        assert_eq!(
            player.numbers.last().unwrap().number_type,
            DamageNumberType::Enemy
        );

        let mut monster = DamageNumberManager::new();
        monster.emit(1, 0, &hit, false, false);
        let on_monster = monster.numbers.last().unwrap();
        assert_eq!(on_monster.number_type, DamageNumberType::Skill);
        assert_eq!(on_monster.number_type.color(), SKILL_YELLOW);

        let weapon_hit = ScheduledHit::single(100, 0, false);
        let mut plain = DamageNumberManager::new();
        plain.emit(1, 0, &weapon_hit, false, false);
        let blow = plain.numbers.last().unwrap();
        assert_eq!(blow.number_type, DamageNumberType::Normal);
        assert_eq!(blow.number_type.color(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn fully_missed_multi_hit_shows_single_miss() {
        let mut mgr = DamageNumberManager::new();
        mgr.emit(
            1,
            0,
            &ScheduledHit::multi_hit(0, 0, 10, 0, false),
            false,
            false,
        );
        mgr.emit(
            1,
            0,
            &ScheduledHit::multi_hit(0, 0, 10, 1, true),
            false,
            false,
        );
        assert_eq!(mgr.numbers.len(), 1);
        assert_eq!(mgr.numbers[0].number_type, DamageNumberType::Miss);
    }

    #[test]
    fn a_players_miss_is_red_a_monsters_stays_white() {
        let miss = ScheduledHit::single(0, 0, false);

        let mut by_player = DamageNumberManager::new();
        by_player.emit(1, 0, &miss, false, true);
        assert_eq!(
            by_player.numbers[0].render_data().unwrap().color[..3],
            PLAYER_RED
        );

        let mut by_monster = DamageNumberManager::new();
        by_monster.emit(1, 0, &miss, true, false);
        assert_eq!(
            by_monster.numbers[0].render_data().unwrap().color[..3],
            [1.0, 1.0, 1.0]
        );
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
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Normal, 0));
        mgr.add(DamageNumber::new(2, 50, DamageNumberType::Combo, 0));
        mgr.clear();
        assert!(mgr.numbers.is_empty());
    }

    #[test]
    fn expired_numbers_removed_on_update() {
        let mut mgr = DamageNumberManager::new();
        mgr.add(DamageNumber::new(1, 100, DamageNumberType::Normal, 0));
        mgr.update(3.0);
        assert!(mgr.numbers.is_empty());
    }

    #[test]
    fn x_offset_direction_based() {
        let d_left = DamageNumber::new(1, 100, DamageNumberType::Normal, 1);
        let d_right = DamageNumber::new(1, 100, DamageNumberType::Normal, 5);
        assert!(d_left.x_offset() < 0.0);
        assert!(d_right.x_offset() > 0.0);
    }

    #[test]
    fn combat_hidden_suppresses_damage_but_keeps_miss_and_heal() {
        let mut mgr = DamageNumberManager::new();
        mgr.combat_hidden = true;

        mgr.emit(1, 0, &ScheduledHit::single(100, 0, false), false, false);
        assert!(mgr.numbers.is_empty());

        mgr.emit(1, 0, &ScheduledHit::single(0, 0, false), false, false);
        assert_eq!(
            mgr.numbers.last().unwrap().number_type,
            DamageNumberType::Miss
        );

        mgr.add(DamageNumber::new(1, 42, DamageNumberType::Heal, 0));
        assert!(
            mgr.numbers
                .iter()
                .any(|n| n.number_type == DamageNumberType::Heal)
        );

        mgr.combat_hidden = false;
        let before = mgr.numbers.len();
        mgr.emit(1, 0, &ScheduledHit::single(100, 0, false), false, false);
        assert_eq!(mgr.numbers.len(), before + 1);
    }

    #[test]
    fn a_multi_hit_keeps_every_blow_and_one_running_total() {
        let mut mgr = DamageNumberManager::new();
        for (index, last) in [(0u16, false), (1, false), (2, true)] {
            mgr.emit(
                1,
                0,
                &ScheduledHit::multi_hit(30, 90, 17, index, last),
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
        assert_eq!(totals[0].number_type.color(), SKILL_YELLOW);
    }

    #[test]
    fn damage_fades_out_faster_and_earlier_than_a_total() {
        let mut damage = DamageNumber::new(1, 100, DamageNumberType::Normal, 0);
        let mut total = DamageNumber::new(1, 100, DamageNumberType::ComboFinal, 0);

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
        let mut crit = ScheduledHit::multi_hit(50, 100, 0, 1, true);
        crit.is_critical = true;
        mgr.emit(1, 0, &crit, false, false);
        mgr.emit(
            2,
            0,
            &ScheduledHit::multi_hit(50, 100, 0, 1, true),
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
        assert_eq!(backdrop.lift, TOTAL_CRIT_LIFT);

        let mut grown = DamageNumber::new(1, 100, DamageNumberType::MultiHitTotal, 0);
        grown.has_critical = true;
        grown.elapsed = 20.0 * FRAME_MS / 1000.0;
        let grown_zoom = grown.render_data().unwrap().critical_backdrop.unwrap().zoom;
        assert!(grown_zoom > backdrop.zoom && grown_zoom <= TOTAL_CRIT_ZOOM_MAX);
        assert!(grown_zoom < grown.zoom());
    }

    #[test]
    fn lucky_animates_in_place_at_full_opacity() {
        let mut lucky = DamageNumber::new(1, 0, DamageNumberType::Lucky, 0);
        assert_eq!(lucky.y_offset(), 0.0);
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
        assert_eq!(data.y_offset, 0.0);
        assert!(!lucky.is_expired());

        lucky.elapsed = 113.0 * FRAME_MS / 1000.0;
        assert!(lucky.is_expired());
    }

    #[test]
    fn combo_has_no_x_offset() {
        let d = DamageNumber::new(1, 100, DamageNumberType::Combo, 3);
        assert_eq!(d.x_offset(), 0.0);
    }
}
