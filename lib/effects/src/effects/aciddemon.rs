//! `EF_ACIDDEMON` — Acid Demonstration cast funnel (enum id 537).
//!
//! Renders a `ring_black.tga` swirling-cone cast funnel via the shared
//! casting path — the same machinery as Portal / FlowerCast. Four rings
//! spin around
//! a vertical axis, each at its own radius and rise angle, growing slowly and
//! fading after the early hold. The reference reads as a tall magenta/purple
//! funnel of flames rising from the caster.
//!
//! The four rings are seeded once:
//! * ec0 `{max_height 25, distance 2.4, rise 70, arc 315, rot 0}`
//! * ec1 `{22, 2.7, 57, 315, 90}`
//! * ec2 `{19, 3.0, 45, 315, 180}`
//! * ec3 `{30, 2.2, 90, 360, 0}` — the full upright ring.
//! Per frame each ring spins (~4°/frame, ec3 slower), its radius
//! creeps outward (×~1.005/frame), alpha ramps to a cap then fades.
//!
//! Reproduced with the `Frustum` primitive (one flared cone per ring), the
//! same `(distance, rise, max_height) → (bottom, top, height)` decomposition
//! `flowercast` uses, plus per-frame rotation.

use crate::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode,
};
use crate::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};

/// Stepped per-frame jitter → `[0, 1)`, varied by `salt`. Stepping on the
/// integer frame (not continuous) is what makes the flames *shake* frame to
/// frame instead of drifting smoothly.
fn frame_jitter(frame: u32, salt: u32) -> f32 {
    let x = frame
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

pub const RING_BLACK_TEXTURE: &str = "ring_black.tga";
pub const TEXTURES: &[&str] = &[RING_BLACK_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

/// Magenta/purple flame tint (the reference funnel is distinctly purple);
/// `ring_black.tga` is a near-white bar so the tint carries the colour.
const TINT: [f32; 3] = [200.0 / 255.0, 80.0 / 255.0, 1.0];

/// Source heights 19..30 vs a gif funnel ~2-3 characters tall — scale
/// like `flowercast` / `saint_casting`.
const HEIGHT_SCALE: f32 = 1.0;
const CONE_SIDES: u32 = 28;
/// Cones reach full height over the first ~40 frames (`sin` ramp).
const GROW_FRAMES: f32 = 40.0;
/// Per-frame outward radius creep (×1.005 each frame).
const RADIUS_GROWTH_PER_FRAME: f32 = 1.005;
/// Flame crown: many tongues around the ring (`Sine` wave frequency), at
/// roughly half the cone height, **re-randomised every frame** so the
/// funnel flickers/shakes rather than expanding smoothly — matching the
/// original's jittery ~4-7°/frame spin churn.
const WAVE_FREQUENCY: f32 = 12.0;
const WAVE_REL_AMPLITUDE: f32 = 0.55;
/// Continuous phase drift per frame (on top of the per-frame jitter).
const PHASE_DRIFT_PER_FRAME: f32 = 0.6;

/// The original fires a screen quake at frame 5. We emit a
/// one-shot [`CameraShake`] request at that frame; the holder's shake
/// controller trembles the whole view (see `Effect::take_camera_shake`).
const QUAKE_AT_FRAME: f32 = 5.0;
const QUAKE_AMPLITUDE: f32 = 1.6;
const QUAKE_DURATION_MS: u32 = 600;

const FADE_IN_FRAMES: f32 = 20.0;
const FADE_OUT_FRAMES: f32 = 40.0;
const PEAK_ALPHA: f32 = 110.0 / 255.0;
/// Four cones overlap additively at the centre; pre-attenuate so the
/// `ring_black` striping survives the sum (cf. `flowercast`).
const OVERDRAW_DIVISOR: f32 = 3.0;

const TOTAL_FRAMES: f32 = 120.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;
const FADE_OUT_START_FRAME: f32 = TOTAL_FRAMES - FADE_OUT_FRAMES;

const NUM_RINGS: usize = 4;
/// Per-ring seed: `(distance, rise_deg, max_height, arc_deg, rot_start_deg, spin_per_frame)`.
const RINGS: [(f32, f32, f32, f32, f32, f32); NUM_RINGS] = [
    (2.4, 70.0, 25.0, 315.0, 0.0, 4.0),
    (2.7, 57.0, 22.0, 315.0, 90.0, 4.0),
    (3.0, 45.0, 19.0, 315.0, 180.0, 4.0),
    (2.2, 90.0, 30.0, 360.0, 0.0, 2.0),
];

#[derive(Clone, Copy)]
struct Ring {
    distance: f32,
    rise_deg: f32,
    base_max_height: f32,
    arc_deg: f32,
    rot_start_deg: f32,
    spin_per_frame: f32,
}

pub struct AcidDemonEffect {
    world_pos: [f32; 3],
    rings: [Ring; NUM_RINGS],
    process: f32,
    shake_fired: bool,
}

impl AcidDemonEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rings = RINGS.map(
            |(distance, rise_deg, base_max_height, arc_deg, rot_start_deg, spin_per_frame)| Ring {
                distance,
                rise_deg,
                base_max_height,
                arc_deg,
                rot_start_deg,
                spin_per_frame,
            },
        );
        Self {
            world_pos,
            rings,
            process: 0.0,
            shake_fired: false,
        }
    }

    /// Height ramp (`sin` to the 40-frame peak) times the slow radius creep.
    fn grow(&self) -> f32 {
        let ramp = self.process.min(GROW_FRAMES).to_radians().sin();
        ramp * RADIUS_GROWTH_PER_FRAME.powf(self.process)
    }

    fn alpha(&self) -> f32 {
        let a = if self.process < FADE_IN_FRAMES {
            PEAK_ALPHA * (self.process / FADE_IN_FRAMES)
        } else if self.process < FADE_OUT_START_FRAME {
            PEAK_ALPHA
        } else {
            PEAK_ALPHA
                * (1.0 - (self.process - FADE_OUT_START_FRAME) / FADE_OUT_FRAMES).clamp(0.0, 1.0)
        };
        a / OVERDRAW_DIVISOR
    }
}

