//! `EF_BARRIER` (id 63) — a single faceted energy sphere flashing around the
//! caster (Energy Coat / barrier cast).
//!
//! Reference: the original game gif library `50-100/63.gif`
//! (16 frames ≈ 250 ms). The effect is a
//! single energy sphere:
//!
//! ```text
//! duration   = 250 ms
//! radius     = 13.5
//! delta_y    = -10        // raised to body centre (native RO -Y = up)
//! longitude  = 2          // facets around the sphere
//! alpha      = 100
//! texture    = bigbang.tga
//! growth     = 0           // radius/orientation never change
//! ```
//!
//! The sphere's radius and orientation never change over its life, so the
//! sphere is static in size and orientation — it simply fades in, holds, and
//! fades out over its 250 ms life. There are no cylinder / particle /
//! quad-horn layers around it; the gif confirms a
//! lone shimmering sphere.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SPHERE_TEXTURE: &str = "bigbang.tga";
pub const TEXTURES: &[&str] = &[SPHERE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 15.0;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

// radius = 13.5 in source units; a character is ~5–8 world units here, so a
// ~0.5× port keeps the sphere about one character across (matching the gif).
const WORLD_SCALE: f32 = 0.5;
const SPHERE_RADIUS: f32 = 13.5 * WORLD_SCALE;
// delta_y = -10 lifts the sphere to body centre.
const SPHERE_Y_OFFSET: f32 = -10.0 * WORLD_SCALE;
// alpha = 100 (of 255).
const PEAK_ALPHA: f32 = 100.0 / 255.0;
const FADE_IN_FRAMES: f32 = 3.0;
const FADE_OUT_START_FRAME: f32 = DURATION_FRAMES - 5.0;
const SPHERE_SIDES_LAT: u32 = 5;
const SPHERE_SIDES_LON: u32 = 10;

pub struct BarrierEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl BarrierEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }

    fn alpha(&self) -> f32 {
        let f = self.age_frames;
        if f < 0.0 || f >= DURATION_FRAMES {
            return 0.0;
        }
        let in_curve = (f / FADE_IN_FRAMES).clamp(0.0, 1.0);
        let out_curve = if f <= FADE_OUT_START_FRAME {
            1.0
        } else {
            ((DURATION_FRAMES - f) / (DURATION_FRAMES - FADE_OUT_START_FRAME)).max(0.0)
        };
        PEAK_ALPHA * in_curve * out_curve
    }
}

impl Effect for BarrierEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let a = self.alpha();
        if a <= 0.0 {
            return;
        }
        out.push(EffectPrimitiveDraw::Sphere {
            center: [
                self.world_pos[0],
                self.world_pos[1] + SPHERE_Y_OFFSET,
                self.world_pos[2],
            ],
            radius: SPHERE_RADIUS,
            sides_lat: SPHERE_SIDES_LAT,
            sides_lon: SPHERE_SIDES_LON,
            longitude_offset: 0.0,
            longitude_arc: std::f32::consts::TAU,
            uv_repeat: [1.0, 1.0],
            texture: SPHERE_TEXTURE,
            // Warm yellow energy shell (matches the original game's tint).
            color: [1.0, 0.92, 0.45, a],
            blend: BlendKind::Additive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draw_at(e: &mut BarrierEffect, frame: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&ctx(frame / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_single_sphere_centred_on_caster() {
        let mut e = BarrierEffect::new([10.0, 0.0, 20.0]);
        let prims = draw_at(&mut e, 5.0);
        let spheres: Vec<&EffectPrimitiveDraw> = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Sphere { texture, .. } if *texture == SPHERE_TEXTURE))
            .collect();
        assert_eq!(spheres.len(), 1);
        if let EffectPrimitiveDraw::Sphere { center, .. } = spheres[0] {
            assert!((center[0] - 10.0).abs() < 1e-3 && (center[2] - 20.0).abs() < 1e-3);
        }
    }

    #[test]
    fn fades_in_then_out() {
        let mut e = BarrierEffect::new([0.0; 3]);
        let early = e.alpha();
        e.update(&ctx(5.0 / FRAMES_PER_SECOND));
        let mid = e.alpha();
        e.update(&ctx((DURATION_FRAMES - 6.0) / FRAMES_PER_SECOND));
        let late = e.alpha();
        assert!(mid > early, "fades in {early} → {mid}");
        assert!(late < mid, "fades out {mid} → {late}");
    }

    #[test]
    fn dies_after_duration() {
        let mut e = BarrierEffect::new([0.0; 3]);
        let s = e.update(&ctx((DURATION_FRAMES + 1.0) / FRAMES_PER_SECOND));
        assert_eq!(s, EffectStatus::Dead);
    }
}
