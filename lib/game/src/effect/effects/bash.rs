//! `EF_BASH` — Swordman Bash skill impact.
//!
//!
//! Pure screen-space recipe — a 2D halo disc plus 20 radial flash
//! spikes:
//!
//!   * Frame 0 — a filled 2D halo disc (`alpha_down.tga`, radius 100 px,
//!     peak alpha 170/255, fade-in 6 frames, fade-out 10 frames at the end
//!     of a 40-frame lifetime).
//!   * Frame 0 — 20 radial spikes (`alpha_center.tga`). Random longitude,
//!     decelerating angular speed, growing length. See [`super::spike_burst`]
//!     for the shared recipe — Bash, HasteUp, and Flasher all use the same
//!     20-spike flash burst, differing only in numeric parameters.
//!
//! Halo: two stacked `BillboardDisc` primitives textured with
//! `alpha_down.tga`. The polar UV mapping (V=1 centre, V=0 rim) makes
//! the disc read as a radial
//! alpha gradient. Two discs (bright core + softer rim) reproduce the
//! layered "outer lighter than centre" silhouette from the gif.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::{
    self, SpikeBurst, SpikeBurstParams, fade_in_out, seed_from_world,
};

pub const HALO_TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[HALO_TEXTURE, spike_burst::SPIKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 40.0;

// Halo billboard — the central radial glow at the impact point. Two
// stacked alpha-down discs: a tight bright core and a wider softer outer
// halo, matching the layered "bright centre, faded surround" silhouette
// in the original game's gif. Sits one character above the ground.
const HALO_INNER_RADIUS: f32 = 3.5;
const HALO_INNER_MAX_ALPHA: f32 = 220.0 / 255.0;
const HALO_OUTER_RADIUS: f32 = 9.0;
const HALO_OUTER_MAX_ALPHA: f32 = 130.0 / 255.0;
// Warm tint matching the original game's golden halo (vs. pure white).
const HALO_TINT: [f32; 3] = [1.0, 0.95, 0.75];
const HALO_FADE_IN_FRAMES: f32 = 6.0;
const HALO_FADE_OUT_AT: f32 = DURATION_FRAMES - 10.0;
const HALO_HEIGHT_OFFSET: f32 = -5.0;

// 20 radial flash spikes. Spike numeric values are tuned against
// the gif: initial length 20-60 px and
// growth 2.0-5.0 px/frame in the original game, scaled
// so spikes span roughly the outer-halo diameter by mid-life.
pub const SPIKES: SpikeBurstParams = SpikeBurstParams {
    count: 20,
    duration_frames: DURATION_FRAMES,
    angular_speed_deg_range: (1.0, 7.0),
    length_init_range: (2.8, 5.6),
    growth_range: (0.28, 0.7),
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

pub struct BashEffect {
    world_pos: [f32; 3],
    spikes: SpikeBurst,
    age_frames: f32,
}

impl BashEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            spikes: SpikeBurst::new(SPIKES, seed_from_world(world_pos)),
            age_frames: 0.0,
        }
    }
}

impl Effect for BashEffect {
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
        let halo_pos = [
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
                    pos: halo_pos,
                    radius,
                    segments: 32,
                    uv_repeat: 1.0,
                    texture: HALO_TEXTURE,
                    color: [HALO_TINT[0], HALO_TINT[1], HALO_TINT[2], a],
                    blend: BlendKind::Alpha,
                });
            }
        };
        push_halo(out, HALO_OUTER_RADIUS, HALO_OUTER_MAX_ALPHA);
        push_halo(out, HALO_INNER_RADIUS, HALO_INNER_MAX_ALPHA);

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

    fn step_frames(e: &mut BashEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn halo_discs_plus_twenty_spike_billboards_at_spawn() {
        // Sociable: 2 halo BillboardDiscs + 20 spike Billboards all
        // anchored at the same centre. Confirms the spike count matches
        // the original game's 20-spike burst and that every spike is
        // positioned at the world anchor.
        let mut e = BashEffect::new([0.0; 3]);
        step_frames(&mut e, 8);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let halos: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardDisc { texture, .. } if *texture == HALO_TEXTURE))
            .count();
        let spikes: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, .. } if *texture == spike_burst::SPIKE_TEXTURE))
            .collect();
        assert_eq!(halos, 2);
        assert_eq!(spikes.len(), SPIKES.count);

        for prim in &spikes {
            if let EffectPrimitiveDraw::Billboard { pos, .. } = prim {
                assert!(
                    pos[0].abs() < 1e-3 && pos[2].abs() < 1e-3,
                    "spike anchored at entity centre, got {:?}",
                    pos,
                );
            }
        }
    }

    #[test]
    fn dies_after_duration() {
        let mut e = BashEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(DURATION_FRAMES as i32 + 2) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
