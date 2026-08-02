//! `EF_SUMMONSLAVE` (id 215) — the summoner's radial flash + smoke puff.
//!
//! The effect has two parts:
//!   * frame 0 — 20 radial flash spikes (`alpha_center.tga`),
//!     the same shape Bash and Flasher use. Each spike spins slowly
//!     (`1..7°`/frame, decelerating), elongates over its 40-frame life and
//!     fades in/out. See [`super::spike_burst`].
//!   * frame 35 — the master emits a smoke puff,
//!     a single one-shot smoke sprite drifting upward
//!     (size 1.5, ~833 ms life, gentle up-drift).
//!
//! The flash spike reads as screen-space in the original game; like Bash and
//! Flasher we render it as world-space billboards straddling the entity
//! centre, which reads identically in a perspective view.

use super::spike_burst::{self, SpikeBurst, SpikeBurstParams, seed_from_world};
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Large source length/growth literals; downscale so the
/// rays fill roughly the same fraction of frame as the gif (the burst spans
/// most of the capture — comparable to Flasher's spike envelope).
const SPIKE_SCALE: f32 = 0.3;
const SPIKE_DURATION_FRAMES: f32 = 40.0;

/// Plays a single one-shot animated smoke poof at the master
/// (a smoke poof sprite), **not** the
/// chimney-smoke particle puffs of the unrelated `EF_SMOKE`.
const SMOKE_SPRITE: &str = ragnarok_resources::sprite::effect::SMOKE;
/// Preloaded so the sprite is cached before frame 35 (otherwise the first
/// emitted SpriteParticle is silently skipped).
pub const SPRITES: &[&str] = &[SMOKE_SPRITE];
pub const TEXTURES: &[&str] = &[spike_burst::SPIKE_TEXTURE];

/// Spike length randomised in 20..60, scaled.
const SPIKE_LENGTH_INIT: (f32, f32) = (20.0 * SPIKE_SCALE, 60.0 * SPIKE_SCALE);
/// Spike growth randomised in 2.0..5.0/frame, scaled.
const SPIKE_GROWTH: (f32, f32) = (2.0 * SPIKE_SCALE, 5.0 * SPIKE_SCALE);

pub const SPIKES: SpikeBurstParams = SpikeBurstParams {
    count: 20,
    duration_frames: SPIKE_DURATION_FRAMES,
    angular_speed_deg_range: (1.0, 7.0),
    length_init_range: SPIKE_LENGTH_INIT,
    growth_range: SPIKE_GROWTH,
    change_growth: None,
    thickness: 0.5,
    max_alpha: 200.0 / 255.0,
    fade_in_frames: 10.0,
    fade_out_start_frame: SpikeBurstParams::default_fade_out_start(SPIKE_DURATION_FRAMES),
    // Lift the burst above ground (native RO −Y is up), scaled.
    height_offset: -40.0 * SPIKE_SCALE,
    texture: spike_burst::SPIKE_TEXTURE,
    color_tint: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
};

/// The smoke poof starts at frame 35 and plays its one-shot animation over
/// ~50 frames, so the effect's wall-clock end is well past the spikes'
/// 40-frame envelope.
const SMOKE_START_FRAME: f32 = 35.0;
const SMOKE_LIFETIME_FRAMES: f32 = 50.0;
const SMOKE_SIZE: f32 = 2.6;
/// Small lift to torso height (native RO: `-Y` is up). The poof draws with
/// `no_depth` so it isn't occluded by the floor regardless.
const SMOKE_HEIGHT_OFFSET: f32 = -3.0;
/// ACT frames advance every other engine frame so the poof animation reads
/// at a smoke-like pace over its lifetime.
const SMOKE_ANIM_DIVISOR: f32 = 2.0;

const TOTAL_FRAMES: f32 = SMOKE_START_FRAME + SMOKE_LIFETIME_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct SummonSlaveEffect {
    world_pos: [f32; 3],
    spikes: SpikeBurst,
    age_frames: f32,
}

impl SummonSlaveEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            spikes: SpikeBurst::new(SPIKES, seed_from_world(world_pos)),
            age_frames: 0.0,
        }
    }
}

impl Effect for SummonSlaveEffect {
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
        self.spikes.collect_draws(out, self.world_pos);

        let smoke_age = self.age_frames - SMOKE_START_FRAME;
        if smoke_age < 0.0 {
            return;
        }
        // Gentle fade over the back third so the poof dissipates rather than
        // snapping off at the lifetime end.
        let fade = (1.0 - (smoke_age / SMOKE_LIFETIME_FRAMES - 0.66) / 0.34).clamp(0.0, 1.0);
        if fade <= 0.0 {
            return;
        }
        // One in-place animated smoke poof (the one-shot `smoke.spr`).
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: SMOKE_SPRITE,
            position: [
                self.world_pos[0],
                self.world_pos[1] + SMOKE_HEIGHT_OFFSET,
                self.world_pos[2],
            ],
            action_index: 0,
            motion_index: (smoke_age / SMOKE_ANIM_DIVISOR) as usize,
            size_scale: SMOKE_SIZE,
            color: [1.0, 1.0, 1.0, fade],
            blend: BlendKind::Alpha,
            aim_target: None,
            // Sits at the caster's feet; draw over the floor instead of being
            // depth-occluded by the ground.
            no_depth: true,
        });
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

    fn step(e: &mut SummonSlaveEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn counts(e: &SummonSlaveEffect) -> (usize, usize) {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        let spikes = l.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, .. } if *texture == spike_burst::SPIKE_TEXTURE)).count();
        let smoke = l
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        (spikes, smoke)
    }

    #[test]
    fn twenty_spikes_at_frame_zero_no_smoke_yet() {
        let mut e = SummonSlaveEffect::new([0.0; 3]);
        step(&mut e, 5);
        let (spikes, smoke) = counts(&e);
        assert_eq!(spikes, 20, "all 20 radial spikes live early");
        assert_eq!(smoke, 0, "smoke only erupts at frame 35");
    }

    #[test]
    fn smoke_erupts_after_frame_35_once_spikes_are_gone() {
        let mut e = SummonSlaveEffect::new([0.0; 3]);
        step(&mut e, 42);
        let (spikes, smoke) = counts(&e);
        assert_eq!(spikes, 0, "spikes die by frame 40");
        assert_eq!(smoke, 1, "one smoke poof sprite plays after frame 35");
    }

    #[test]
    fn smoke_poof_animates_and_effect_terminates() {
        let motion = |e: &SummonSlaveEffect| {
            let mut l = EffectDrawList::new();
            e.collect_draws(&mut l, &render_ctx());
            l.primitives
                .iter()
                .find_map(|p| match p {
                    EffectPrimitiveDraw::SpriteParticle { motion_index, .. } => Some(*motion_index),
                    _ => None,
                })
                .unwrap()
        };
        let mut e = SummonSlaveEffect::new([0.0; 3]);
        step(&mut e, 40);
        let early = motion(&e);
        step(&mut e, 10);
        let later = motion(&e);
        assert!(
            later > early,
            "smoke ACT frame advances over time: {early} -> {later}"
        );
        let status = step(&mut e, (TOTAL_FRAMES as u32) + 2);
        assert_eq!(status, EffectStatus::Dead);
    }
}
