//! Bottom_Light family — a 315° rotating curtain of light above the actor.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const E_DIVISION: usize = 21;
const FULL_DISPLAY_ANGLE_DEG: f32 = 315.0;
const BASIC_ANGLE_DEG: f32 = FULL_DISPLAY_ANGLE_DEG / (E_DIVISION as f32 - 1.0);

#[derive(Clone, Copy, Debug)]
pub struct BottomLightParams {
    pub texture: &'static str,
    pub tint_rgb: [f32; 3],
}

pub const ETERNALCHAOS: BottomLightParams = BottomLightParams {
    texture: "twirl.bmp",
    tint_rgb: [1.0, 1.0, 170.0 / 255.0],
};

pub const SIEGFRIED: BottomLightParams = BottomLightParams {
    texture: "twirl.bmp",
    tint_rgb: [100.0 / 255.0, 100.0 / 255.0, 1.0],
};

pub const TEXTURES: &[&str] = &["twirl.bmp"];

pub struct BottomLightEffect {
    world_pos: [f32; 3],
    params: BottomLightParams,
    age: f32,
    frames: u32,
    max_height: f32,
    rot_start_init: f32,
    alpha_t_init: f32,
    alpha_b: f32,
}

impl BottomLightEffect {
    pub fn new(world_pos: [f32; 3], params: BottomLightParams) -> Self {
        let seed = position_hash(&world_pos);
        Self {
            world_pos,
            params,
            age: 0.0,
            frames: 0,
            max_height: 25.0 + rand_in_range(seed, 1, 0.0, 10.0),
            rot_start_init: rand_in_range(seed, 2, 0.0, 360.0),
            alpha_t_init: rand_in_range(seed, 3, 0.0, 360.0),
            alpha_b: 50.0 - rand_in_range(seed, 4, 0.0, 10.0),
        }
    }

    fn frame_state(&self) -> FrameState {
        let f = self.frames as f32;
        let alpha_t = (self.alpha_t_init + f) % 360.0;
        let rot_start = (self.rot_start_init + f * 3.0) % 360.0;
        let pr = f % 360.0;
        FrameState {
            distance: 3.0 + alpha_t.to_radians().sin(),
            rot_start,
            pr,
        }
    }
}

struct FrameState {
    distance: f32,
    rot_start: f32,
    pr: f32,
}

impl Effect for BottomLightEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let st = self.frame_state();
        let alpha = self.alpha_b / 255.0;
        let [tr, tg, tb] = self.params.tint_rgb;

        let middle = (E_DIVISION as f32 - 1.0) * 0.5;
        let m2 = 90.0 / middle;
        let pr_mod = 1.0 + 0.3 * st.pr.to_radians().sin();
        let mut heights = [0.0f32; E_DIVISION];
        for i in 0..E_DIVISION {
            let sin_limit_deg = 90.0 + (i as f32 - middle) * m2;
            heights[i] = self.max_height * sin_limit_deg.to_radians().sin() * pr_mod;
        }

        let base_y = self.world_pos[1];
        let mut points = [([0.0_f32; 3], [0.0_f32; 3]); E_DIVISION];
        for order in 0..E_DIVISION {
            let angle_deg = (order as f32 * BASIC_ANGLE_DEG + st.rot_start) % 360.0;
            let (s, c) = angle_deg.to_radians().sin_cos();
            let bx = self.world_pos[0] + c * st.distance;
            let bz = self.world_pos[2] + s * st.distance;
            let base = [bx, base_y, bz];
            let top = [bx, base_y - heights[order], bz];
            points[order] = (base, top);
        }

        for order in 1..E_DIVISION {
            let (prev_base, prev_top) = points[order - 1];
            let (cur_base, cur_top) = points[order];
            let corners = [cur_base, prev_base, prev_top, cur_top];
            let u_pre = (order as f32 - 1.0) / E_DIVISION as f32;
            let u_now = order as f32 / E_DIVISION as f32;
            let uv = [[u_now, 1.0], [u_pre, 1.0], [u_pre, 0.0], [u_now, 0.0]];
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners,
                uv,
                texture: self.params.texture,
                color: [tr, tg, tb, alpha],
                blend: BlendKind::Additive,
                no_depth: false,
            });
        }
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

    fn step(effect: &mut BottomLightEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomLightEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn eternalchaos_emits_twenty_additive_segments_arched_around_master() {
        let mut e = BottomLightEffect::new([50.0, 0.0, 20.0], ETERNALCHAOS);
        step(&mut e, 0.5);
        let prims = draws(&e);
        assert_eq!(prims.len(), 20);

        let mut peak_drop = 0.0_f32;
        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad {
                corners,
                blend,
                color,
                texture,
                ..
            } = p
            else {
                panic!("expected WorldQuad, got {p:?}");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert_eq!(*texture, "twirl.bmp");
            assert!(color[2] < color[0]);
            assert!(color[2] < color[1]);
            let drop_now = corners[0][1] - corners[3][1];
            let drop_pre = corners[1][1] - corners[2][1];
            assert!(drop_now >= -1e-3 && drop_pre >= -1e-3);
            peak_drop = peak_drop.max(drop_now).max(drop_pre);
        }
        assert!(
            peak_drop > 15.0,
            "sine arch should peak near max_height (25..35); got {peak_drop}"
        );
    }

    #[test]
    fn siegfried_uses_light_blue_additive_tint() {
        let mut e = BottomLightEffect::new([0.0, 0.0, 0.0], SIEGFRIED);
        step(&mut e, 0.5);
        let prims = draws(&e);
        assert!(!prims.is_empty());
        let EffectPrimitiveDraw::WorldQuad { color, blend, .. } = &prims[0] else {
            panic!("expected WorldQuad");
        };
        assert_eq!(*blend, BlendKind::Additive);
        assert!(color[2] > color[0]);
        assert!(color[2] > color[1]);
    }

    #[test]
    fn cone_rotates_over_time() {
        let mut e = BottomLightEffect::new([0.0, 0.0, 0.0], ETERNALCHAOS);
        step(&mut e, 0.5);
        let p_a = match &draws(&e)[0] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => corners[0],
            _ => panic!(),
        };
        step(&mut e, 1.0);
        let p_b = match &draws(&e)[0] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => corners[0],
            _ => panic!(),
        };
        let d = (p_a[0] - p_b[0]).hypot(p_a[2] - p_b[2]);
        assert!(
            d > 0.5,
            "expected rotation to displace the first base point, got d={d}"
        );
    }
}