impl Effect for AcidDemonEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.process += ctx.delta * FRAMES_PER_SECOND;
        if self.process >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let grow = self.grow();
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        // Stepped jitter on the integer frame makes the flame crown flicker.
        let frame = self.process.floor() as u32;
        let base = self.world_pos;
        for (idx, r) in self.rings.iter().enumerate() {
            let max_h = r.base_max_height * HEIGHT_SCALE * grow;
            let (sin_rise, cos_rise) = r.rise_deg.to_radians().sin_cos();
            let height = sin_rise * max_h;
            let bottom = r.distance;
            let top = r.distance + cos_rise * max_h;
            let rotation = (r.rot_start_deg + r.spin_per_frame * self.process).to_radians();
            let wave_phase = self.process * PHASE_DRIFT_PER_FRAME
                + frame_jitter(frame, idx as u32) * std::f32::consts::TAU;
            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
                base,
                bottom_size: bottom,
                top_size: top,
                height,
                sides: CONE_SIDES,
                arc_angle_deg: r.arc_deg,
                rotation,
                uv_repeat: 1.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: WAVE_REL_AMPLITUDE * max_h,
                wave_frequency: WAVE_FREQUENCY,
                wave_phase,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: RING_BLACK_TEXTURE,
                color: [TINT[0], TINT[1], TINT[2], alpha],
                blend: BlendKind::Additive,
            });
        }
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        if !self.shake_fired && self.process >= QUAKE_AT_FRAME {
            self.shake_fired = true;
            Some(CameraShake {
                amplitude: QUAKE_AMPLITUDE,
                duration_ms: QUAKE_DURATION_MS,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut AcidDemonEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None, caster_yaw: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn frustums(e: &AcidDemonEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_four_rotating_purple_cones() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        step(&mut e, 25.0); // past fade-in so alpha is up
        let prims = frustums(&e);
        assert_eq!(prims.len(), NUM_RINGS);
        for p in &prims {
            match p {
                EffectPrimitiveDraw::Frustum { color, blend, .. } => {
                    assert_eq!(*blend, BlendKind::Additive);
                    // Purple: blue dominates, green is the smallest channel.
                    assert!(color[2] > color[0] && color[0] > color[1]);
                }
                other => panic!("expected Frustum, got {other:?}"),
            }
        }
    }

    #[test]
    fn cones_grow_then_alpha_fades_out() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        step(&mut e, 5.0);
        let early_h = match &frustums(&e)[0] {
            EffectPrimitiveDraw::Frustum { height, .. } => *height,
            _ => unreachable!(),
        };
        step(&mut e, 30.0); // ~frame 35, near full height
        let grown_h = match &frustums(&e)[0] {
            EffectPrimitiveDraw::Frustum { height, .. } => *height,
            _ => unreachable!(),
        };
        assert!(grown_h > early_h, "cone grows: {early_h} -> {grown_h}");

        let a_mid = e.alpha();
        step(&mut e, TOTAL_FRAMES - 35.0 - 1.0); // near the end
        assert!(e.alpha() < a_mid, "alpha fades out by the end");
    }

    #[test]
    fn first_ring_rotates_over_time() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        step(&mut e, 20.0);
        let rot_a = match &frustums(&e)[0] {
            EffectPrimitiveDraw::Frustum { rotation, .. } => *rotation,
            _ => unreachable!(),
        };
        step(&mut e, 10.0);
        let rot_b = match &frustums(&e)[0] {
            EffectPrimitiveDraw::Frustum { rotation, .. } => *rotation,
            _ => unreachable!(),
        };
        assert!(rot_b > rot_a, "ring spins: {rot_a} -> {rot_b}");
    }

    #[test]
    fn flame_crown_flickers_frame_to_frame() {
        // The high-frequency Sine wave plus stepped per-frame jitter means the
        // first ring's wave_phase jumps between consecutive integer frames
        // (the "shake"), rather than drifting smoothly.
        let wave_phase = |e: &AcidDemonEffect| match &frustums(e)[0] {
            EffectPrimitiveDraw::Frustum {
                wave_phase,
                wave_frequency,
                wave_mode,
                ..
            } => {
                assert!(*wave_frequency > 1.0, "flame crown has many tongues");
                assert!(matches!(wave_mode, FrustumWaveMode::Sine));
                *wave_phase
            }
            _ => unreachable!(),
        };
        let mut e = AcidDemonEffect::new([0.0; 3]);
        step(&mut e, 10.0);
        let p0 = wave_phase(&e);
        step(&mut e, 1.0);
        let p1 = wave_phase(&e);
        step(&mut e, 1.0);
        let p2 = wave_phase(&e);
        // Frame-to-frame deltas differ (jitter), not a constant smooth step.
        assert!(((p1 - p0) - (p2 - p1)).abs() > 1e-3, "phase jitters, not smooth");
    }

    #[test]
    fn fires_one_shot_camera_shake_after_frame_5() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        // Before frame 5 there's no shake yet.
        step(&mut e, 2.0);
        assert!(e.take_camera_shake().is_none(), "no shake before frame 5");
        // Past frame 5 it fires exactly once.
        step(&mut e, 5.0);
        let shake = e.take_camera_shake().expect("shake fires after frame 5");
        assert!(shake.amplitude > 0.0 && shake.duration_ms > 0);
        assert!(e.take_camera_shake().is_none(), "shake is one-shot");
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
