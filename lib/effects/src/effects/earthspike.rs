//! `EF_EARTHSPIKE` — Wizard Earth Spike (id 79) and `EF_HYOUSENSOU` (id 617).
//!
//! Emits two groups of blade horns at frame 0,
//! one-shot:
//! * one tall central spike (rises straight up, height ≈ 18,
//!   half-size ≈ 3.0..3.5, tilt ∈ [80,100]°),
//! * a surrounding ring of six shorter-but-wider blades (`radius = 3.5`,
//!   height ≈ 20, half-size ≈ 4..5, tilt = 100°).
//!
//! `F1` only swaps the texture (and sound): `F1 = 0` Earth Spike uses
//! `stone.bmp`, `F1 = 2` Hyousensou uses `ice.tga`. Identical geometry, so
//! both run through the same effect parameterised by [`EarthSpikeParams`].
//!
//! Sizes/heights are scaled to our world units (~⅓ of the original height, cf.
//! `frost_diver`) and tuned against the gif. The central
//! blade shoots up then retracts; we reproduce that as a damped-spring height
//! so the
//! spike erupts, overshoots and vibrates down to rest (see
//! `spike_util::spring_height_scale`). The small ring shards just rise a
//! little and hold. The screen shake is skipped (no effect-layer
//! hook).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::{ICE_TEXTURE, STONE_TEXTURE};
use crate::effects::spike_util::{
    FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, rise_step, spring_height_scale,
};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE, ICE_TEXTURE];

/// Per-variant differences — only the blade texture changes between Earth
/// Spike and Hyousensou. Both keep `BlendKind::Alpha`: the textures carry
/// their own opacity (opaque brown stone, white-blue ice crystal).
#[derive(Clone, Copy)]
pub struct EarthSpikeParams {
    pub texture: &'static str,
}

/// `F1 = 0` — Wizard Earth Spike, brown stone blades.
pub const EARTHSPIKE: EarthSpikeParams = EarthSpikeParams {
    texture: STONE_TEXTURE,
};
/// `F1 = 2` — Hyousensou, white-blue ice crystal blades.
pub const HYOUSENSOU: EarthSpikeParams = EarthSpikeParams {
    texture: ICE_TEXTURE,
};

const RING_COUNT: usize = 6;
const RING_RADIUS: f32 = 3.0;

// The reference settles on one dominant central spike ringed by small shards:
// the ring blades launch big (half-size 4..5) but retract almost
// immediately, so the lasting silhouette is a tall narrow
// centre over short stubs. We bake that end-state directly — tall central,
// small ring — rather than animating the retraction.
const CENTER_TILT_DEG: f32 = 90.0;
const CENTER_SIZE: f32 = 2.4;
const CENTER_HEIGHT: f32 = 12.0;
const RING_TILT_DEG: f32 = 100.0;
const RING_SIZE: f32 = 1.2;
const RING_HEIGHT: f32 = 4.0;

const SPIKE_SPEED_PER_S: f32 = 0.12 * FRAMES_PER_SECOND;
const SPEED_LIMIT_S: f32 = 12.0 / FRAMES_PER_SECOND;

// Central spike vibration: erupts, overshoots ~20%, rings down over ~0.4 s.
// `omega` puts the first overshoot near frame 7 (`π/OMEGA ≈ 0.12 s`); `decay`
// damps it to a few visible oscillations (the original retracts once, but the
// gif reads as a short ring-down).
const SPRING_OMEGA: f32 = 26.0;
const SPRING_DECAY: f32 = 13.0;
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
    rest_height: f32,
    /// Central spike vibrates its height (erupts + ring-down); ring shards
    /// hold a fixed height and rise slightly via `velocity` instead.
    vibrate: bool,
}

pub struct EarthSpikeEffect {
    spikes: Vec<Spike>,
    age: f32,
    texture: &'static str,
}

impl EarthSpikeEffect {
    pub fn new(world_pos: [f32; 3], params: EarthSpikeParams) -> Self {
        let mut spikes = Vec::with_capacity(RING_COUNT + 1);
        // Central blade — vertical, narrow, tall. Anchored at the ground and
        // vibrated via its height, so it erupts and rings down in place.
        spikes.push(Spike {
            base: world_pos,
            velocity: [0.0; 3],
            tilt_deg: CENTER_TILT_DEG,
            heading_deg: 0.0,
            size: CENTER_SIZE,
            rest_height: CENTER_HEIGHT,
            vibrate: true,
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
                rest_height: RING_HEIGHT,
                vibrate: false,
            });
        }
        Self {
            spikes,
            age: 0.0,
            texture: params.texture,
        }
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
            let height = if s.vibrate {
                s.rest_height * spring_height_scale(self.age, SPRING_OMEGA, SPRING_DECAY)
            } else {
                s.rest_height
            };
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: s.base,
                size: s.size,
                height,
                tilt_x_deg: s.tilt_deg,
                rotation_y_deg: s.heading_deg,
                texture: self.texture,
                color: [1.0, 1.0, 1.0, alpha],
                // Textures carry their own opacity (brown stone / blue ice) —
                // alpha keeps the colour (cf. grimtooth).
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
        // central blade dominates (taller) and the ring blades sit out at
        // radius as small shards; the effect ends after its window.
        let mut e = EarthSpikeEffect::new([0.0, 0.0, 0.0], EARTHSPIKE);

