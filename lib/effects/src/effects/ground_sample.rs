use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "magic_target.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

pub const TOTAL_DURATION_MS: u32 = 30_000;
const TOTAL_DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;

const HALF_SIZE: f32 = 17.5;
const ALPHA: f32 = 128.0 / 255.0;
const FRAMES_PER_SECOND: f32 = 60.0;
const ROTATION_DEG_PER_FRAME: f32 = 1.0;

pub struct GroundSampleEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl GroundSampleEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }
}

impl Effect for GroundSampleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        let theta = (frame * ROTATION_DEG_PER_FRAME).to_radians();
        let (s, c) = theta.sin_cos();
        let [cx, cy, cz] = self.world_pos;
        let rotate =
            |dx: f32, dz: f32| -> [f32; 3] { [cx + dx * c - dz * s, cy, cz + dx * s + dz * c] };
        let corners = [
            rotate(-HALF_SIZE, -HALF_SIZE),
            rotate(HALF_SIZE, -HALF_SIZE),
            rotate(HALF_SIZE, HALF_SIZE),
            rotate(-HALF_SIZE, HALF_SIZE),
        ];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners,
            uv,
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, ALPHA],
            blend: BlendKind::Alpha,
            no_depth: false,
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

    fn draws(effect: &GroundSampleEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut GroundSampleEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn emits_one_world_quad_lying_flat_on_xz_at_caster_y() {
        let cy = 0.5;
        let mut eff = GroundSampleEffect::new([5.0, cy, -2.0]);
        step(&mut eff, 0.0);
        let prims = draws(&eff);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::WorldQuad {
                corners,
                uv,
                color,
                texture,
                blend,
                ..
            } => {
                assert_eq!(*texture, TEXTURE);
                assert_eq!(*blend, BlendKind::Alpha);
                assert!((color[3] - ALPHA).abs() < 1e-6, "alpha is 0x80/0xFF");
                assert_eq!(*uv, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
                for corner in corners {
                    assert!(
                        (corner[1] - cy).abs() < 1e-6,
                        "all corners share the caster's Y"
                    );
                }
            }
            _ => panic!("expected WorldQuad"),
        }
    }

    #[test]
    fn quad_corners_rotate_around_center_each_frame() {
        let mut eff = GroundSampleEffect::new([0.0; 3]);
        step(&mut eff, 0.0);
        let c0 = match &draws(&eff)[0] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => *corners,
            _ => unreachable!(),
        };
        step(&mut eff, 1.0);
        let c1 = match &draws(&eff)[0] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => *corners,
            _ => unreachable!(),
        };
        assert_ne!(c0[0], c1[0], "first corner rotates around the center");
        let dist = |p: [f32; 3]| (p[0] * p[0] + p[2] * p[2]).sqrt();
        for (a, b) in c0.iter().zip(c1.iter()) {
            assert!(
                (dist(*a) - dist(*b)).abs() < 1e-4,
                "corners stay on the same ring"
            );
        }
    }

    #[test]
    fn dies_after_total_duration() {
        let mut eff = GroundSampleEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_S * 1.2 {
            status = step(&mut eff, 1.0 / 60.0);
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
