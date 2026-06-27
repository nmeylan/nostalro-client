//! `EF_PARTY` — persistent party-member marker (id 160).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const PARTY_TEXTURE: &str = "party.tga";
pub const TEXTURES: &[&str] = &[PARTY_TEXTURE];

const HALF_SIZE: f32 = 4.0;
const TINT: [f32; 4] = [50.0 / 255.0, 50.0 / 255.0, 253.0 / 255.0, 1.0];

pub struct PartyEffect {
    world_pos: [f32; 3],
}

impl PartyEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos }
    }
}

impl Effect for PartyEffect {
    fn update(&mut self, _ctx: &EffectUpdateCtx) -> EffectStatus {
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        out.push(EffectPrimitiveDraw::Texture3D {
            center: self.world_pos,
            size: [HALF_SIZE, HALF_SIZE],
            plane: QuadPlane::Horizontal,
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            texture: PARTY_TEXTURE,
            color: TINT,
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
    fn emits_one_horizontal_quad_at_anchor_and_stays_alive() {
        let mut e = PartyEffect::new([3.0, 0.5, 7.0]);
        let status = e.update(&EffectUpdateCtx {
            delta: 1.0 / 60.0,
            camera_target: None,
            caster_yaw: None,
        });
        assert_eq!(status, EffectStatus::Running);

        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1);
        match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D {
                center,
                plane,
                texture,
                ..
            } => {
                assert_eq!(*center, [3.0, 0.5, 7.0]);
                assert_eq!(*plane, QuadPlane::Horizontal);
                assert_eq!(*texture, PARTY_TEXTURE);
            }
            _ => panic!("expected Texture3D"),
        }

        for _ in 0..600 {
            assert_eq!(
                e.update(&EffectUpdateCtx {
                    delta: 1.0 / 60.0,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
    }
}
