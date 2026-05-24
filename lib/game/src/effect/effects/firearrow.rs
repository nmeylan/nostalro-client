//! `EF_FIREARROW` — Archer Fire Arrow (id 31).
//!
//! Original game's FireArrow emits one cross-texture — two
//! perpendicular textured quads — cycling through 6 flame frames
//! (`archers1-6.tga`, stored in GRF as `불화살1-8.tga`). The cross spawns at
//! frame 12 with a 70-frame lifetime, oriented along a launch trajectory.
//! Trail particles (`particle4.spr`) emit every 4 frames, plus a
//! `ring_yellow.tga` impact ring on hit.
//!
//! We approximate the cross as two perpendicular `Billboard`s with the flame
//! textures cycling each frame.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FLAME_TEXTURES: &[&str] = &[
    "불화살1.tga",
    "불화살2.tga",
    "불화살3.tga",
    "불화살4.tga",
    "불화살5.tga",
    "불화살6.tga",
    "불화살7.tga",
    "불화살8.tga",
];
pub const TEXTURES: &[&str] = FLAME_TEXTURES;

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 96.0;
const TEXTURE_COUNT: usize = 8;
const ANIM_SPEED: f32 = 1.0;
const FADE_OUT_FRAMES: f32 = 20.0;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const WIDTH: f32 = 4.0;
const HEIGHT: f32 = 1.0;

pub struct FireArrowEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl FireArrowEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }
}

impl Effect for FireArrowEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let t = (self.age_frames / DURATION_FRAMES).clamp(0.0, 1.0);
        let fade_out_start = 1.0 - FADE_OUT_FRAMES / DURATION_FRAMES;
        let alpha = if t < fade_out_start {
            1.0
        } else {
            1.0 - (t - fade_out_start) / (1.0 - fade_out_start)
        };

        let tex_step = (self.age_frames * ANIM_SPEED) as usize;
        let tex_idx = tex_step % TEXTURE_COUNT;
        let texture = FLAME_TEXTURES[tex_idx];

        let pos = [self.world_pos[0], self.world_pos[1] - 1.5, self.world_pos[2]];

        for rotation in [0.0_f32, std::f32::consts::FRAC_PI_2] {
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [WIDTH, HEIGHT],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation,
                texture,
                color: [1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)],
                blend: BlendKind::Additive,
            });
        }
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

    fn draws(e: &FireArrowEffect) -> Vec<(String, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, color, .. } => {
                    (texture.to_string(), color[3])
                }
                _ => unreachable!(),
            })
            .collect()
    }

    #[test]
    fn emits_two_perpendicular_billboards_with_cycling_textures() {
        let mut e = FireArrowEffect::new([0.0; 3]);
        e.update(&ctx(0.5 / FRAMES_PER_SECOND));
        let d = draws(&e);
        assert_eq!(d.len(), 2, "cross = two billboards");
        assert!(d[0].0.starts_with("불화살"));

        e.update(&ctx(10.0 / FRAMES_PER_SECOND));
        let d2 = draws(&e);
        assert_ne!(d[0].0, d2[0].0, "texture should cycle");
    }

    #[test]
    fn dies_after_duration() {
        let mut e = FireArrowEffect::new([0.0; 3]);
        let status = e.update(&ctx(DURATION_FRAMES / FRAMES_PER_SECOND + 0.01));
        assert_eq!(status, EffectStatus::Dead);
    }
}
