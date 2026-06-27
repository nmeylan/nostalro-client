//! `EF_BANJJAKII` (id 165) — a single twinkling sparkle sprite.

use super::spike_burst::seed_from_world;
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SPARKLE_SPRITE: &str = "data/sprite/이팩트/크리스마스";
pub const SPRITES: &[&str] = &[SPARKLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 125.0;
const Y_OFFSET: f32 = -10.0;
/// `random(10) % 3 == 1` → invisible one time in three.
const TINY_SCALE: f32 = 0.01;
const NORMAL_SCALE: f32 = 1.0;
const ANIM_FRAMES_PER_MOTION: f32 = 2.0;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct BanjjakiiEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    action_index: usize,
    size_scale: f32,
}

impl BanjjakiiEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let seed = seed_from_world(world_pos);
        let action_index = ((seed % 5) / 2) as usize;
        let size_scale = if (seed / 5) % 3 == 1 {
            TINY_SCALE
        } else {
            NORMAL_SCALE
        };
        Self {
            world_pos,
            age_frames: 0.0,
            action_index,
            size_scale,
        }
    }
}

impl Effect for BanjjakiiEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let motion = (self.age_frames / ANIM_FRAMES_PER_MOTION) as usize;
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: SPARKLE_SPRITE,
            position: [
                self.world_pos[0],
                self.world_pos[1] + Y_OFFSET,
                self.world_pos[2],
            ],
            action_index: self.action_index,
            motion_index: motion,
            size_scale: self.size_scale,
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Additive,
            aim_target: None,
            no_depth: false,
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

    #[test]
    fn emits_one_sparkle_sprite_with_motion_advancing() {
        let mut e = BanjjakiiEffect::new([0.0; 3]);
        e.update(&ctx(2.0 / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let sprites: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPARKLE_SPRITE))
            .collect();
        assert_eq!(sprites.len(), 1);
        let early_motion =
            if let EffectPrimitiveDraw::SpriteParticle { motion_index, .. } = sprites[0] {
                *motion_index
            } else {
                unreachable!()
            };

        e.update(&ctx(20.0 / FRAMES_PER_SECOND));
        let mut list2 = EffectDrawList::new();
        e.collect_draws(&mut list2, &render_ctx());
        if let EffectPrimitiveDraw::SpriteParticle { motion_index, .. } = list2.primitives[0] {
            assert!(
                motion_index > early_motion,
                "anim advances {early_motion} → {motion_index}"
            );
        }
    }

    #[test]
    fn dies_after_duration() {
        let mut e = BanjjakiiEffect::new([0.0; 3]);
        let s = e.update(&ctx((DURATION_FRAMES + 1.0) / FRAMES_PER_SECOND));
        assert_eq!(s, EffectStatus::Dead);
    }
}
