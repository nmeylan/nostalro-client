//! `EF_SPEARBMR` (id 80) — Spear Boomerang's projectile barrage on the target.
//!
//! Reference gif `50-100/80.gif` (a thin spear streaking to the target).
//! Four spear sprites
//! (`data/sprite/이팩트/창`) launched
//! at frames 0, 4, 8, 12, each flying from caster to target in 15 frames
//! and stopping on arrival. The
//! sprite faces along travel. Successive spears are launched
//! at decreasing alpha (255 / 180 / 130 / 80) for a trailing-barrage look.

use crate::draw::{aim_backward, BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SPEAR_SPRITE: &str = "data/sprite/이팩트/창";
pub const SPRITES: &[&str] = &[SPEAR_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const WORLD_SCALE: f32 = 0.3;
/// Lifts the flight path to chest height (−Y = up).
const FLIGHT_Y_OFFSET: f32 = -15.0 * WORLD_SCALE;
/// Each spear reaches the target in 15 frames.
const FLIGHT_FRAMES: f32 = 15.0;
const SPAWN_FRAMES: [f32; 4] = [0.0, 4.0, 8.0, 12.0];
const ALPHAS: [f32; 4] = [255.0 / 255.0, 180.0 / 255.0, 130.0 / 255.0, 80.0 / 255.0];
const SPEAR_SIZE: f32 = 1.0;
const ANIM_FRAMES_PER_MOTION: f32 = 2.0;

const TOTAL_FRAMES: f32 = SPAWN_FRAMES[3] + FLIGHT_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Reaches the target in a fixed frame count; each spear covers any distance in
/// `FLIGHT_FRAMES`, so the time is distance-independent.
pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::FixedFrames(SPAWN_FRAMES[0] + FLIGHT_FRAMES);

pub struct SpearBmrEffect {
    from: [f32; 3],
    to: [f32; 3],
    age_frames: f32,
}

impl SpearBmrEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let lift = |p: [f32; 3]| [p[0], p[1] + FLIGHT_Y_OFFSET, p[2]];
        Self { from: lift(from), to: lift(to), age_frames: 0.0 }
    }

    fn spear_pos(&self, local_frame: f32) -> [f32; 3] {
        let t = (local_frame / FLIGHT_FRAMES).clamp(0.0, 1.0);
        [
            self.from[0] + (self.to[0] - self.from[0]) * t,
            self.from[1] + (self.to[1] - self.from[1]) * t,
            self.from[2] + (self.to[2] - self.from[2]) * t,
        ]
    }
}

impl Effect for SpearBmrEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for (i, &spawn) in SPAWN_FRAMES.iter().enumerate() {
            let local = self.age_frames - spawn;
            if !(0.0..FLIGHT_FRAMES).contains(&local) {
                continue;
            }
            let motion = (local / ANIM_FRAMES_PER_MOTION) as usize;
            let pos = self.spear_pos(local);
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: SPEAR_SPRITE,
                position: pos,
                action_index: 0,
                motion_index: motion,
                size_scale: SPEAR_SIZE,
                color: [1.0, 1.0, 1.0, ALPHAS[i]],
                blend: BlendKind::Alpha,
                // Spear SPR points opposite the arrow convention, so the shared
                // screen-space aim lands 180° off. Aim at the mirrored point to
                // add the half-turn.
                aim_target: Some(aim_backward(pos, self.to)),
                no_depth: false,
            });
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

    fn spears(e: &mut SpearBmrEffect, frames: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&ctx(frames / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .into_iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPEAR_SPRITE))
            .collect()
    }

    #[test]
    fn staggers_spears_then_all_in_flight() {
        let mut e = SpearBmrEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 30.0]);
        // At frame ~1 only the first spear is airborne.
        assert_eq!(spears(&mut e, 1.0).len(), 1);
        // By frame ~13 all four have launched and none has landed yet.
        assert_eq!(spears(&mut e, 12.0).len(), 4);
    }

    #[test]
    fn spear_travels_from_caster_toward_target() {
        let mut e = SpearBmrEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 30.0]);
        let early = spears(&mut e, 2.0);
        let z_early = match early[0] {
            EffectPrimitiveDraw::SpriteParticle { position, .. } => position[2],
            _ => unreachable!(),
        };
        let later = spears(&mut e, 6.0);
        let z_later = match later[0] {
            EffectPrimitiveDraw::SpriteParticle { position, .. } => position[2],
            _ => unreachable!(),
        };
        assert!(z_later > z_early, "spear advances toward +Z target {z_early} → {z_later}");
    }

    #[test]
    fn dies_after_last_spear_lands() {
        let mut e = SpearBmrEffect::new([0.0; 3], [0.0, 0.0, 30.0]);
        let s = e.update(&ctx((TOTAL_FRAMES + 1.0) / FRAMES_PER_SECOND));
        assert_eq!(s, EffectStatus::Dead);
    }
}
