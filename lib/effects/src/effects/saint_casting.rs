//! Shared casting cone aura — backs `BeginSpell` and `BeginSpell6` (and any
//! future `BeginSpell*` colour variant) with a single integrator that seeds the
//! emitters and steps them once per frame. One code path serves every colour
//! variant in the family.
//!
//! Geometry: per pass (`time = 45` or `25`), 4 closed truncated cones are
//! seeded at `distance = 4.1`, rise angle 80°, full 360° spread, sharing a
//! common base point. The rise angle collapses from 80° to a floor of 10° at
//! 1°/frame while `distance` slides outward by 0.07/frame — angle expands,
//! height shrinks. A fixed-azimuth bell-shaped flame-tip envelope (handled by
//! `FrustumWaveMode::SaintBell` in the renderer) pulses in amplitude over
//! time; the bell does **not** rotate around the cone. Two passes overlay 8
//! emitters total.
//!
//! Because every cast aura here runs the short 56-frame (≤ 60) path, the seed
//! takes the branch that zeroes each emitter's alpha and offsets its `process`
//! counter to `-ec·5`. An emitter only advances once `process > 0`, so the
//! 4 cones of a pass rise in a staggered cascade (frames 1, 6, 11, 16) and
//! fade *in* from zero rather than all popping on at full brightness — this
//! is the "small delay" between the cones, not a hard pop. Aura Blade
//! additionally refills alpha at
//! `+5`/frame instead of `+10` and resets to a `64°` rise instead of `74°`.

use crate::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode,
};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

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
/// Reset point when an emitter's alpha drops to 0 before the effect ends —
/// the cone collapses back to a closer/steeper position (`4.0 - 0.63`) and
/// the alpha pulse refills again.
const RESET_DISTANCE: f32 = 3.37;
const ALPHA_DRAIN_PER_FRAME: f32 = 3.0;
const ALPHA_REFILL_DISTANCE_GATE: f32 = 4.0;
/// A drained emitter only resets while `process < duration - 30` so it doesn't
/// spawn another pulse it can't fade out before the parent expires. With a
/// 56-frame duration that floor is frame 26.
const RESET_PROCESS_LIMIT: i32 = TOTAL_FRAMES as i32 - 30;
/// Short-duration stagger: `process[ec] = -ec·5`. Each emitter idles until its
/// `process` counter climbs above 0.
const PROCESS_STAGGER: i32 = 5;

pub const NUM_EMITTERS: usize = 4;
/// Per-emitter starting azimuths (degrees).
const ROT_START_DEG: [f32; NUM_EMITTERS] = [180.0, 270.0, 0.0, 90.0];
/// Two passes fire at spawn. The `time` argument only seeds alpha on the
/// long-duration path; on our short path it's overwritten to 0, so these
/// values just count the passes and pick per-pass textures.
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
    /// Optional per-pass texture override. The two passes fire at spawn with
    /// times `[45, 25]`; when `Some([t0, t1])` the `time=45` pass uses `t0`
    /// and the `time=25` pass uses `t1`. Aura Blade stacks a white ring
    /// (pass 0) over a yellow ring (pass 1). `None` uses [`Self::texture`]
    /// for both passes (the `BeginSpell*` family).
    pub pass_textures: Option<[&'static str; 2]>,
    /// Max height for each of the 4 emitters, driving the cone's initial
    /// height. F1=1 → `[20,19,18,17]`, default-F1 → `[15,14,13,12]`. The
    /// descending order matches the alpha staircase — brightest emitter is
    /// also tallest.
    pub max_heights: [f32; NUM_EMITTERS],
    /// Vertex tint chosen by the effect's size/flavour code, which picks both
    /// the rgb and the blend. The flavour code maps a given variant to one of
    /// these (e.g. F1=5 → (100,100,255) additive; F1=6 → (50,50,50) alpha),
    /// so the same cone integrator serves every `BeginSpell*` colour.
    pub color_rgb: [f32; 3],
    pub blend: BlendKind,
    /// Alpha refill rate per frame while the cone is still inside the
    /// `distance < 4.0` window: `+5` for Aura Blade and `+10` for everything
    /// else.
    pub refill_per_frame: f32,
    /// Rise angle an emitter snaps back to when its alpha drains to 0 and it
    /// resets: `55 + 9 = 64°` for Aura Blade, `65 + 9 = 74°` otherwise. Aura
    /// Blade resets to a steeper cone than the other casting variants.
    pub reset_rise_deg: f32,
}

#[derive(Clone, Copy)]
struct Emitter {
    distance: f32,
    rise_deg: f32,
    alpha: f32,
    rot_start_deg: f32,
    max_height: f32,
    /// Process counter. Starts negative (`-ec·5`) so the cone idles before
    /// rising; advances 1/frame and only animates once positive.
    process: i32,
    /// Per-frame phase advance for the flame-tip bell (`1°` for emitters 0,1;
    /// `2°` for 2,3).
    wave_rate_deg: f32,
    /// `ec·90` constant offset in the bell phase term.
    wave_base_deg: f32,
    texture: &'static str,
}

