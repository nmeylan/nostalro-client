use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "magic_red.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 99_990;

const WORLD_SCALE: f32 = 1.0;
const SIDES: u32 = 24;
const SPIN_DEG_PER_FRAME: f32 = 3.0;
const UV_RISE_PER_FRAME: f32 = 0.0;

const FLAME_TINT: [f32; 3] = [1.0, 0.72, 0.68];

const CONES: [(f32, f32, f32, f32); 3] = [
    (2.5, 4.0, 27.5, 240.0 / 255.0),
    (3.5, 5.0, 15.5, 233.0 / 255.0),
    (4.5, 6.0, 7.5, 226.0 / 255.0),
];

pub struct FirePillarOnEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl FirePillarOnEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }
}

impl Effect for FirePillarOnEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let rotation = (self.age_frames * SPIN_DEG_PER_FRAME).to_radians();
        let scroll_v = self.age_frames * UV_RISE_PER_FRAME;
        for (bottom, top, height, alpha) in CONES {
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: bottom * WORLD_SCALE,
                top_size: top * WORLD_SCALE,
                height: height * WORLD_SCALE,
                sides: SIDES,
                rotation,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, scroll_v],
                texture: TEXTURE,
                color: [FLAME_TINT[0], FLAME_TINT[1], FLAME_TINT[2], alpha],
                alpha_bottom: alpha,
                blend: BlendKind::Additive,
            });
        }
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
    fn emits_three_nested_cones() {
        let mut e = FirePillarOnEffect::new([0.0; 3]);
        e.update(&ctx(0.1));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let cones: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { texture, .. } if *texture == TEXTURE))
            .collect();
        assert_eq!(cones.len(), 3);
        let heights: Vec<f32> = cones
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Cylinder { height, .. } => *height,
                _ => unreachable!(),
            })
            .collect();
        assert!(heights[0] > heights[1] && heights[1] > heights[2]);
    }

    #[test]
    fn spins_over_time_and_stays_alive() {
        let mut e = FirePillarOnEffect::new([0.0; 3]);
        assert_eq!(e.update(&ctx(0.5)), EffectStatus::Running);
        let mut a = EffectDrawList::new();
        e.collect_draws(&mut a, &render_ctx());
        e.update(&ctx(0.5));
        let mut b = EffectDrawList::new();
        e.collect_draws(&mut b, &render_ctx());
        let rot = |l: &EffectDrawList| match l.primitives[0] {
            EffectPrimitiveDraw::Cylinder { rotation, .. } => rotation,
            _ => unreachable!(),
        };
        assert!((rot(&a) - rot(&b)).abs() > 1e-4, "seam spins over time");
        assert_eq!(e.update(&ctx(60.0)), EffectStatus::Running);
    }
}
