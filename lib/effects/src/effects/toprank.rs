//! `EF_TOPRANK` — top-ranker indicator (id 159).
//!
//! A persistent
//! `LockOn128.tga` quad over the actor laid flat on the ground
//! plane, spinning 4.5°/frame around the world Y
//! axis. Tint is derived from the PvP rank carried in the spawn count.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TOPRANK_TEXTURE: &str = "LockOn128.tga";
pub const TEXTURES: &[&str] = &[TOPRANK_TEXTURE];

const HALF_SIZE: f32 = 4.0;
const DEG_PER_FRAME: f32 = 4.5;
const FRAMES_PER_SECOND: f32 = 60.0;
const GROUND_Y_OFFSET: f32 = -2.0;

pub fn rank_tint(rank: u8) -> [f32; 4] {
    if rank == 11 {
        [0.0, 250.0 / 255.0, 0.0, 1.0]
    } else {
        let fade = f32::from(10u8.saturating_sub(rank)) * 25.0 / 255.0;
        [250.0 / 255.0, fade, fade, 1.0]
    }
}

pub struct ToprankEffect {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
}

impl ToprankEffect {
    pub fn new(world_pos: [f32; 3], rank: u8) -> Self {
        Self {
            world_pos,
            tint: rank_tint(rank),
            age: 0.0,
        }
    }
}

impl Effect for ToprankEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frames = self.age * FRAMES_PER_SECOND;
        let yaw = (frames * DEG_PER_FRAME).to_radians();
        let center = [
            self.world_pos[0],
            self.world_pos[1] + GROUND_Y_OFFSET,
            self.world_pos[2],
        ];
        out.push(EffectPrimitiveDraw::Texture3D {
            center,
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

    fn quad(e: &ToprankEffect) -> ([f32; 3], f32, [f32; 4]) {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D {
                center,
                plane: QuadPlane::HorizontalYaw(yaw),
                color,
                ..
            } => (*center, *yaw, *color),
            _ => panic!("expected Texture3D::HorizontalYaw"),
        }
    }

    #[test]
    fn yaw_advances_per_frame_and_stays_alive() {
        let mut e = ToprankEffect::new([0.0, 0.0, 0.0], 1);
        let (_, yaw_a, _) = quad(&e);
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
        let (_, yaw_b, _) = quad(&e);
        assert!(yaw_b > yaw_a, "yaw rotates over time: {yaw_a} -> {yaw_b}");
    }

    #[test]
    fn tint_fades_with_rank_and_quad_follows_the_actor() {
        let (_, _, rank1) = quad(&ToprankEffect::new([0.0, 0.0, 0.0], 1));
        let (_, _, rank10) = quad(&ToprankEffect::new([0.0, 0.0, 0.0], 10));
        let (_, _, rank11) = quad(&ToprankEffect::new([0.0, 0.0, 0.0], 11));
        assert_eq!(rank1, [250.0 / 255.0, 225.0 / 255.0, 225.0 / 255.0, 1.0]);
        assert_eq!(rank10, [250.0 / 255.0, 0.0, 0.0, 1.0]);
        assert_eq!(rank11, [0.0, 250.0 / 255.0, 0.0, 1.0]);

        let mut e = ToprankEffect::new([1.0, 2.0, 3.0], 1);
        e.set_position([10.0, 20.0, 30.0]);
        let (center, _, _) = quad(&e);
        assert_eq!(center, [10.0, 20.0 + GROUND_Y_OFFSET, 30.0]);
    }
}