impl Emitter {
    fn step(&mut self, refill_per_frame: f32, reset_rise_deg: f32) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
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

        if self.distance >= ALPHA_REFILL_DISTANCE_GATE {
            self.alpha -= ALPHA_DRAIN_PER_FRAME;
            if self.alpha <= 0.0 {
                self.alpha = 0.0;
                if self.process < RESET_PROCESS_LIMIT {
                    self.distance = RESET_DISTANCE;
                    self.rise_deg = reset_rise_deg;
                }
            }
        } else {
            self.alpha += refill_per_frame;
        }
    }

    /// Phase the flame-tip bell rides on. Frozen while the emitter idles
    /// (`process <= 0`).
    fn wave_phase_rad(&self) -> f32 {
        (self.process.max(0) as f32 * self.wave_rate_deg + self.wave_base_deg).to_radians()
    }

    fn alpha_unit(&self, blend: BlendKind, refill_per_frame: f32) -> f32 {
        // 8 emitters all rendering at the same world position with additive
        // blending saturate our framebuffer to white at the centre, erasing
        // the ring texture's striped flame-tongue pattern. Pre-attenuate so
        // the additive sum at peak stays below 1.0 and the texture detail
        // survives.
        //
        // An emitter refills over ~8 frames before the distance gate, so its
        // peak `alpha ≈ refill_per_frame · 8`. Scaling the divisor by the
        // refill rate keeps every variant's per-emitter peak at the same
        // visible brightness (≈0.16) regardless of whether it refills at +10
        // (begin-spell family) or +5 (Aura Blade) — otherwise Aura Blade, at
        // half the alpha, washes out to nothing. Alpha-blended variants
        // (DarkCasting's dark dome) can't saturate — full strength so the
        // stack genuinely darkens the scene.
        let overdraw_divisor: f32 = match blend {
            BlendKind::Additive => refill_per_frame / 5.0,
            _ => 1.0,
        };
        (self.alpha / (255.0 * overdraw_divisor)).clamp(0.0, 1.0)
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
        for pass_idx in 0..PASS_TIMES.len() {
            for ec in 0..NUM_EMITTERS {
                let texture = cfg
                    .pass_textures
                    .map(|t| t[pass_idx])
                    .unwrap_or(cfg.texture);
                emitters.push(Emitter {
                    distance: INIT_DISTANCE,
                    rise_deg: INIT_RISE_DEG,
                    // Short-duration path: every emitter starts fully
                    // transparent and fades in.
                    alpha: 0.0,
                    rot_start_deg: ROT_START_DEG[ec],
                    max_height: cfg.max_heights[ec],
                    process: -(ec as i32) * PROCESS_STAGGER,
                    wave_rate_deg: WAVE_PHASE_PER_FRAME_DEG[ec],
                    wave_base_deg: ec as f32 * 90.0,
                    texture,
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
            for em in &mut self.emitters {
                em.step(self.cfg.refill_per_frame, self.cfg.reset_rise_deg);
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
            let alpha = em.alpha_unit(self.cfg.blend, self.cfg.refill_per_frame);
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
            let wave_amplitude = WAVE_REL_AMPLITUDE * em.max_height * em.wave_phase_rad().sin();
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
                texture: em.texture,
                color: [
                    self.cfg.color_rgb[0],
                    self.cfg.color_rgb[1],
                    self.cfg.color_rgb[2],
                    alpha,
                ],
                blend: self.cfg.blend,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG: SaintCastingConfig = SaintCastingConfig {
        texture: "ring_test.tga",
        pass_textures: None,
        max_heights: [17.0, 18.0, 19.0, 20.0],
        color_rgb: [1.0, 1.0, 1.0],
        blend: BlendKind::Additive,
        refill_per_frame: 10.0,
        reset_rise_deg: 74.0,
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
    fn spawns_eight_emitters_once_the_cascade_is_up() {
        // Each emitter idles for `ec·5` frames then fades in; the last starts
        // at frame 16, so by frame 18 all 8 are drawing.
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        step_frames(&mut e, 18);
        let n = draws(&e)
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(n, 8);
    }

    #[test]
    fn emitters_start_staggered_and_fade_in_from_zero() {
        // Frame 0: nothing drawn (all emitters transparent). As `process`
        // climbs past 0 for each `ec`, more cones appear — the cascade.
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        let count = |e: &SaintCastingEffect| {
            draws(e)
                .iter()
                .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
                .count()
        };
        assert_eq!(count(&e), 0, "everything fades in — frame 0 is empty");
        step_frames(&mut e, 4);
        let early = count(&e);
        step_frames(&mut e, 14);
        let late = count(&e);
        assert!(early > 0 && early < 8, "only the lead emitters are up: {early}");
        assert_eq!(late, 8, "the whole cascade is up by frame 18");
    }
}
