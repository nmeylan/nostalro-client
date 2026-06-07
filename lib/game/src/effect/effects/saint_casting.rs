//! Shared casting cone aura — backs `BeginSpell` and `BeginSpell6` (and any
//! future `BeginSpell*` colour variant) with a single integrator that seeds the
//! emitters and steps them once per frame. One code path serves every colour
//! variant in the family.
//!
//! Geometry: per pass (`time = 45` or `25`), 4 closed truncated cones are
//! seeded at `distance = 4.1`, `rise_angle = 80°`, `full_display_angle_deg = 360°`,
//! sharing a common base point. `rise_angle` collapses from 80° to a floor
//! of 10° at 1°/frame while `distance` slides outward by 0.07/frame — angle
//! expands, height shrinks. A fixed-azimuth bell-shaped flame-tip envelope
//! (handled by `FrustumWaveMode::SaintBell` in the renderer) pulses in
//! amplitude over time; the bell does **not** rotate around the cone. Two
//! passes overlay 8 emitters total with staggered initial alphas.

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
/// The parent duration is clamped to 56 if smaller; the parent then runs for
/// that many ticks.
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const INIT_DISTANCE: f32 = 4.1;
const INIT_RISE_DEG: f32 = 80.0;
const DISTANCE_GROW_PER_FRAME: f32 = 0.07;
const RISE_SHRINK_PER_FRAME: f32 = 1.0;
const RISE_FLOOR_DEG: f32 = 10.0;
/// Original game's reset point when an emitter's alpha drops to 0 before the
/// effect ends — the cone collapses back to a closer/steeper position and
/// the alpha pulse restarts.
const RESET_DISTANCE: f32 = 3.37;
const RESET_RISE_DEG: f32 = 74.0;
const ALPHA_DRAIN_PER_FRAME: f32 = 3.0;
const ALPHA_REFILL_PER_FRAME: f32 = 10.0;
const ALPHA_REFILL_DISTANCE_GATE: f32 = 4.0;
/// Block the reset branch in the last `duration - 30` frames so the effect
/// doesn't spawn another pulse it can't fade out (original GI1 pulse logic).
const RESET_BLOCK_FRAMES_FROM_END: f32 = 30.0;

pub const NUM_EMITTERS: usize = 4;
/// Per-emitter starting azimuths (degrees).
const ROT_START_DEG: [f32; NUM_EMITTERS] = [180.0, 270.0, 0.0, 90.0];
/// start alpha = time + offset — descending brightness staircase.
const ALPHA_OFFSET: [f32; NUM_EMITTERS] = [135.0, 90.0, 45.0, 0.0];
/// Two saint-casting calls fire at `state_cnt==0` with these `time`s.
pub const PASS_TIMES: [f32; 2] = [45.0, 25.0];

/// Closed-cone segment count (20) so the per-segment flicker bell has a clean
/// angular resolution.
const CONE_SIDES: u32 = 20;
const CONE_UV_REPEAT: f32 = 1.0;
/// Cone height swings ±30% around its max each pulse.
const WAVE_REL_AMPLITUDE: f32 = 0.3;
/// Per-emitter bell phase advance (degrees/frame): emitters 0,1 advance at
/// `process + ec*90` (= 1°/frame), emitters 2,3 advance at `process*2 + ec*90`
/// (= 2°/frame).
const WAVE_PHASE_PER_FRAME_DEG: [f32; NUM_EMITTERS] = [1.0, 1.0, 2.0, 2.0];

/// Per-effect colour/size parameters. Everything else is shared across the
/// `BeginSpell*` family.
#[derive(Clone, Copy)]
pub struct SaintCastingConfig {
    pub texture: &'static str,
    /// Max height for each of the 4 emitters, driving the cone's initial
    /// height. F1=1 → `[20,19,18,17]`, default-F1 → `[15,14,13,12]`. The
    /// descending order matches the alpha staircase — brightest emitter is
    /// also tallest.
    pub max_heights: [f32; NUM_EMITTERS],
}

#[derive(Clone, Copy)]
struct Emitter {
    distance: f32,
    rise_deg: f32,
    alpha: f32,
    rot_start_deg: f32,
    max_height: f32,
    wave_phase_rad: f32,
    wave_phase_rate_rad: f32,
}

