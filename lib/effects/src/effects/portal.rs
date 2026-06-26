//! `EF_PORTAL` — warp portal visual: a sustained pair of rotating ring
//! columns over a pulsing ground pad.
//!
//! Sustained composite (persistent until the portal NPC is removed):
//!
//!   * Frame 0 — two nested rotating cylinder columns sharing the
//!     `ring_blue.tga` texture:
//!     * inner: radius 3.5, height 40, spin 4°/frame, max alpha 128/255
//!     * outer: radius 4.0, height 50, spin 5°/frame, max alpha 128/255
//!     Both ramp alpha 0 → 128 over 6 frames and hold.
//!   * Ground pad — reuses `EF_READYPORTAL`'s
//!     ground-disc emission (every 14 frames, `ring_blue.tga`).
//!     Shared via [`super::ready_portal::ReadyPortalDiscEmitter`].

use super::ready_portal::ReadyPortalDiscEmitter;
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

// Two cylinder columns.
const CYL_INNER_RADIUS: f32 = 3.5;
const CYL_INNER_HEIGHT: f32 = 40.0;
const CYL_INNER_SPIN_DEG_PER_FRAME: f32 = 4.0;
const CYL_OUTER_RADIUS: f32 = 4.0;
const CYL_OUTER_HEIGHT: f32 = 50.0;
const CYL_OUTER_SPIN_DEG_PER_FRAME: f32 = 5.0;
const CYL_MAX_ALPHA: f32 = 128.0 / 255.0;
const CYL_FADE_IN_FRAMES: f32 = 6.0;
const CYL_SIDES: u32 = 10; // arcAngle 36° → 360 / 36 = 10 segments.

pub const TOTAL_DURATION_MS: u32 = 99990;

fn cylinder_alpha(frame: f32) -> f32 {
    (frame / CYL_FADE_IN_FRAMES).clamp(0.0, 1.0) * CYL_MAX_ALPHA
}

pub struct PortalEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    disc_emitter: ReadyPortalDiscEmitter,
}

impl PortalEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
            disc_emitter: ReadyPortalDiscEmitter::new(world_pos),
        }
    }
}

impl Effect for PortalEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;
        // Portal's parent emitter is sustained — keep spawning ground
        // discs for the whole lifetime.
        self.disc_emitter.step(dt_frames, f32::INFINITY);
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = cylinder_alpha(self.age_frames);
        if alpha > 0.0 {
            for &(radius, height, spin) in &[
                (
                    CYL_INNER_RADIUS,
                    CYL_INNER_HEIGHT,
                    CYL_INNER_SPIN_DEG_PER_FRAME,
                ),
                (
                    CYL_OUTER_RADIUS,
                    CYL_OUTER_HEIGHT,
                    CYL_OUTER_SPIN_DEG_PER_FRAME,
                ),
            ] {
                let rotation = (self.age_frames * spin).to_radians();
                out.push(EffectPrimitiveDraw::Cylinder {
                    base: self.world_pos,
                    bottom_size: radius,
                    top_size: radius,
                    height,
                    sides: CYL_SIDES,
                    rotation,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    uv_scroll: [0.0, 0.0],
                    texture: RING_TEXTURE,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Additive,
                });
            }
        }

        self.disc_emitter.collect_draws(out);
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

    fn step_frames(e: &mut PortalEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn two_cylinders_plus_periodic_ground_rings() {
        // Sociable: by frame 30 both cylinders are emitted and the
        // shared ReadyPortal disc emitter has spawned at least two
        // discs (frames 0 and 14).
        let mut e = PortalEffect::new([5.0, 0.0, 7.0]);
        step_frames(&mut e, 30);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let cylinders = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. }))
            .count();
        assert_eq!(cylinders, 2);

        let ground_rings = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { texture, .. } if *texture == RING_TEXTURE))
            .count();
        assert!(
            ground_rings >= 2,
            "ReadyPortal disc respawns every 14 frames"
        );
    }

    #[test]
    fn cylinder_alpha_ramps_to_max() {
        assert_eq!(cylinder_alpha(0.0), 0.0);
        assert!((cylinder_alpha(CYL_FADE_IN_FRAMES) - CYL_MAX_ALPHA).abs() < 1e-3);
        assert!((cylinder_alpha(60.0) - CYL_MAX_ALPHA).abs() < 1e-3);
    }
}
