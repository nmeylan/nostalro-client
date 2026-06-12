//! `EF_FLASHER` (id 99) — Hunter's Flasher trap detonation.
//!
//! The detonation has two layers, both spawned at frame 0:
//!
//!   * 1 central halo disc using `thunder_center.bmp` with
//!     additive blend. 70-frame lifetime, max alpha
//!     200, fades in over ~10 frames (one sixth of max per frame),
//!     fades out over the last 10 frames. The original timeline also
//!     scales the texture mid-life (around frames 40 and 100);
//!     we approximate as a growing-then-shrinking disc.
//!   * 20 radial spikes (`alpha_center.tga`) — same
//!     shape as Bash but tuned for a longer, brighter burst:
//!     grows ~5..8/frame and runs the full 70-frame
//!     lifetime. See [`super::spike_burst`] for the shared
//!     radial-spike machinery.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::{
    self, SpikeBurst, SpikeBurstParams, fade_in_out, seed_from_world,
};

pub const CENTER_TEXTURE: &str = "thunder_center.bmp";
pub const TEXTURES: &[&str] = &[CENTER_TEXTURE, spike_burst::SPIKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 70.0;

// Central thunder_center halo. The original's ~20 px width
// reads as ~2 wu; the gif shows it dominating roughly half the
// frame, so we scale up to a torso-radius glow. Two stacked discs (bright
// core + softer halo) reproduce the layered silhouette in the gif.
const HALO_HEIGHT_OFFSET: f32 = -5.0;
const HALO_CORE_RADIUS: f32 = 4.5;
const HALO_RIM_RADIUS: f32 = 10.0;
const HALO_CORE_MAX_ALPHA: f32 = 230.0 / 255.0;
const HALO_RIM_MAX_ALPHA: f32 = 150.0 / 255.0;
const HALO_FADE_IN_FRAMES: f32 = 6.0;
const HALO_FADE_OUT_AT: f32 = DURATION_FRAMES - 10.0;
const HALO_TINT: [f32; 3] = [0.85, 0.9, 1.0];

pub const SPIKES: SpikeBurstParams = SpikeBurstParams {
    count: 20,
    duration_frames: DURATION_FRAMES,
    angular_speed_deg_range: (1.0, 7.0),
    length_init_range: (3.5, 6.5),
    growth_range: (5.0 / 6.0, 8.0 / 6.0),
    change_growth: None,
    thickness: 0.5,
    max_alpha: 200.0 / 255.0,
    fade_in_frames: 10.0,
    fade_out_start_frame: SpikeBurstParams::default_fade_out_start(DURATION_FRAMES),
    height_offset: HALO_HEIGHT_OFFSET,
    texture: spike_burst::SPIKE_TEXTURE,
    color_tint: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
};

pub const TOTAL_DURATION_MS: u32 =
    (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct FlasherEffect {
    world_pos: [f32; 3],
    spikes: SpikeBurst,
    age_frames: f32,
}

impl FlasherEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            spikes: SpikeBurst::new(SPIKES, seed_from_world(world_pos)),
            age_frames: 0.0,
        }
    }
}

impl Effect for FlasherEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        self.spikes.tick(ctx.delta);
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let pos = [
            self.world_pos[0],
            self.world_pos[1] + HALO_HEIGHT_OFFSET,
            self.world_pos[2],
        ];
        let push_halo = |out: &mut EffectDrawList, radius: f32, max_alpha: f32| {
            let a = fade_in_out(
                self.age_frames,
                max_alpha,
                HALO_FADE_IN_FRAMES,
                HALO_FADE_OUT_AT,
                DURATION_FRAMES,
            );
            if a > 0.0 {
                out.push(EffectPrimitiveDraw::BillboardDisc {
                    pos,
                    radius,
                    segments: 32,
                    uv_repeat: 1.0,
                    texture: CENTER_TEXTURE,
                    color: [HALO_TINT[0], HALO_TINT[1], HALO_TINT[2], a],
                    // Emissive glow → additive blend.
                    blend: BlendKind::Additive,
                });
            }
        };
        push_halo(out, HALO_RIM_RADIUS, HALO_RIM_MAX_ALPHA);
        push_halo(out, HALO_CORE_RADIUS, HALO_CORE_MAX_ALPHA);

        self.spikes.collect_draws(out, self.world_pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn central_halo_plus_twenty_spikes_with_additive_blend() {
        let mut e = FlasherEffect::new([0.0; 3]);
        e.update(&ctx(8.0 / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let halos: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardDisc { texture, .. } if *texture == CENTER_TEXTURE))
            .collect();
        let spikes = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, .. } if *texture == spike_burst::SPIKE_TEXTURE))
            .count();
        assert_eq!(halos.len(), 2);
        assert_eq!(spikes, SPIKES.count);
        // Emissive halo — additive.
        for h in halos {
            if let EffectPrimitiveDraw::BillboardDisc { blend, .. } = h {
                assert_eq!(*blend, BlendKind::Additive);
            }
        }
    }

    #[test]
    fn dies_after_seventy_frames() {
        let mut e = FlasherEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(DURATION_FRAMES as i32 + 2) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