impl Emitter {
    fn step(&mut self) {
        self.distance += DISTANCE_GROW_PER_FRAME;
        let next_rise = self.rise_deg - RISE_SHRINK_PER_FRAME;
        if next_rise < RISE_FLOOR_DEG {
            self.rise_deg = RISE_FLOOR_DEG;
            // Also drop alpha to 0 the frame the rise angle floors — the
            // cone has finished its expansion arc, no more visible
            // animation. Without this the cone parks at the floor showing
            // a stale flat fan until the parent expires.
            self.alpha = 0.0;
        } else {
            self.rise_deg = next_rise;
        }
        self.wave_phase_rad += self.wave_phase_rate_rad;

        if self.distance >= ALPHA_REFILL_DISTANCE_GATE {
            self.alpha -= ALPHA_DRAIN_PER_FRAME;
        } else {
            self.alpha += ALPHA_REFILL_PER_FRAME;
        }
    }

    fn try_reset(&mut self, frames_remaining: f32) {
        if self.alpha > 0.0 || frames_remaining <= RESET_BLOCK_FRAMES_FROM_END {
            return;
        }
        self.distance = RESET_DISTANCE;
        self.rise_deg = RESET_RISE_DEG;
        self.alpha = 0.0;
    }

    fn alpha_unit(&self) -> f32 {
        // 8 emitters all rendering at the same world position with additive
        // blending; the original game's accumulator absorbs the ~3× overdraw
        // but our framebuffer saturates to white at the centre, erasing the
        // ring texture's striped flame-tongue pattern. Pre-attenuate so the
        // additive sum at peak stays below 1.0 and the texture detail
        // survives. Ratios between emitters (the per-emitter alpha staircase
        // {180,135,90,45,...}) are preserved.
        const OVERDRAW_DIVISOR: f32 = 4.0;
        (self.alpha / (255.0 * OVERDRAW_DIVISOR)).clamp(0.0, 1.0)
    }
}

pub struct SaintCastingEffect {
    world_pos: [f32; 3],
    age: f32,
    emitters: Vec<Emitter>,
    cfg: SaintCastingConfig,
}

