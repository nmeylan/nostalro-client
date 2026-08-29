use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const NUM_QUADS: usize = 2;
/// 23° offset interleaves the two pikapika2 stars into a 16-ray burst; 45° would make them coincide.
const QUAD_ROT_OFFSET: f32 = 23.0 * std::f32::consts::PI / 180.0;
const PULSE_SPEED_RAD_PER_S: f32 = 3.0 * std::f32::consts::PI / 180.0 * FRAMES_PER_SECOND;
const PULSE_MID: f32 = 0.9;
const PULSE_HALF: f32 = 0.1;
/// Negative Y = up; lift off ground to avoid z-fighting.
const GROUND_LIFT: f32 = -1.0;

#[derive(Clone, Copy, Debug)]
pub struct FloorAuraParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub radius: f32,
    pub alpha_max: f32,
}

pub const LV99_BLUE: FloorAuraParams = FloorAuraParams {
    texture: "pikapika2.bmp",
    color_rgb: [1.00, 1.00, 1.00],
    radius: 15.0,
    alpha_max: 200.0 / 255.0,
};

pub const LV99_GREEN: FloorAuraParams = FloorAuraParams {
    texture: "pikapika2.bmp",
    color_rgb: [0.14, 1.00, 0.14],
    radius: 15.0,
    alpha_max: 200.0 / 255.0,
};

pub const MAP_PIKA: FloorAuraParams = FloorAuraParams {
    texture: "pikapika2.bmp",
    color_rgb: [0.39, 0.39, 1.00],
    radius: 46.0,
    alpha_max: 25.0 / 255.0,
};

pub const TEXTURES: &[&str] = &["pikapika2.bmp"];

pub struct FloorAuraEffect {
    params: FloorAuraParams,
    world_pos: [f32; 3],
    age: f32,
}

impl FloorAuraEffect {
    pub fn new(world_pos: [f32; 3], params: FloorAuraParams) -> Self {
        Self {
            params,
            world_pos,
            age: 0.0,
        }
    }
}

impl Effect for FloorAuraEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let alpha = self.params.alpha_max;
        let y = self.world_pos[1] + GROUND_LIFT;
        for i in 0..NUM_QUADS {
            let phase = self.age * PULSE_SPEED_RAD_PER_S + i as f32 * std::f32::consts::PI;
            let radius = self.params.radius * (PULSE_MID + PULSE_HALF * phase.sin());
            let rot = i as f32 * QUAD_ROT_OFFSET;
            let mut corners = [[0.0f32; 3]; 4];
            for (k, corner) in corners.iter_mut().enumerate() {
                let a = rot + k as f32 * std::f32::consts::FRAC_PI_2;
                *corner = [
                    self.world_pos[0] + a.cos() * radius,
                    y,
                    self.world_pos[2] + a.sin() * radius,
                ];
            }
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners,
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                texture: self.params.texture,
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
                no_depth: false,
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

    fn quads(c: &FloorAuraEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn run_to(c: &mut FloorAuraEffect, frame: f32) {
        let delta = (frame - c.age * FRAMES_PER_SECOND) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx {
                delta,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn radius_and_flat(p: &EffectPrimitiveDraw, center: [f32; 3]) -> (f32, bool) {
        let EffectPrimitiveDraw::WorldQuad { corners, .. } = p else {
            panic!("expected WorldQuad")
        };
        let y0 = corners[0][1];
        let flat = corners.iter().all(|c| (c[1] - y0).abs() < 1e-4);
        let r = ((corners[0][0] - center[0]).powi(2) + (corners[0][2] - center[2]).powi(2)).sqrt();
        (r, flat)
    }

    #[test]
    fn emits_two_flat_ground_quads() {
        let center = [4.0, 1.0, 6.0];
        let c = FloorAuraEffect::new(center, LV99_BLUE);
        let prims = quads(&c);
        assert_eq!(prims.len(), NUM_QUADS);
        for p in &prims {
            let (_, flat) = radius_and_flat(p, center);
            assert!(
                flat,
                "floor aura quad must lie on a single horizontal plane"
            );
            let EffectPrimitiveDraw::WorldQuad { blend, .. } = p else {
                panic!()
            };
            assert_eq!(*blend, BlendKind::Additive);
        }
    }

    #[test]
    fn quads_pulse_out_of_phase() {
        let center = [0.0, 0.0, 0.0];
        let mut c = FloorAuraEffect::new(center, LV99_BLUE);
        let quarter = (std::f32::consts::FRAC_PI_2 / PULSE_SPEED_RAD_PER_S) * FRAMES_PER_SECOND;
        run_to(&mut c, quarter);
        let prims = quads(&c);
        let (r0, _) = radius_and_flat(&prims[0], center);
        let (r1, _) = radius_and_flat(&prims[1], center);
        assert!(
            (r0 - r1).abs() > 1e-3,
            "quads expand/shrink out of phase ({r0} vs {r1})"
        );
    }

    #[test]
    fn variants_have_distinct_tints() {
        assert_ne!(LV99_BLUE.color_rgb, LV99_GREEN.color_rgb);
        for p in [LV99_BLUE, LV99_GREEN] {
            assert!(TEXTURES.contains(&p.texture));
        }
    }

    #[test]
    fn never_self_terminates() {
        let mut c = FloorAuraEffect::new([0.0; 3], LV99_GREEN);
        for _ in 0..200 {
            assert_eq!(
                c.update(&EffectUpdateCtx {
                    delta: 0.1,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
    }
}
