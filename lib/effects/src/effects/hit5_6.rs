//! `EF_HIT5` / `EF_HIT6` — critical-strike sparkle.
//!
//! Both effects spawn two
//! billboards at frame 0
//! arranged in a cross (the second petal offset 90° from the first).
//! Each petal:
//!
//!   * Starts at a **random** screen-space roll angle (first =
//!     `random(360)`, second = first + 90°).
//!   * Rotates over time (roll speed -5°/frame, decelerated by
//!     -(roll speed / duration) / 1.5).
//!   * Shrinks in width and grows tall — same width-shrink + height-grow
//!     pattern as Hit2, but with no radial outward translation.
//!   * Fades in to peak then fades out over the back half
//!     (fade-out begins at duration/2).
//!   * Uses `lens2.tga`.
//!
//! Hit5 and Hit6 differ only in size and height-growth speed: Hit5 is
//! larger and more dramatic, Hit6 smaller and subtler.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const LENS2: &str = "lens2.tga";
pub const TEXTURES: &[&str] = &[LENS2];

const FRAMES_PER_SECOND: f32 = 60.0;
const PETAL_COUNT: usize = 2;

/// Lift the cross off the ground to chest level. The original lifts
/// it further; the same compensation as Hit2
/// applies (our viewer's `world_pos` is at the entity's ground anchor).
const Y_OFFSET_BASE: f32 = -10.0;

/// Linear scale factor on the width/height
/// values — same calibration as Hit2.
const SIZE_SCALE: f32 = 1.0 / 3.0;

/// Roll speed -5°/frame. CCW positive in
/// screen space, so a negative roll = clockwise
/// rotation in our convention.
const ROLL_SPEED_DEG_PER_FRAME: f32 = -5.0;

#[derive(Clone, Copy, Debug)]
pub struct HitCrossParams {
    pub width_min: f32,
    pub width_max: f32,
    pub height_min: f32,
    pub height_max: f32,
    /// Initial height-growth speed (per-frame at 60 fps) — Hit5 = 2.5,
    /// Hit6 = 1.7.
    pub height_speed_init_orig: f32,
    /// Height-growth accel — both variants use 0.5.
    pub height_accel_orig: f32,
    /// Duration — both variants use 17 frames.
    pub duration_frames: f32,
    /// Constant rotation offset added to the first petal's roll.
    /// The original randomises the roll for both Hit5
    /// and Hit6, but the reference gifs (`imgs/0-50/4.gif` /
    /// `imgs/0-50/5.gif`) consistently show Hit5 as a diagonal "×"
    /// and Hit6 as an axis-aligned "+". We pin each variant to its
    /// reference orientation. See the per-`const` comments for the
    /// concrete values — they depend on `lens2.tga`'s native texture
    /// orientation, not on screen geometry alone.
    pub base_roll_offset_rad: f32,
}

pub const HIT5: HitCrossParams = HitCrossParams {
    width_min: 20.0 * SIZE_SCALE,
    width_max: 25.0 * SIZE_SCALE,
    height_min: 30.0 * SIZE_SCALE,
    height_max: 40.0 * SIZE_SCALE,
    height_speed_init_orig: 2.5,
    height_accel_orig: 0.5,
    duration_frames: 17.0,
    // Diagonal "×". The `lens2.tga` texture's bright rays run along
    // its diagonals (each petal looks like an X already), so a roll
    // of 0 makes the two perpendicular petals' rays align as one
    // larger "×". `π/4` rotates each petal so the rays land on the
    // screen-vertical and screen-horizontal axes, giving "+".
    base_roll_offset_rad: 0.0,
};

pub const HIT6: HitCrossParams = HitCrossParams {
    width_min: 10.0 * SIZE_SCALE,
    width_max: 15.0 * SIZE_SCALE,
    height_min: 15.0 * SIZE_SCALE,
    height_max: 20.0 * SIZE_SCALE,
    height_speed_init_orig: 1.7,
    height_accel_orig: 0.5,
    duration_frames: 17.0,
    // Axis-aligned "+": petals rolled by 45° so the texture's
    // diagonal rays land on the screen axes.
    base_roll_offset_rad: std::f32::consts::FRAC_PI_4,
};

