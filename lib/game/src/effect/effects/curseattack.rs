//! `EF_CURSEATTACK` — curse mark hovering over the cursed entity (id 194).
//!
//! Original game spawns a persistent (clipped to 1500 ms by the table) curse.bmp
//! quad, yawing 4.5°/frame around the world Y axis. It is a
//! vertical quad (no roll), anchored at the actor and lifted above
//! the head by 30 units. Native RO coordinates are `-Y = up`, so
//! that lift is `world_pos.y - 30` here (1:1 with the original, as for
//! DarkBreath's `-20`). The grim-reaper texture is vertically asymmetric, so the
//! UV must put the texture top (`v = 0`) at the quad's top vertex — matching the
//! observed orientation — or the reaper renders upside down.

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const CURSE_TEXTURE: &str = "curse.bmp";
pub const TEXTURES: &[&str] = &[CURSE_TEXTURE];

const HALF_SIZE: f32 = 4.0;
const HEAD_OFFSET: f32 = 25.0;
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
            // VerticalYaw corners are [bottom, bottom, top, top]; the reaper
            // texture's top is v=0, so bottoms take v=1 and tops v=0.
            uv: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
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
            EffectPrimitiveDraw::Texture3D {
                center, uv, plane, ..
            } => {
                assert!(
                    center[1] <= -HEAD_OFFSET,
                    "mark is lifted above the head (native RO -Y up)"
                );
                assert!(matches!(plane, QuadPlane::VerticalYaw(_)), "vertical quad");
                // VerticalYaw corners are [bottom, bottom, top, top]; an upright
                // reaper needs the texture top (v=0) on the top vertices.
                assert!(
                    uv[0][1] > uv[2][1] && uv[1][1] > uv[3][1],
                    "texture is upright, not flipped vertically"
                );
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
