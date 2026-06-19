//! `EF_TOPRANK` — top-ranker indicator (id 159).
//!
//! A persistent
//! `LockOn128.tga` quad over the actor laid flat on the ground
//! plane, spinning 4.5°/frame around the world Y
//! axis. Tint varies by rank tier (red / blue / green); the entity-attached
//! spawn picks the tint outside the effect.

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TOPRANK_TEXTURE: &str = "LockOn128.tga";
pub const TEXTURES: &[&str] = &[TOPRANK_TEXTURE];

const HALF_SIZE: f32 = 4.0;
const DEG_PER_FRAME: f32 = 4.5;
const FRAMES_PER_SECOND: f32 = 60.0;

pub struct ToprankEffect {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
}

impl ToprankEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            tint: [1.0, 0.2, 0.2, 1.0],
            age: 0.0,
        }
    }
}

impl Effect for ToprankEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frames = self.age * FRAMES_PER_SECOND;
        let yaw = (frames * DEG_PER_FRAME).to_radians();
        out.push(EffectPrimitiveDraw::Texture3D {
            center: self.world_pos,
            size: [HALF_SIZE, HALF_SIZE],
            plane: QuadPlane::HorizontalYaw(yaw),
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            texture: TOPRANK_TEXTURE,
            color: self.tint,
            blend: BlendKind::Additive,
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
    fn yaw_advances_per_frame_and_stays_alive() {
        let mut e = ToprankEffect::new([0.0, 0.0, 0.0]);

        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let yaw_a = match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D {
                plane: QuadPlane::HorizontalYaw(y),
                ..
            } => *y,
            _ => panic!("expected Texture3D::HorizontalYaw"),
        };
        assert_eq!(yaw_a, 0.0);

        for _ in 0..10 {
            assert_eq!(
                e.update(&EffectUpdateCtx {
                    delta: 1.0 / FRAMES_PER_SECOND,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let yaw_b = match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D {
                plane: QuadPlane::HorizontalYaw(y),
                ..
            } => *y,
            _ => panic!("expected Texture3D::VerticalYaw"),
        };
        assert!(yaw_b > yaw_a, "yaw rotates over time: {yaw_a} -> {yaw_b}");
    }
}
