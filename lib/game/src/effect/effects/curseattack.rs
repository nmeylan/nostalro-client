//! `EF_CURSEATTACK` — curse mark hovering over the cursed entity (id 194).
//!
//! Original game spawns a persistent (clipped to 1500 ms by the table) curse.bmp
//! 3D-texture quad, yawing 4.5°/frame around the world Y axis. Anchored at
//! the actor; rendered slightly above the head in the original. Native RO
//! coordinates are `-Y = up`, so head-level is `world_pos.y - 1.5` (matching
//! Fireball's per-sprite head-height tweak).

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const CURSE_TEXTURE: &str = "curse.bmp";
pub const TEXTURES: &[&str] = &[CURSE_TEXTURE];

const HALF_SIZE: f32 = 2.0;
const HEAD_OFFSET: f32 = 5.0;
const DEG_PER_FRAME: f32 = 4.5;
const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 1500;
const DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;

pub struct CurseattackEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl CurseattackEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }
}

impl Effect for CurseattackEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frames = self.age * FRAMES_PER_SECOND;
        let yaw = (frames * DEG_PER_FRAME).to_radians();
        let center = [
            self.world_pos[0],
            self.world_pos[1] - HEAD_OFFSET,
            self.world_pos[2],
        ];
        out.push(EffectPrimitiveDraw::Texture3D {
            center,
            size: [HALF_SIZE, HALF_SIZE],
            plane: QuadPlane::VerticalYaw(yaw),
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            texture: CURSE_TEXTURE,
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Alpha,
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

    #[test]
    fn rotates_above_actor_and_dies_after_duration() {
        let mut e = CurseattackEffect::new([0.0, 0.0, 0.0]);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D { center, .. } => {
                assert!(center[1] < 0.0, "mark sits above ground (native RO -Y up)");
            }
            _ => panic!("expected Texture3D"),
        }

        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None, caster_yaw: None,
            });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
