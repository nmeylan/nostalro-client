//! `EF_ACIDDEMON` — Acid Demonstration cast funnel (enum id 537).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};

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

const TINT: [f32; 3] = [200.0 / 255.0, 80.0 / 255.0, 1.0];

const HEIGHT_SCALE: f32 = 1.0;
const CONE_SIDES: u32 = 28;
const GROW_FRAMES: f32 = 40.0;
const RADIUS_GROWTH_PER_FRAME: f32 = 1.005;
const WAVE_FREQUENCY: f32 = 12.0;
const WAVE_REL_AMPLITUDE: f32 = 0.55;
const PHASE_DRIFT_PER_FRAME: f32 = 0.6;

const QUAKE_AT_FRAME: f32 = 5.0;
const QUAKE_AMPLITUDE: f32 = 1.0;
const QUAKE_DURATION_MS: u32 = 650;

/// The aura and its shake are the bomb's impact, but this effect spawns
/// alongside the thrown molotov (`Throwitem4`), which arcs to the target over
/// 30 frames (`throw_item`: 5-frame launch + 25-frame flight, distance-
/// independent). Hold the aura dormant for that long so it erupts as the bottle
/// lands rather than at cast time.
const SPAWN_DELAY_FRAMES: f32 = 30.0;

const FADE_IN_FRAMES: f32 = 20.0;
const FADE_OUT_FRAMES: f32 = 40.0;
const PEAK_ALPHA: f32 = 110.0 / 255.0;
/// Four overlapping additive cones — pre-attenuate so striping survives the sum.
const OVERDRAW_DIVISOR: f32 = 3.0;

const TOTAL_FRAMES: f32 = 120.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;
const FADE_OUT_START_FRAME: f32 = TOTAL_FRAMES - FADE_OUT_FRAMES;

const NUM_RINGS: usize = 4;
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
    age: f32,
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
            age: 0.0,
            shake_fired: false,
        }
    }

    /// Animation clock, zero until the thrown bottle lands.
    fn process(&self) -> f32 {
        (self.age - SPAWN_DELAY_FRAMES).max(0.0)
    }

    fn grow(&self) -> f32 {
        let process = self.process();
        let ramp = process.min(GROW_FRAMES).to_radians().sin();
        ramp * RADIUS_GROWTH_PER_FRAME.powf(process)
    }

    fn alpha(&self) -> f32 {
        let process = self.process();
        let a = if process < FADE_IN_FRAMES {
            PEAK_ALPHA * (process / FADE_IN_FRAMES)
        } else if process < FADE_OUT_START_FRAME {
            PEAK_ALPHA
        } else {
            PEAK_ALPHA * (1.0 - (process - FADE_OUT_START_FRAME) / FADE_OUT_FRAMES).clamp(0.0, 1.0)
        };
        a / OVERDRAW_DIVISOR
    }
}

impl Effect for AcidDemonEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta * FRAMES_PER_SECOND;
        if self.process() >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let process = self.process();
        let grow = self.grow();
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let frame = process.floor() as u32;
        let base = self.world_pos;
        for (idx, r) in self.rings.iter().enumerate() {
            let max_h = r.base_max_height * HEIGHT_SCALE * grow;
            let (sin_rise, cos_rise) = r.rise_deg.to_radians().sin_cos();
            let height = sin_rise * max_h;
            let bottom = r.distance;
            let top = r.distance + cos_rise * max_h;
            let rotation = (r.rot_start_deg + r.spin_per_frame * process).to_radians();
            let wave_phase = process * PHASE_DRIFT_PER_FRAME
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
        if !self.shake_fired && self.process() >= QUAKE_AT_FRAME {
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
            camera_target: None,
            caster_yaw: None,
        })
    }

    /// Advance to the moment the thrown bottle lands and the aura wakes up.
    fn wake(e: &mut AcidDemonEffect) {
        step(e, SPAWN_DELAY_FRAMES);
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
        wake(&mut e);
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
        wake(&mut e);
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
        wake(&mut e);
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
        wake(&mut e);
        step(&mut e, 10.0);
        let p0 = wave_phase(&e);
        step(&mut e, 1.0);
        let p1 = wave_phase(&e);
        step(&mut e, 1.0);
        let p2 = wave_phase(&e);
        assert!(
            ((p1 - p0) - (p2 - p1)).abs() > 1e-3,
            "phase jitters, not smooth"
        );
    }

    #[test]
    fn stays_dark_and_silent_until_the_bomb_lands() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        step(&mut e, SPAWN_DELAY_FRAMES - 1.0);
        assert!(frustums(&e).is_empty(), "no aura before the bottle lands");
        assert!(
            e.take_camera_shake().is_none(),
            "no shake before the bottle lands"
        );
    }

    #[test]
    fn fires_one_shot_camera_shake_after_frame_5() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        wake(&mut e);
        step(&mut e, 2.0);
        assert!(e.take_camera_shake().is_none(), "no shake before frame 5");
        step(&mut e, 5.0);
        let shake = e.take_camera_shake().expect("shake fires after frame 5");
        assert!(shake.amplitude > 0.0 && shake.duration_ms > 0);
        assert!(e.take_camera_shake().is_none(), "shake is one-shot");
    }

    #[test]
    fn cones_follow_the_entity_while_it_moves() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        wake(&mut e);
        step(&mut e, 25.0);
        e.set_position([10.0, 1.0, -4.0]);
        for p in frustums(&e) {
            match p {
                EffectPrimitiveDraw::Frustum { base, .. } => assert_eq!(base, [10.0, 1.0, -4.0]),
                other => panic!("expected Frustum, got {other:?}"),
            }
        }
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = AcidDemonEffect::new([0.0; 3]);
        wake(&mut e);
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
