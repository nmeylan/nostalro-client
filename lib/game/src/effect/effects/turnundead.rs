//! `EF_TURNUNDEAD` (id 82) — a two-phase screen burst used by Impositio
//! Manus / Turn Undead.
//!
//! Reference: the
//! original game gif `50-100/82.gif` (radial ray burst, then a ground ring).
//!
//! Phase 1 (frame 0, 20-frame life):
//!   * 1× filled halo disc (`alpha_down.tga`) shrinking inward,
//!     peak alpha 130/255, fade-out from frame 10.
//!   * 5× flash rays (`alpha_center.tga`) — the same shape as
//!     `Bash`/`HasteUp`, but the rays *shrink* (negative growth) instead of
//!     growing. Reuses [`super::spike_burst`].
//! Phase 2 (frame 50, 30-frame life):
//!   * 1× expanding ground ring laid flat
//!     (`ring_liner.tga`), peak alpha 150/255, growth slows at frame 15, fade-out
//!     from frame 20.
//!
//! The parent emitter lasts 250 ms; the visible
//! effect runs to the phase-2 ring's death at frame 80.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::{self, SpikeBurst, SpikeBurstParams, fade_in_out, seed_from_world};

pub const HALO_TEXTURE: &str = "alpha_down.tga";
pub const RING_TEXTURE: &str = "ring_liner.tga";
pub const TEXTURES: &[&str] = &[HALO_TEXTURE, RING_TEXTURE, spike_burst::SPIKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

// Phase 1 — halo disc + 5 shrinking rays.
const PHASE1_DURATION_FRAMES: f32 = 20.0;
const HALO_INITIAL_RADIUS: f32 = 10.0;
const HALO_MAX_ALPHA: f32 = 130.0 / 255.0;
const HALO_FADE_IN_FRAMES: f32 = 6.0;
const HALO_FADE_OUT_AT: f32 = 10.0;
const HALO_HEIGHT_OFFSET: f32 = -5.0;

// 5 flash rays starting 350..380 px long and shrinking
// 15..18 px/frame. Ported to world units: start long, shrink over the
// 20-frame life.
pub const SPIKES: SpikeBurstParams = SpikeBurstParams {
    count: 5,
    duration_frames: PHASE1_DURATION_FRAMES,
    angular_speed_deg_range: (1.0, 7.0),
    length_init_range: (9.0, 15.0),
    growth_range: (-0.3, -0.2),
    change_growth: None,
    thickness: 0.3,
    max_alpha: 200.0 / 255.0,
    fade_in_frames: 10.0,
    // Fade-out starts at duration − duration/3 ≈ frame 13.
    fade_out_start_frame: SpikeBurstParams::default_fade_out_start(PHASE1_DURATION_FRAMES),
    height_offset: HALO_HEIGHT_OFFSET,
    texture: spike_burst::SPIKE_TEXTURE,
    color_tint: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
};

// Phase 2 — expanding ground ring.
const RING_SPAWN_FRAME: f32 = 50.0;
const RING_DURATION_FRAMES: f32 = 30.0;
const RING_MAX_ALPHA: f32 = 150.0 / 255.0;
const RING_FADE_OUT_AT: f32 = 20.0;
const RING_HEIGHT_OFFSET: f32 = -2.0;
// Growth 1.5/frame with accel 0.15, swapping to 0.5 / −0.033 at frame
// 15 — ported uniformly so the ring half-extent peaks ~8 wu.
const RING_SCALE: f32 = 0.5;
const RING_CHANGE_FRAME: f32 = 15.0;

const TOTAL_FRAMES: f32 = RING_SPAWN_FRAME + RING_DURATION_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Half-extent of the phase-2 ring at a given local frame (two-phase
/// growth, scaled).
fn ring_half_extent(frame: f32) -> f32 {
    let raw = if frame <= RING_CHANGE_FRAME {
        1.5 * frame + 0.075 * frame * (frame + 1.0) / 2.0
    } else {
        let s15 = 1.5 * RING_CHANGE_FRAME + 0.075 * RING_CHANGE_FRAME * (RING_CHANGE_FRAME + 1.0) / 2.0;
        let g = frame - RING_CHANGE_FRAME;
        s15 + 0.5 * g - (0.5 / 15.0) * g * (g + 1.0) / 2.0
    };
    raw * RING_SCALE
}

pub struct TurnUndeadEffect {
    world_pos: [f32; 3],
    spikes: SpikeBurst,
    age_frames: f32,
}

impl TurnUndeadEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            spikes: SpikeBurst::new(SPIKES, seed_from_world(world_pos)),
            age_frames: 0.0,
        }
    }
}

