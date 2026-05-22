//! `EF_FIREBALL` — Mage Fireball (id 24).
//!
//! Original game's FireBall runs a single rotating particle
//! sprite that travels from caster toward target. Without projectile
//! threading we render the impact-frame visual: an expanding fireball
//! sprite at the attached point.
//!
//! Lifetime: ~1000 ms (60 frames).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const FIREBALL_SPRITE: &str = "data/sprite/이팩트/fireball";
pub const SPRITES: &[&str] = &[FIREBALL_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 60.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * 3.0;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const BASE_SIZE: f32 = 2.0;

pub struct FireballEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl FireballEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }
}

impl Effect for FireballEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let t = self.age_frames / DURATION_FRAMES;
        let alpha = if t < 0.7 {
            1.0
        } else {
            (1.0 - (t - 0.7) / 0.3).clamp(0.0, 1.0)
        };
        let scale = BASE_SIZE * (1.0 + t * 0.6);
        let pos = [
            self.world_pos[0],
            self.world_pos[1] - 1.5,
            self.world_pos[2],
        ];
        let motion =
            (self.age_frames * (1000.0 / FRAMES_PER_SECOND) / PARTICLE_FRAME_MS) as usize;
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: FIREBALL_SPRITE,
            position: pos,
            motion_index: motion,
            size_scale: scale,
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

    #[test]
    fn emits_single_sprite_then_dies() {
        let mut e = FireballEffect::new([5.0, 0.0, 7.0]);
        e.update(&ctx(0.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1);
        let mut status = EffectStatus::Running;
        for _ in 0..120 {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
