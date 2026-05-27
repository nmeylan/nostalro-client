//! `EF_EARTHSPIKE` — Wizard Earth Spike (id 79).
//!
//! Original game's EarthSpike emits two groups of
//! 3D-quad-horn `stone.bmp` blades at frame 0, one-shot:
//! * one tall central spike (second position offset = (0,10,0), height = 18,
//!   size ≈ 3.0..3.5, latitude ∈ [80,100]),
//! * a surrounding ring of six shorter-but-wider blades (`radius = 3.5`,
//!   height ≈ 20, half-size ≈ 4..5, tilt = 100°).
//!
//! Sizes/heights are scaled to our world units (~⅓ of orig height, cf.
//! `frost_diver`) and tuned against the gif. The original game's central
//! blade retracts (the return-down phase); we approximate the silhouette with a
//! rise-then-hold-then-fade like the other QuadHorn effects, and skip the
//! screen shake (no effect-layer hook).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::effects::frost_diver::STONE_TEXTURE;
use crate::effect::effects::spike_util::{
    FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, rise_step,
};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE];

const RING_COUNT: usize = 6;
const RING_RADIUS: f32 = 3.5;

const CENTER_TILT_DEG: f32 = 90.0;
const CENTER_SIZE: f32 = 1.2;
const CENTER_HEIGHT: f32 = 9.0;
const RING_TILT_DEG: f32 = 100.0;
const RING_SIZE: f32 = 1.8;
const RING_HEIGHT: f32 = 6.7;

const SPIKE_SPEED_PER_S: f32 = 0.12 * FRAMES_PER_SECOND;
const SPEED_LIMIT_S: f32 = 12.0 / FRAMES_PER_SECOND;
/// Visible window; the original runs 240 frames but the gif's silhouette
/// settles well before that, so we hold then fade earlier.
const DURATION_FRAMES: f32 = 120.0;
const FADE_OUT_FRAMES: f32 = 20.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

struct Spike {
    base: [f32; 3],
    velocity: [f32; 3],
    tilt_deg: f32,
    heading_deg: f32,
    size: f32,
    height: f32,
}

pub struct EarthSpikeEffect {
    spikes: Vec<Spike>,
    age: f32,
}

impl EarthSpikeEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut spikes = Vec::with_capacity(RING_COUNT + 1);
        // Central blade — vertical, narrow, tall.
        spikes.push(Spike {
            base: world_pos,
            velocity: apex_velocity(CENTER_TILT_DEG, 0.0, SPIKE_SPEED_PER_S),
            tilt_deg: CENTER_TILT_DEG,
            heading_deg: 0.0,
            size: CENTER_SIZE,
            height: CENTER_HEIGHT,
        });
        // Ring — six blades evenly spaced, each leaning outward.
        for i in 0..RING_COUNT {
            let heading = i as f32 * (360.0 / RING_COUNT as f32);
            let rad = heading.to_radians();
            let base = [
                world_pos[0] + RING_RADIUS * rad.cos(),
                world_pos[1],
                world_pos[2] + RING_RADIUS * rad.sin(),
            ];
            spikes.push(Spike {
                base,
                velocity: apex_velocity(RING_TILT_DEG, heading, SPIKE_SPEED_PER_S),
                tilt_deg: RING_TILT_DEG,
                heading_deg: heading,
                size: RING_SIZE,
                height: RING_HEIGHT,
            });
        }
        Self { spikes, age: 0.0 }
    }

    fn duration_s(&self) -> f32 {
        DURATION_FRAMES / FRAMES_PER_SECOND
    }
}

impl Effect for EarthSpikeEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        for s in &mut self.spikes {
            rise_step(&mut s.base, s.velocity, self.age, ctx.delta, SPEED_LIMIT_S);
        }
        self.age += ctx.delta;
        if self.age >= self.duration_s() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = fade_tail_alpha(self.age, self.duration_s(), 1.0, FADE_OUT_FRAMES);
        for s in &self.spikes {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: s.base,
                size: s.size,
                height: s.height,
                tilt_x_deg: s.tilt_deg,
                rotation_y_deg: s.heading_deg,
                texture: STONE_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                // Opaque brown stone — alpha keeps the colour (cf. grimtooth).
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &EarthSpikeEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_central_plus_ring_then_dies() {
        // Sociable test: 1 central + 6 ring = 7 QuadHorns at frame 0; the
        // central blade is narrower and the ring blades sit out at radius;
        // the effect ends after its window.
        let mut e = EarthSpikeEffect::new([0.0, 0.0, 0.0]);
        e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None });
        let prims = draws(&e);
        assert_eq!(prims.len(), 7);

        let central = match &prims[0] {
            EffectPrimitiveDraw::QuadHorn { base, size, texture, .. } => {
                assert_eq!(*texture, STONE_TEXTURE);
                (*base, *size)
            }
            _ => panic!("expected QuadHorn"),
        };
        // Central blade sits on the anchor; ring blades sit out at radius.
        assert!(central.0[0].abs() < 1e-3 && central.0[2].abs() < 1e-3);
        for p in &prims[1..] {
            let EffectPrimitiveDraw::QuadHorn { base, size, .. } = p else {
                panic!("expected QuadHorn");
            };
            let r = (base[0] * base[0] + base[2] * base[2]).sqrt();
            assert!((r - RING_RADIUS).abs() < 1e-3, "ring blade at radius");
            assert!(*size > central.1, "ring blades wider than central");
        }

        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_MS as f32 / 1000.0 + 0.1 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None });
            t += 1.0 / 60.0;
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn central_blade_rises_during_speed_window() {
        // Sociable test: the rising base translates up (native RO -Y) during
        // the speed-limit window, then freezes.
        let mut e = EarthSpikeEffect::new([0.0, 0.0, 0.0]);
        e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None });
        let y0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };
        let mut t = 0.0;
        while t < SPEED_LIMIT_S {
            e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None });
            t += 1.0 / 60.0;
        }
        let y1 = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };
        assert!(y1 < y0, "central base rose (Y more negative): {y0} -> {y1}");
    }
}