pub const HIT5_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT5);
pub const HIT6_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT6);

const fn total_duration_ms(p: HitCrossParams) -> u32 {
    (p.duration_frames / FRAMES_PER_SECOND * 1000.0) as u32
}

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy)]
struct Petal {
    /// Current roll angle in screen space (radians).
    roll_rad: f32,
    /// Per-frame angular velocity (radians/frame).
    /// Integrated each tick using the roll accel.
    roll_speed_per_frame: f32,
    /// Constant per-frame roll accel. Decelerates `roll_speed`.
    roll_accel_per_frame: f32,
    width: f32,
    height: f32,
    /// Per-second width-shrink rate (negative).
    width_speed_world_per_s: f32,
    /// Integrated per frame from a height speed
    /// and accel.
    height_speed_per_frame: f32,
    height_accel_per_frame: f32,
}

pub struct HitCrossEffect {
    world_pos: [f32; 3],
    params: HitCrossParams,
    petals: [Petal; PETAL_COUNT],
    age: f32,
    lifetime: f32,
    has_spawned: bool,
    rng_state: u32,
}

impl HitCrossEffect {
    pub fn new(world_pos: [f32; 3], params: HitCrossParams) -> Self {
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13)
            ^ params.duration_frames.to_bits().rotate_left(7);
        let lifetime = params.duration_frames / FRAMES_PER_SECOND;
        Self {
            world_pos,
            params,
            // Two zero-initialised petals; `spawn_petals` fills them in
            // on the first update tick.
            petals: [
                Petal {
                    roll_rad: 0.0,
                    roll_speed_per_frame: 0.0,
                    roll_accel_per_frame: 0.0,
                    width: 0.0,
                    height: 0.0,
                    width_speed_world_per_s: 0.0,
                    height_speed_per_frame: 0.0,
                    height_accel_per_frame: 0.0,
                },
                Petal {
                    roll_rad: 0.0,
                    roll_speed_per_frame: 0.0,
                    roll_accel_per_frame: 0.0,
                    width: 0.0,
                    height: 0.0,
                    width_speed_world_per_s: 0.0,
                    height_speed_per_frame: 0.0,
                    height_accel_per_frame: 0.0,
                },
            ],
            age: 0.0,
            lifetime,
            has_spawned: false,
            rng_state,
        }
    }

    fn spawn_petals(&mut self) {
        // A random initial roll would make the
        // cross orientation random per-spawn, but the reference gifs
        // pin Hit5 as "×" and Hit6 as "+". Use the variant's fixed
        // `base_roll_offset_rad` to honour the reference orientation;
        // the second petal stays 90° offset (first roll +
        // 90°).
        let base_roll = self.params.base_roll_offset_rad;
        let roll_speed_per_frame = ROLL_SPEED_DEG_PER_FRAME.to_radians();
        // Roll accel = -(roll speed / duration) / 1.5 (per-frame).
        let roll_accel_per_frame =
            -(roll_speed_per_frame / self.params.duration_frames) / 1.5;
        for k in 0..PETAL_COUNT {
            let width = self.params.width_min
                + lcg_float(&mut self.rng_state)
                    * (self.params.width_max - self.params.width_min);
            let height = self.params.height_min
                + lcg_float(&mut self.rng_state)
                    * (self.params.height_max - self.params.height_min);
            self.petals[k] = Petal {
                roll_rad: base_roll + (k as f32) * std::f32::consts::FRAC_PI_2,
                roll_speed_per_frame,
                roll_accel_per_frame,
                width,
                height,
                // Width speed = -width / duration per-frame ->
                // per-second by × 60.
                width_speed_world_per_s: -width / self.lifetime,
                height_speed_per_frame: self.params.height_speed_init_orig
                    * SIZE_SCALE,
                height_accel_per_frame: self.params.height_accel_orig * SIZE_SCALE,
            };
        }
    }

    fn step_petals(&mut self, dt: f32) {
        let dt_frames = dt * FRAMES_PER_SECOND;
        for p in &mut self.petals {
            p.roll_speed_per_frame += p.roll_accel_per_frame * dt_frames;
            p.roll_rad += p.roll_speed_per_frame * dt_frames;
            p.width = (p.width + p.width_speed_world_per_s * dt).max(0.0);
            p.height_speed_per_frame += p.height_accel_per_frame * dt_frames;
            p.height += p.height_speed_per_frame * dt_frames;
        }
    }

    /// Linear fade-in over the first half, linear fade-out over the
    /// back half — fade-out begins at
    /// duration/2.
    fn alpha(&self) -> f32 {
        let frame = self.age * FRAMES_PER_SECOND;
        let fade_out_at = self.params.duration_frames / 2.0;
        if frame <= fade_out_at {
            (frame / fade_out_at).clamp(0.0, 1.0)
        } else {
            let remaining = (self.params.duration_frames - fade_out_at).max(1e-3);
            (1.0 - (frame - fade_out_at) / remaining).clamp(0.0, 1.0)
        }
    }
}

