//! Bottom_LandProtector family — a horizontal square ward above the actor's feet.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
const GROUND_Y_OFFSET: f32 = -2.0;

#[derive(Clone, Copy, Debug)]
pub struct BottomLandProtectorParams {
    pub texture: &'static str,
    pub distance: f32,
    pub rot_start_deg: f32,
    pub alpha_b: f32,
    pub tint_rgb: [f32; 3],
    pub rise_speed_deg_per_frame: f32,
}

pub const LA: BottomLandProtectorParams = BottomLandProtectorParams {
    texture: "aaa copy.bmp",
    distance: 7.0,
    rot_start_deg: 0.0,
    alpha_b: 100.0 / 255.0,
    tint_rgb: [1.0, 1.0, 1.0],
    rise_speed_deg_per_frame: 3.0,
};

pub const RUNNER: BottomLandProtectorParams = BottomLandProtectorParams {
    texture: "hanmoon1.tga",
    distance: 10.0,
    rot_start_deg: 225.0,
    alpha_b: 250.0 / 255.0,
    tint_rgb: [55.0 / 255.0, 55.0 / 255.0, 1.0],
    rise_speed_deg_per_frame: 3.0,
};

pub const TRANSFER: BottomLandProtectorParams = BottomLandProtectorParams {
    texture: "hanmoon2.tga",
    distance: 10.0,
    rot_start_deg: 225.0,
    alpha_b: 250.0 / 255.0,
    tint_rgb: [55.0 / 255.0, 55.0 / 255.0, 1.0],
    rise_speed_deg_per_frame: 3.0,
};

pub const SPIDER: BottomLandProtectorParams = BottomLandProtectorParams {
    texture: "spiderweb.tga",
    distance: 12.0,
    rot_start_deg: 0.0,
    alpha_b: 100.0 / 255.0,
    tint_rgb: [1.0, 1.0, 1.0],
    rise_speed_deg_per_frame: 1.0,
};

pub const TEXTURES: &[&str] = &[
    "aaa copy.bmp",
    "hanmoon1.tga",
    "hanmoon2.tga",
    "spiderweb.tga",
];

pub struct BottomLandProtectorEffect {
    world_pos: [f32; 3],
    params: BottomLandProtectorParams,
    age: f32,
    frames: u32,
    rise_angle_init: f32,
}

impl BottomLandProtectorEffect {
    pub fn new(world_pos: [f32; 3], params: BottomLandProtectorParams) -> Self {
        let seed = position_hash(&world_pos);
        Self {
            world_pos,
            params,
            age: 0.0,
            frames: 0,
            rise_angle_init: rand_in_range(seed, 1, 0.0, 360.0),
        }
    }
}

impl Effect for BottomLandProtectorEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let f = self.frames as f32;
        let rise_angle = (self.rise_angle_init + f * self.params.rise_speed_deg_per_frame) % 360.0;
        let rx = self.params.distance * 0.1 * (rise_angle.to_radians().sin() + 1.0);
        let r = self.params.distance * 0.8 + rx;

        let y = self.world_pos[1] + GROUND_Y_OFFSET;
        let mut corners = [[0.0_f32; 3]; 4];
        for (i, c) in corners.iter_mut().enumerate() {
            let angle_deg = (self.params.rot_start_deg + i as f32 * 90.0) % 360.0;
            let (s, cs) = angle_deg.to_radians().sin_cos();
            *c = [self.world_pos[0] + cs * r, y, self.world_pos[2] + s * r];
        }

        let [tr, tg, tb] = self.params.tint_rgb;
        let uv = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners,
            uv,
            texture: self.params.texture,
            color: [tr, tg, tb, self.params.alpha_b],
            blend: BlendKind::Additive,
            no_depth: false,
        });
    }
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
}

fn rand_in_range(seed: u64, salt: u64, lo: f32, hi: f32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(salt);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let t = ((x >> 40) as f32) / ((1u64 << 24) as f32);
    lo + t * (hi - lo)
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

    fn step(effect: &mut BottomLandProtectorEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomLandProtectorEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn la_emits_one_horizontal_square_above_master_feet() {
        let mut e = BottomLandProtectorEffect::new([12.0, 5.0, 34.0], LA);
        step(&mut e, 0.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 1);

        let EffectPrimitiveDraw::WorldQuad {
            corners,
            blend,
            color,
            texture,
            ..
        } = &prims[0]
        else {
            panic!("expected WorldQuad");
        };
        assert_eq!(*blend, BlendKind::Additive);
        assert_eq!(*texture, "aaa copy.bmp");
        let y0 = corners[0][1];
        for c in corners {
            assert!((c[1] - y0).abs() < 1e-4, "corners not horizontal");
        }
        assert!((y0 - 3.0).abs() < 1e-4);
        let cx: f32 = corners.iter().map(|c| c[0]).sum::<f32>() / 4.0;
        let cz: f32 = corners.iter().map(|c| c[2]).sum::<f32>() / 4.0;
        assert!((cx - 12.0).abs() < 1e-3);
        assert!((cz - 34.0).abs() < 1e-3);
        assert!((color[0] - 1.0).abs() < 1e-3);
        assert!((color[1] - 1.0).abs() < 1e-3);
        assert!((color[2] - 1.0).abs() < 1e-3);
        assert!((color[3] - 100.0 / 255.0).abs() < 1e-3);
    }

    #[test]
    fn runner_uses_blue_tint_and_high_alpha_at_225_offset() {
        let mut e = BottomLandProtectorEffect::new([0.0, 0.0, 0.0], RUNNER);
        step(&mut e, 0.0);
        let prims = draws(&e);
        let EffectPrimitiveDraw::WorldQuad { corners, color, .. } = &prims[0] else {
            panic!();
        };
        assert!(color[2] > color[0] && color[2] > color[1]);
        assert!((color[3] - 250.0 / 255.0).abs() < 1e-3);
        assert!(corners[0][0] < 0.0, "corner 0 should be in -X quadrant");
        assert!(corners[0][2] < 0.0, "corner 0 should be in -Z quadrant");
    }

    #[test]
    fn spider_corners_breathe_radially_over_time() {
        let pos = [100.0, 0.0, 100.0];
        let mut e = BottomLandProtectorEffect::new(pos, SPIDER);
        step(&mut e, 0.0);
        let r_a = corner_radius(&draws(&e)[0], pos);
        step(&mut e, 1.5);
        let r_b = corner_radius(&draws(&e)[0], pos);
        assert!(
            (r_a - r_b).abs() > 0.2,
            "expected breathing radius to differ; r_a={r_a}, r_b={r_b}"
        );
        for r in [r_a, r_b] {
            assert!(r >= 9.6 - 1e-3 && r <= 12.0 + 1e-3, "r out of band: {r}");
        }
    }

    fn corner_radius(prim: &EffectPrimitiveDraw, master: [f32; 3]) -> f32 {
        let EffectPrimitiveDraw::WorldQuad { corners, .. } = prim else {
            panic!();
        };
        let dx = corners[0][0] - master[0];
        let dz = corners[0][2] - master[2];
        (dx * dx + dz * dz).sqrt()
    }
}
