//! `EF_TELEPORTATION` — vertical blue light beam that shoots up when an
//! actor teleports away. Reuses the same cylinder (`Frustum`) +
//! `ring_blue.tga` recipe as `EF_PORTAL`'s columns; differs in that the
//! cylinder grows in height, spins, shrinks radially, and fades out over
//! its 60-frame life instead of holding steady.
//!
//! Recipe (observed behaviour over a 60-frame life):
//!   * Frame 0 — launch one cylinder with duration 60,
//!     top and bottom radius both 4, height speed 5,
//!     height accel 0.25, spin 2°/frame, and a
//!     disappear pattern, texture `ring_blue.tga`.
//!   * Each tick the cylinder grows in height:
//!     `height_speed += accel; height += height_speed`.
//!   * The bottom ring radius sits at y=0
//!     and the top ring radius sits at y=-height.
//!     The disappear pattern shrinks only the top via
//!     `outer_speed = -(top / (duration + 1))` — the
//!     bottom radius stays at 4 while the top narrows to zero, so the
//!     shape morphs from cylinder → cone → needle.
//!   * Alpha fades linearly
//!     from full to 0 across the cylinder's lifetime.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 60.0;

/// Bottom ring radius (at y=0); the disappear pattern never touches it, so
/// this stays constant.
const BOTTOM_RADIUS: f32 = 4.0;
/// Top ring radius. The disappear pattern sets the shrink rate
/// `-(top / (duration + 1))` on frame 0, narrowing
/// the top to zero over the lifetime.
const TOP_RADIUS_INITIAL: f32 = 4.0;
const TOP_RADIUS_SHRINK_PER_FRAME: f32 = TOP_RADIUS_INITIAL / (DURATION_FRAMES + 1.0);

const HEIGHT_SPEED_PER_FRAME: f32 = 5.0;
const HEIGHT_ACCEL_PER_FRAME2: f32 = 0.25;

const SPIN_DEG_PER_FRAME: f32 = 2.0;
const SIDES: u32 = 10;
const UV_REPEAT: f32 = 1.0;
const MAX_ALPHA: f32 = 1.0;

pub const TOTAL_DURATION_MS: u32 =
    (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct TeleportationEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl TeleportationEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }

    fn top_radius(&self) -> f32 {
        (TOP_RADIUS_INITIAL - TOP_RADIUS_SHRINK_PER_FRAME * self.age_frames).max(0.0)
    }

    fn height(&self) -> f32 {
        // Closed form of `height_speed += a; height += height_speed`
        // after n ticks: h(n) = n*s0 + a*n(n+1)/2.
        let n = self.age_frames.clamp(0.0, DURATION_FRAMES);
        n * HEIGHT_SPEED_PER_FRAME + HEIGHT_ACCEL_PER_FRAME2 * n * (n + 1.0) / 2.0
    }

    fn alpha(&self) -> f32 {
        // Linear fade across the whole life.
        let n = self.age_frames.clamp(0.0, DURATION_FRAMES);
        MAX_ALPHA * (1.0 - n / DURATION_FRAMES)
    }
}

impl Effect for TeleportationEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let top = self.top_radius();
        let height = self.height();
        let alpha = self.alpha();
        if height <= 0.0 || alpha <= 0.0 {
            return;
        }
        let rotation = (self.age_frames * SPIN_DEG_PER_FRAME).to_radians();
        out.push(EffectPrimitiveDraw::Frustum {
            base: self.world_pos,
            bottom_size: BOTTOM_RADIUS,
            top_size: top,
            height,
            sides: SIDES,
            rotation,
            uv_repeat: UV_REPEAT,
            uv_scroll: [0.0, 0.0],
            wave_amplitude: 0.0,
            wave_frequency: 0.0,
            wave_phase: 0.0,
            wave_mode: FrustumWaveMode::Sine,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            cull_back: false,
            texture: RING_TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Additive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut TeleportationEffect, frames: f32) {
        e.update(&ctx(frames / FRAMES_PER_SECOND));
    }

    fn frustum_of(e: &TeleportationEffect) -> Option<(f32, f32, f32, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives.into_iter().find_map(|p| match p {
            EffectPrimitiveDraw::Frustum {
                bottom_size,
                top_size,
                height,
                color,
                texture,
                ..
            } if texture == RING_TEXTURE => Some((bottom_size, top_size, height, color[3])),
            _ => None,
        })
    }

    #[test]
    fn emits_a_blue_ring_frustum_starting_as_a_cylinder() {
        // Sociable: a couple of frames in, exactly one Frustum is emitted
        // with the shared ring_blue.tga texture, additive blend, and the
        // fixed bottom radius.
        let mut e = TeleportationEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 1.0);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1);
        match &list.primitives[0] {
            EffectPrimitiveDraw::Frustum {
                bottom_size,
                top_size,
                texture,
                blend,
                ..
            } => {
                assert_eq!(*texture, RING_TEXTURE);
                assert_eq!(*blend, BlendKind::Additive);
                assert!((bottom_size - BOTTOM_RADIUS).abs() < f32::EPSILON);
                // Bottom stays put while the top is only just starting
                // to narrow — so they're close but the top is already
                // strictly smaller.
                assert!(*top_size < BOTTOM_RADIUS && *top_size > 0.0);
            }
            other => panic!("expected Frustum, got {other:?}"),
        }
    }

    #[test]
    fn top_narrows_while_bottom_stays_and_height_grows_with_alpha_fading() {
        // Sociable: only the top radius shrinks while the bottom stays
        // at 4. Verify all four
        // quantities together over the lifetime.
        let mut e = TeleportationEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 5.0);
        let (b_early, t_early, h_early, a_early) = frustum_of(&e).expect("visible");

        step(&mut e, 25.0);
        let (b_mid, t_mid, h_mid, a_mid) = frustum_of(&e).expect("visible");

        assert!((b_mid - b_early).abs() < 1e-4, "bottom stays constant");
        assert!(t_mid < t_early, "top narrows: {t_early} -> {t_mid}");
        assert!(h_mid > h_early, "height grows: {h_early} -> {h_mid}");
        assert!(a_mid < a_early, "alpha fades: {a_early} -> {a_mid}");
    }

    #[test]
    fn effect_dies_after_sixty_frames() {
        let mut e = TeleportationEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..70 {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