impl Effect for HitCrossEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if !self.has_spawned {
            self.spawn_petals();
            self.has_spawned = true;
        }
        self.age += ctx.delta;
        self.step_petals(ctx.delta);
        if self.age >= self.lifetime {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let pos = [
            self.world_pos[0],
            self.world_pos[1] + Y_OFFSET_BASE,
            self.world_pos[2],
        ];
        for p in &self.petals {
            if p.width <= 0.0 || p.height <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [p.width, p.height],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: p.roll_rad,
                texture: LENS2,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    #[test]
    fn spawns_two_petals_at_90deg_offset() {
        let mut e = HitCrossEffect::new([0.0; 3], HIT5);
        e.update(&ctx(1.0 / 60.0));
        // Spawn pass ran on first update — the two petals must differ
        // by exactly π/2 (90°) at frame 0 (modulo the small first-tick
        // roll integration).
        let dr = (e.petals[1].roll_rad - e.petals[0].roll_rad).abs();
        // After 1 frame both petals have the same per-tick roll delta,
        // so their absolute difference is still π/2 ± 0.
        assert!(
            (dr - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
            "petals 90° apart: dr={dr}"
        );
    }

    #[test]
    fn hit5_petals_larger_than_hit6_petals() {
        let mut h5 = HitCrossEffect::new([0.0; 3], HIT5);
        let mut h6 = HitCrossEffect::new([0.0; 3], HIT6);
        h5.update(&ctx(0.0));
        h6.update(&ctx(0.0));
        // The narrower-of-Hit5 must still exceed the widest-of-Hit6,
        // because the size ranges don't overlap (Hit5 width: 20..25 vs
        // Hit6 width: 10..15 in unscaled units).
        assert!(
            h5.params.width_min > h6.params.width_max,
            "Hit5 width range strictly above Hit6"
        );
        assert!(
            h5.params.height_min > h6.params.height_max,
            "Hit5 height range strictly above Hit6"
        );
    }

    #[test]
    fn petals_rotate_and_resize_over_time() {
        let mut e = HitCrossEffect::new([0.0; 3], HIT5);
        e.update(&ctx(0.0));
        let r0 = e.petals[0].roll_rad;
        let w0 = e.petals[0].width;
        let h0 = e.petals[0].height;
        // Step ~half the petal's lifetime.
        for _ in 0..8 {
            e.update(&ctx(1.0 / 60.0));
        }
        let r1 = e.petals[0].roll_rad;
        assert!(
            (r1 - r0).abs() > 1e-3,
            "roll changed after 8 frames (rotates over time)"
        );
        assert!(e.petals[0].width < w0, "width shrinks");
        assert!(e.petals[0].height > h0, "height grows");
    }

    #[test]
    fn effect_dies_after_duration() {
        let mut e = HitCrossEffect::new([0.0; 3], HIT5);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        // Hit5 duration = 17 frames ≈ 0.28 s; run for 1 s.
        while t < 1.0 {
            status = e.update(&ctx(1.0 / 60.0));
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
