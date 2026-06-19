//! `EF_GRAVITATION` (id 522) — Gravity Field: a dense field of stone and ice
//! shards erupting from the ground while the camera trembles.
//!
//! The cast triggers a continuous camera shake (amplitude 1.0)
//! then launches 4 shard families (`stone.bmp` ×2 + `ice.tga`
//! ×2): spikes width 3.0–3.5, height 18, tilt 60–100°,
//! random heading, with an in/out speed oscillation (every 6 frames flips
//! between `+1.18` push and `−1.2` pull — the "gravitation" writhe).
//!
//! Each spike is one 4-triangle horn. The original's 4
//! spikes do not match the gif's dense shard field, so — gif outranking the
//! original — this spreads many spikes across a ground disc, each a [`QuadHorn`]
//! pointing up with a random tilt, alpha-fading in then out, the whole field
//! pulsing toward centre. The continuous tremor is the dominant "gravity" cue.
//!
//! [`QuadHorn`]: crate::draw::EffectPrimitiveDraw::QuadHorn

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const STONE: &str = "stone.bmp";
const ICE: &str = "ice.tga";
pub const TEXTURES: &[&str] = &[STONE, ICE];

/// Height 18 / width 3–3.5 — shards a touch taller than the caster,
/// the field a bit wider.
const WORLD_SCALE: f32 = 0.8;

const SPIKE_COUNT: usize = 60;
/// Ground-disc radius the shard bases spread across.
const FIELD_RADIUS: f32 = 13.0;

/// The field erupts, trembles, then fades — pinned shorter than the skill's
/// nominal field life so the visible burst matches the reference loop.
const DURATION_FRAMES: f32 = 240.0;
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_OUT_FRAMES: f32 = 30.0;
/// Alpha 20/255 (≈0.08) in the original — the shards are very translucent;
/// overlapping ones build up but never go solid.
const MAX_ALPHA: f32 = 0.22;

/// Continuous tremor for the field's life (amplitude 1.0).
const QUAKE_AMPLITUDE: f32 = 1.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

struct Spike {
    base: [f32; 3],
    /// Direction from the field centre (for the inward pulse), XZ only.
    inward: [f32; 2],
    size: f32,
    height: f32,
    latitude_deg: f32,
    longitude_deg: f32,
    /// Per-spike phase so the field writhes rather than pulsing in lockstep.
    pulse_phase: f32,
    texture: &'static str,
}

pub struct GravitationEffect {
    spikes: Vec<Spike>,
    age_frames: f32,
    shake_fired: bool,
}

impl GravitationEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        let [ax, ay, az] = anchor;
        let seed = ax.to_bits() ^ az.to_bits() ^ 0x6BA1_7A70;
        let mut rng = Rng(seed | 1);
        let spikes = (0..SPIKE_COUNT)
            .map(|i| {
                let angle = rng.range(0.0, std::f32::consts::TAU);
                let radius = FIELD_RADIUS * rng.range(0.0, 1.0).sqrt();
                let (dx, dz) = (angle.cos(), angle.sin());
                Spike {
                    base: [ax + dx * radius, ay, az + dz * radius],
                    inward: [-dx, -dz],
                    size: rng.range(3.0, 3.5) * WORLD_SCALE,
                    height: 18.0 * WORLD_SCALE * rng.range(0.8, 1.1),
                    latitude_deg: rng.range(60.0, 100.0),
                    longitude_deg: rng.range(0.0, 360.0),
                    pulse_phase: rng.range(0.0, std::f32::consts::TAU),
                    texture: if i < SPIKE_COUNT / 2 { STONE } else { ICE },
                }
            })
            .collect();
        Self { spikes, age_frames: 0.0, shake_fired: false }
    }

    fn alpha(&self) -> f32 {
        let a = self.age_frames;
        if a < FADE_IN_FRAMES {
            MAX_ALPHA * (a / FADE_IN_FRAMES)
        } else if a > DURATION_FRAMES - FADE_OUT_FRAMES {
            MAX_ALPHA * ((DURATION_FRAMES - a) / FADE_OUT_FRAMES).max(0.0)
        } else {
            MAX_ALPHA
        }
    }
}

impl Effect for GravitationEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
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
        for s in &self.spikes {
            // In/out writhe along the inward direction (the gravitation pull).
            let pulse = (self.age_frames * 0.25 + s.pulse_phase).sin();
            let reach = pulse * 1.2 * WORLD_SCALE;
            let base = [s.base[0] + s.inward[0] * reach, s.base[1], s.base[2] + s.inward[1] * reach];
            out.push(EffectPrimitiveDraw::QuadHorn {
                base,
                size: s.size,
                height: s.height,
                tilt_x_deg: s.latitude_deg,
                rotation_y_deg: s.longitude_deg,
                texture: s.texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
            });
        }
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        if !self.shake_fired {
            self.shake_fired = true;
            Some(CameraShake { amplitude: QUAKE_AMPLITUDE, duration_ms: TOTAL_DURATION_MS })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 256.0, screen_h: 256.0, elapsed: 0.0 }
    }

    fn step(e: &mut GravitationEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None })
    }

    fn horns(e: &GravitationEffect) -> Vec<([f32; 3], f32, &'static str)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::QuadHorn { base, color, texture, blend: BlendKind::Alpha, .. } => {
                    (*base, color[3], *texture)
                }
                other => panic!("expected alpha QuadHorn, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn dense_field_of_stone_and_ice_shards() {
        let mut e = GravitationEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 8.0);
        let h = horns(&e);
        assert_eq!(h.len(), SPIKE_COUNT, "full shard field");
        assert!(h.iter().any(|(_, _, t)| *t == STONE), "stone shards present");
        assert!(h.iter().any(|(_, _, t)| *t == ICE), "ice shards present");
        // Spread across a disc, not a single point.
        let xs: Vec<f32> = h.iter().map(|(b, _, _)| b[0]).collect();
        let spread = xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min);
        assert!(spread > 4.0, "shards spread across the field: {spread}");
    }

    #[test]
    fn fades_in_holds_then_dies() {
        let mut e = GravitationEffect::new([0.0; 3]);
        step(&mut e, 2.0);
        let a_early = horns(&e)[0].1;
        step(&mut e, 20.0);
        let a_mid = horns(&e)[0].1;
        assert!(a_mid > a_early, "fades in: {a_early} -> {a_mid}");
        assert_eq!(step(&mut e, DURATION_FRAMES), EffectStatus::Dead);
    }

    #[test]
    fn fires_continuous_camera_shake_once() {
        let mut e = GravitationEffect::new([0.0; 3]);
        let shake = e.take_camera_shake().expect("shake fires at spawn");
        assert!(shake.amplitude > 0.0 && shake.duration_ms > 1000);
        assert!(e.take_camera_shake().is_none(), "shake fires only once");
    }
}