        // Layout at spawn: 1 central on the anchor + 6 ring blades at radius.
        let spawn = draws(&e);
        assert_eq!(spawn.len(), 7);
        match &spawn[0] {
            EffectPrimitiveDraw::QuadHorn { base, texture, .. } => {
                assert_eq!(*texture, STONE_TEXTURE);
                assert!(
                    base[0].abs() < 1e-3 && base[2].abs() < 1e-3,
                    "central on anchor"
                );
            }
            _ => panic!("expected QuadHorn"),
        }
        for p in &spawn[1..] {
            let EffectPrimitiveDraw::QuadHorn { base, .. } = p else {
                panic!("expected QuadHorn")
            };
            let r = (base[0] * base[0] + base[2] * base[2]).sqrt();
            assert!((r - RING_RADIUS).abs() < 1e-3, "ring blade at radius");
        }

        // Once the central spring grows it dominates every ring shard.
        for _ in 0..8 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let grown = draws(&e);
        let central_h = match &grown[0] {
            EffectPrimitiveDraw::QuadHorn { height, .. } => *height,
            _ => unreachable!(),
        };
        for p in &grown[1..] {
            let EffectPrimitiveDraw::QuadHorn { height, .. } = p else {
                panic!("expected QuadHorn")
            };
            assert!(*height < central_h, "central blade is the tallest");
        }

        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_MS as f32 / 1000.0 + 0.1 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
            t += 1.0 / 60.0;
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn central_blade_erupts_overshoots_then_settles() {
        // Sociable test: the central spike vibrates its height — near zero at
        // spawn, overshooting above its rest height during the eruption, then
        // ringing back down to rest. Its base stays planted at the anchor.
        let mut e = EarthSpikeEffect::new([0.0, 0.0, 0.0], EARTHSPIKE);

        let central_height = |e: &EarthSpikeEffect| match &draws(e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, height, .. } => {
                assert_eq!(base[1], 0.0, "central base stays planted");
                *height
            }
            _ => unreachable!(),
        };

        let h_spawn = central_height(&e);
        assert!(h_spawn < CENTER_HEIGHT, "erupts from near zero: {h_spawn}");

        // Peak overshoot lands around frame 7 (first half-period).
        for _ in 0..7 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let h_peak = central_height(&e);
        assert!(
            h_peak > CENTER_HEIGHT,
            "overshoots its rest height: {h_peak}"
        );

        // After the spring settles it rings back down to ~rest height.
        for _ in 0..40 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let h_settled = central_height(&e);
        assert!(
            h_settled < h_peak,
            "rings back down: {h_peak} -> {h_settled}"
        );
        assert!(
            (h_settled - CENTER_HEIGHT).abs() < 1.0,
            "settles near rest: {h_settled}"
        );
    }

    #[test]
    fn hyousensou_shares_geometry_with_ice_texture() {
        // Sociable test: the F1=2 variant emits the same 7-blade layout as
        // Earth Spike but textured with ice instead of stone.
        let mut stone = EarthSpikeEffect::new([0.0, 0.0, 0.0], EARTHSPIKE);
        let mut ice = EarthSpikeEffect::new([0.0, 0.0, 0.0], HYOUSENSOU);
        stone.update(&EffectUpdateCtx {
            delta: 0.0,
            camera_target: None,
            caster_yaw: None,
        });
        ice.update(&EffectUpdateCtx {
            delta: 0.0,
            camera_target: None,
            caster_yaw: None,
        });
        let (sp, ip) = (draws(&stone), draws(&ice));
        assert_eq!(sp.len(), ip.len(), "same blade count");
        for p in &ip {
            let EffectPrimitiveDraw::QuadHorn { texture, .. } = p else {
                panic!("expected QuadHorn");
            };
            assert_eq!(*texture, ICE_TEXTURE, "hyousensou blades use ice");
        }
        match (&sp[0], &ip[0]) {
            (
                EffectPrimitiveDraw::QuadHorn {
                    base: sb, size: ss, ..
                },
                EffectPrimitiveDraw::QuadHorn {
                    base: ib, size: is, ..
                },
            ) => {
                assert_eq!(sb, ib, "central blade at same position");
                assert_eq!(ss, is, "central blade same size");
            }
            _ => panic!("expected QuadHorn"),
        }
    }
}
