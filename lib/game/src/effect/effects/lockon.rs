//! `EF_LOCKON` — lock-on targeting reticle (id 60).
//!
//! In the original game this spawns **two** crossed
//! `lockon128.tga` quads laid flat on the ground at the targeted
//! entity's feet. The second quad is offset 45° in-plane from the first, so the
//! pair reads as an 8-point targeting star. Both spin around world Y
//! (~4.5°/frame), shrink over their life, and pulse a
//! red tint (red holds at 250, green/blue cycle down from 150).
//!
//! (The single `magic_target.tga` quad variant is the *ground-cast*
//! reticle, not the entity lock-on; we render the two-quad reticle.)

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const LOCKON_TEXTURE: &str = "lockon128.tga";
pub const TEXTURES: &[&str] = &[LOCKON_TEXTURE];

const HALF_SIZE_START: f32 = 5.0;
const HALF_SIZE_END: f32 = 2.0;
const DEG_PER_FRAME: f32 = 4.5;
const FRAMES_PER_SECOND: f32 = 60.0;
const LIFETIME: f32 = 3.333;
const QUAD_OFFSET: f32 = std::f32::consts::FRAC_PI_4;
const PULSE_PERIOD: f32 = 20.0 / FRAMES_PER_SECOND;

pub struct LockonEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl LockonEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn half_size(&self) -> f32 {
        let t = (self.age / LIFETIME).clamp(0.0, 1.0);
        HALF_SIZE_START + (HALF_SIZE_END - HALF_SIZE_START) * t
    }

    fn color(&self) -> [f32; 4] {
        let pulse = (self.age / PULSE_PERIOD * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let gb = 0.3 + 0.3 * pulse;
        [1.0, gb, gb, 1.0]
    }
}

impl Effect for LockonEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frames = self.age * FRAMES_PER_SECOND;
        let yaw = (frames * DEG_PER_FRAME).to_radians();
        let half = self.half_size();
        let color = self.color();
        for offset in [0.0, QUAD_OFFSET] {
            out.push(EffectPrimitiveDraw::Texture3D {
                center: self.world_pos,
                size: [half, half],
                plane: QuadPlane::HorizontalYaw(yaw + offset),
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                texture: LOCKON_TEXTURE,
                color,
                blend: BlendKind::Additive,
            });
        }
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

    fn yaws_and_size(e: &LockonEffect) -> ([f32; 2], f32) {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 2, "lock-on emits two crossed quads");
        let mut yaws = [0.0; 2];
        let mut size = 0.0;
        for (i, prim) in list.primitives.iter().enumerate() {
            match prim {
                EffectPrimitiveDraw::Texture3D {
                    plane: QuadPlane::HorizontalYaw(y),
                    size: s,
                    ..
                } => {
                    yaws[i] = *y;
                    size = s[0];
                }
                _ => panic!("expected Texture3D::HorizontalYaw"),
            }
        }
        (yaws, size)
    }

    #[test]
    fn two_crossed_quads_spin_and_shrink() {
        let mut e = LockonEffect::new([0.0, 0.0, 0.0]);

        let (yaws_a, size_a) = yaws_and_size(&e);
        assert!(
            (yaws_a[1] - yaws_a[0] - QUAD_OFFSET).abs() < 1e-5,
            "second quad is offset 45deg in-plane"
        );

        for _ in 0..60 {
            assert_eq!(
                e.update(&EffectUpdateCtx {
                    delta: 1.0 / FRAMES_PER_SECOND,
                    camera_target: None,
                    caster_yaw: None,
                }),
                EffectStatus::Running
            );
        }

        let (yaws_b, size_b) = yaws_and_size(&e);
        assert!(yaws_b[0] > yaws_a[0], "reticle spins over time");
        assert!(size_b < size_a, "reticle shrinks over time");
    }
}