impl SaintCastingEffect {
    pub fn new(world_pos: [f32; 3], cfg: SaintCastingConfig) -> Self {
        let mut emitters = Vec::with_capacity(PASS_TIMES.len() * NUM_EMITTERS);
        for (pass_idx, pass_time) in PASS_TIMES.iter().enumerate() {
            for ec in 0..NUM_EMITTERS {
                let alpha = pass_time + ALPHA_OFFSET[ec];
                // Spread initial phases across emitters so the
                // amplitude envelope is alive on day 1.
                let pass_offset_deg = if pass_idx == 0 { 0.0 } else { 90.0 };
                let initial_phase_deg: f32 = pass_offset_deg + ec as f32 * 45.0;
                emitters.push(Emitter {
                    distance: INIT_DISTANCE,
                    rise_deg: INIT_RISE_DEG,
                    alpha,
                    rot_start_deg: ROT_START_DEG[ec],
                    max_height: cfg.max_heights[ec],
                    wave_phase_rad: initial_phase_deg.to_radians(),
                    wave_phase_rate_rad: WAVE_PHASE_PER_FRAME_DEG[ec].to_radians(),
                });
            }
        }
        Self {
            world_pos,
            age: 0.0,
            emitters,
            cfg,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for SaintCastingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frame_before = self.frame();
        self.age += ctx.delta;
        let frame_after = self.frame();
        let steps = (frame_after.floor() - frame_before.floor()).max(0.0) as i32;
        for _ in 0..steps {
            let frames_remaining = TOTAL_FRAMES - self.frame();
            for em in &mut self.emitters {
                em.step();
                em.try_reset(frames_remaining);
            }
        }
        if frame_after >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        if frame > TOTAL_FRAMES {
            return;
        }
        for em in &self.emitters {
            let alpha = em.alpha_unit();
            if alpha <= 0.0 {
                continue;
            }
            let (sin_rise, cos_rise) = em.rise_deg.to_radians().sin_cos();
            let height = sin_rise * em.max_height;
            let bottom = em.distance;
            let top = em.distance + cos_rise * em.max_height;
            // Height delta `max_h * 0.3 * sin(phase)` scales with `max_h`,
            // not with the current `height = sin(rise) * max_h`. Scaling by
            // `height` instead would shrink the flame-tip pulse by ~6× as
            // the cone flattens (sin80° → sin10°), making the wave invisible
            // late in the effect. The renderer projects this onto
            // (cos rise, sin rise), so the cone-flatness factor is applied
            // exactly once and the pulse stays visible the whole time the
            // cone is alive.
            let wave_amplitude = WAVE_REL_AMPLITUDE * em.max_height * em.wave_phase_rad.sin();
            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: bottom,
                top_size: top,
                height,
                sides: CONE_SIDES,
                arc_angle_deg: 360.0,
                rotation: em.rot_start_deg.to_radians(),
                uv_repeat: CONE_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::SaintBell,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                // A hard back-face discard would be ideal, but our
                // `cull_back: true` is a soft per-segment fade; with 4 cones
                // at 90° starts the fades overlap inconsistently (each
                // cone's "back" sits where another cone's "front" is, so the
                // visible silhouette gains noisy half-faded segments instead
                // of one clean front face). The texture's transparency does
                // the real shaping work, so keep both faces drawn rather
                // than fading the back ones.
                cull_back: false,
                texture: self.cfg.texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG: SaintCastingConfig = SaintCastingConfig {
        texture: "ring_test.tga",
        max_heights: [17.0, 18.0, 19.0, 20.0],
    };

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &SaintCastingEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step_frames(e: &mut SaintCastingEffect, n: u32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        for _ in 0..n {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None, caster_yaw: None,
            });
        }
        status
    }

    fn widest_top(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { top_size, .. } => Some(*top_size),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    }

    fn tallest(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { height, .. } => Some(*height),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn cone_expands_in_breadth_and_collapses_vertically() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        step_frames(&mut e, 4);
        let early_top = widest_top(&draws(&e));
        let early_h = tallest(&draws(&e));
        step_frames(&mut e, 40);
        let late_top = widest_top(&draws(&e));
        let late_h = tallest(&draws(&e));
        assert!(
            late_top > early_top * 2.0,
            "top width must more than double over the effect ({early_top} → {late_top})"
        );
        assert!(
            late_h < early_h,
            "vertical height must collapse as rise angle drops ({early_h} → {late_h})"
        );
    }

    #[test]
    fn no_vertical_center_pillar() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        for _ in 0..(TOTAL_FRAMES as u32) {
            assert!(
                tallest(&draws(&e)) < 25.0,
                "no emitter is tall enough to read as a center pillar"
            );
            step_frames(&mut e, 1);
        }
    }

    #[test]
    fn cone_does_not_rotate_around_its_axis() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        let allowed: Vec<f32> = ROT_START_DEG.iter().map(|d| d.to_radians()).collect();
        for _ in 0..(TOTAL_FRAMES as u32) {
            for p in draws(&e) {
                if let EffectPrimitiveDraw::Frustum { rotation, .. } = p {
                    assert!(
                        allowed.iter().any(|a| (a - rotation).abs() < 1e-5),
                        "rotation {rotation} must match an initial RotStart, never advance"
                    );
                }
            }
            step_frames(&mut e, 1);
        }
    }

    #[test]
    fn emits_saint_bell_wave_mode() {
        let e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        for p in draws(&e) {
            if let EffectPrimitiveDraw::Frustum { wave_mode, .. } = p {
                assert_eq!(wave_mode, FrustumWaveMode::SaintBell);
            }
        }
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        for f in 0..(TOTAL_FRAMES as u32) {
            assert_eq!(
                step_frames(&mut e, 1),
                EffectStatus::Running,
                "still alive at frame {f}"
            );
        }
        assert_eq!(step_frames(&mut e, 1), EffectStatus::Dead);
    }

    #[test]
    fn spawns_eight_emitters() {
        let e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        let n = draws(&e)
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(n, 8);
    }
}