impl Effect for TurnUndeadEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        self.spikes.tick(ctx.delta);
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // Phase 1 — shrinking halo disc.
        if self.age_frames < PHASE1_DURATION_FRAMES {
            let a = fade_in_out(
                self.age_frames,
                HALO_MAX_ALPHA,
                HALO_FADE_IN_FRAMES,
                HALO_FADE_OUT_AT,
                PHASE1_DURATION_FRAMES,
            );
            let radius = HALO_INITIAL_RADIUS * (1.0 - self.age_frames / PHASE1_DURATION_FRAMES);
            if a > 0.0 && radius > 0.0 {
                out.push(EffectPrimitiveDraw::BillboardDisc {
                    pos: [self.world_pos[0], self.world_pos[1] + HALO_HEIGHT_OFFSET, self.world_pos[2]],
                    radius,
                    segments: 32,
                    uv_repeat: 1.0,
                    texture: HALO_TEXTURE,
                    color: [0.6, 0.7, 1.0, a],
                    blend: BlendKind::Alpha,
                });
            }
        }

        // Phase 1 — 5 shrinking rays.
        self.spikes.collect_draws(out, self.world_pos);

        // Phase 2 — expanding ground ring.
        let ring_frame = self.age_frames - RING_SPAWN_FRAME;
        if (0.0..RING_DURATION_FRAMES).contains(&ring_frame) {
            let half = ring_half_extent(ring_frame);
            let a = fade_in_out(ring_frame, RING_MAX_ALPHA, 1.0, RING_FADE_OUT_AT, RING_DURATION_FRAMES);
            if half > 0.0 && a > 0.0 {
                out.push(EffectPrimitiveDraw::Texture3D {
                    center: [self.world_pos[0], self.world_pos[1] + RING_HEIGHT_OFFSET, self.world_pos[2]],
                    size: [half, half],
                    plane: QuadPlane::Horizontal,
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: RING_TEXTURE,
                    color: [1.0, 1.0, 1.0, a],
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn draw_at(e: &mut TurnUndeadEffect, frames: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&ctx(frames / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn phase1_emits_disc_plus_five_rays() {
        let mut e = TurnUndeadEffect::new([0.0; 3]);
        let prims = draw_at(&mut e, 3.0);
        let discs = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::BillboardDisc { .. })).count();
        let rays = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, .. } if *texture == spike_burst::SPIKE_TEXTURE)).count();
        let rings = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::Texture3D { .. })).count();
        assert_eq!(discs, 1);
        assert_eq!(rays, 5);
        assert_eq!(rings, 0, "ring has not spawned yet");
    }

    #[test]
    fn phase2_ring_appears_and_grows() {
        let mut e = TurnUndeadEffect::new([0.0; 3]);
        let early = ring_half_extent(2.0);
        let late = ring_half_extent(20.0);
        assert!(late > early, "ring grows {early} → {late}");
        let prims = draw_at(&mut e, 55.0);
        let rings = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::Texture3D { texture, .. } if *texture == RING_TEXTURE)).count();
        assert_eq!(rings, 1, "ring visible at frame 55");
    }

    #[test]
    fn dies_after_phase2() {
        let mut e = TurnUndeadEffect::new([0.0; 3]);
        let s = e.update(&ctx((TOTAL_FRAMES + 1.0) / FRAMES_PER_SECOND));
        assert_eq!(s, EffectStatus::Dead);
    }
}
