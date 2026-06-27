use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const SQRT2: f32 = std::f32::consts::SQRT_2;

const CLOUD_TEX: [&str; 3] = ["cloud4.tga", "cloud1.tga", "cloud2.tga"];
const FOG_TEX: [&str; 3] = ["fog1.tga", "fog2.tga", "fog3.tga"];

pub const TEXTURES: &[&str] = &[
    "cloud4.tga",
    "cloud1.tga",
    "cloud2.tga",
    "fog1.tga",
    "fog2.tga",
    "fog3.tga",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drift {
    Isotropic(f32),
    Airplane,
}

#[derive(Clone, Copy, Debug)]
pub struct CloudParams {
    pub textures: [&'static str; 3],
    pub tint: [f32; 3],
    pub elevation: f32,
    pub use_ground: bool,
    pub centered: bool,
    pub size_base: f32,
    pub size_rand: f32,
    pub drift: Drift,
    pub alpha_rate: f32,
    pub ramp_frames: f32,
    pub count: u32,
}

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

pub const CLOUD: CloudParams = CloudParams {
    textures: CLOUD_TEX,
    tint: WHITE,
    elevation: -125.0,
    use_ground: false,
    centered: true,
    size_base: 30.0,
    size_rand: 20.0,
    drift: Drift::Isotropic(0.05),
    alpha_rate: 2.0,
    ramp_frames: 80.0,
    count: 160,
};
pub const CLOUD2: CloudParams = CloudParams {
    elevation: 40.0,
    centered: false,
    alpha_rate: 3.0,
    count: 240,
    ..CLOUD
};
pub const CLOUD3: CloudParams = CloudParams {
    elevation: 0.0,
    ..CLOUD
};
pub const CLOUD4: CloudParams = CloudParams {
    textures: FOG_TEX,
    tint: [252.0 / 255.0, 171.0 / 255.0, 143.0 / 255.0],
    elevation: -20.0,
    use_ground: true,
    size_base: 35.0,
    size_rand: 10.0,
    drift: Drift::Isotropic(0.015),
    alpha_rate: 1.0,
    ramp_frames: 170.0,
    count: 320,
    ..CLOUD
};
pub const CLOUD5: CloudParams = CloudParams {
    elevation: 40.0,
    centered: false,
    drift: Drift::Airplane,
    alpha_rate: 3.0,
    count: 320,
    ..CLOUD
};
pub const CLOUD6: CloudParams = CloudParams {
    tint: [94.0 / 255.0, 0.0, 0.0],
    elevation: 20.0,
    drift: Drift::Isotropic(0.035),
    count: 320,
    ..CLOUD
};
pub const CLOUD7: CloudParams = CloudParams {
    tint: [0.0, 0.0, 0.0],
    elevation: 40.0,
    centered: false,
    alpha_rate: 3.0,
    count: 320,
    ..CLOUD
};
pub const CLOUD8: CloudParams = CloudParams {
    tint: [1.0, 180.0 / 255.0, 180.0 / 255.0],
    elevation: 40.0,
    centered: false,
    alpha_rate: 3.0,
    count: 320,
    ..CLOUD
};

fn cloud_alpha(p: &CloudParams, process: f32, rot_start: f32) -> f32 {
    let peak = p.alpha_rate * p.ramp_frames;
    if process < p.ramp_frames {
        p.alpha_rate * process
    } else if process <= rot_start {
        peak
    } else {
        (peak - (process - rot_start)).max(0.0)
    }
}

fn hash01(i: u32, salt: u32) -> f32 {
    let x = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

#[derive(Clone, Copy)]
struct Cloud {
    pos: [f32; 3],
    distance: f32,
    drift_phase: [f32; 2],
    drift_rate: [f32; 2],
    breath_phase: f32,
    process: f32,
    rot_start: f32,
    alpha: f32,
    generation: u32,
}

pub struct CloudEffect {
    world_pos: [f32; 3],
    params: CloudParams,
    clouds: Vec<Cloud>,
}

impl CloudEffect {
    pub fn new(world_pos: [f32; 3], params: CloudParams) -> Self {
        let clouds = (0..params.count)
            .map(|i| spawn_cloud(i, 0, &params, world_pos))
            .collect();
        Self {
            world_pos,
            params,
            clouds,
        }
    }

    fn step(&mut self, df: f32) {
        let peak = self.params.alpha_rate * self.params.ramp_frames;
        for (i, c) in self.clouds.iter_mut().enumerate() {
            c.process += df;
            if c.process >= c.rot_start + peak {
                *c = spawn_cloud(i as u32, c.generation + 1, &self.params, self.world_pos);
                continue;
            }
            c.alpha = cloud_alpha(&self.params, c.process, c.rot_start);
            match self.params.drift {
                Drift::Isotropic(s) => {
                    c.pos[0] += s * c.drift_phase[0].sin() * df;
                    c.pos[2] += s * c.drift_phase[1].sin() * df;
                }
                Drift::Airplane => {
                    c.pos[0] += 0.20 * c.drift_phase[0].sin().abs() * df;
                    c.pos[2] += 0.05 * c.drift_phase[1].sin() * df;
                }
            }
            c.drift_phase[0] += c.drift_rate[0] * df;
            c.drift_phase[1] += c.drift_rate[1] * df;
            c.breath_phase += df.to_radians();
        }
    }
}

fn spawn_cloud(i: u32, generation: u32, p: &CloudParams, world_pos: [f32; 3]) -> Cloud {
    let s = generation.wrapping_mul(11);
    let (dx, dz) = if p.centered {
        (
            hash01(i, s + 1) * 300.0 - 150.0,
            hash01(i, s + 2) * 300.0 - 150.0,
        )
    } else {
        let sign = |h: f32| if h < 0.5 { -1.0 } else { 1.0 };
        (
            (hash01(i, s + 1) * 200.0 + 25.0) * sign(hash01(i, s + 8)),
            (hash01(i, s + 2) * 200.0 + 25.0) * sign(hash01(i, s + 9)),
        )
    };
    let y = world_pos[1]
        + p.elevation
        + if p.use_ground {
            -hash01(i, s + 3) * 5.0
        } else {
            hash01(i, s + 3) * 10.0
        };
    Cloud {
        pos: [world_pos[0] + dx, y, world_pos[2] + dz],
        distance: p.size_base + hash01(i, s + 4) * p.size_rand,
        drift_phase: [
            hash01(i, s + 5) * std::f32::consts::TAU,
            hash01(i, s + 6) * std::f32::consts::TAU,
        ],
        drift_rate: [0.3 + hash01(i, s + 10) * 0.5, 0.3 + hash01(i, s + 11) * 0.5],
        breath_phase: hash01(i, s + 7) * std::f32::consts::TAU,
        process: 0.0,
        rot_start: 300.0 + hash01(i, s + 12) * 200.0,
        alpha: 0.0,
        generation,
    }
}

impl Effect for CloudEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.step(ctx.delta * FRAMES_PER_SECOND);
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.tint;
        for c in &self.clouds {
            if c.alpha <= 0.0 {
                continue;
            }
            let side = c.distance * (1.0 + 0.05 * c.breath_phase.sin()) * SQRT2;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: c.pos,
                size: [side, side],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: self.params.textures
                    [(c.generation as usize).wrapping_add(c.distance as usize) % 3],
                color: [r, g, b, (c.alpha / 255.0).min(1.0)],
                blend: BlendKind::Alpha,
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

    fn step(e: &mut CloudEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &CloudEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn ramps_in_to_an_alpha_blended_tinted_billboard_field() {
        let mut e = CloudEffect::new([0.0, 0.0, 0.0], CLOUD3);
        assert!(draws(&e).is_empty(), "alpha starts at 0");
        step(&mut e, CLOUD3.ramp_frames);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            CLOUD3.count as usize,
            "all quads visible at peak"
        );
        let peak = CLOUD3.alpha_rate * CLOUD3.ramp_frames / 255.0;
        for p in &prims {
            let EffectPrimitiveDraw::Billboard {
                blend,
                texture,
                color,
                ..
            } = p
            else {
                unreachable!()
            };
            assert_eq!(*blend, BlendKind::Alpha);
            assert!(CLOUD_TEX.contains(texture));
            assert!(
                (color[3] - peak).abs() < 0.05,
                "near peak alpha: {}",
                color[3]
            );
        }
    }

    #[test]
    fn variants_differ_in_texture_set_tint_and_count() {
        let mut fog = CloudEffect::new([0.0, 0.0, 0.0], CLOUD4);
        step(&mut fog, CLOUD4.ramp_frames);
        let fp = draws(&fog);
        assert_eq!(fp.len(), 320);
        let EffectPrimitiveDraw::Billboard { texture, color, .. } = &fp[0] else {
            unreachable!()
        };
        assert!(FOG_TEX.contains(texture), "fog textures");
        assert!(color[0] > color[2], "warm peach tint (r>b)");

        let mut black = CloudEffect::new([0.0, 0.0, 0.0], CLOUD7);
        step(&mut black, CLOUD7.ramp_frames);
        let EffectPrimitiveDraw::Billboard { color, .. } = &draws(&black)[0] else {
            unreachable!()
        };
        assert_eq!(
            [color[0], color[1], color[2]],
            [0.0, 0.0, 0.0],
            "black tint"
        );
    }

    #[test]
    fn quads_drift_breathe_and_persist_through_a_full_loop() {
        let mut e = CloudEffect::new([0.0, 0.0, 0.0], CLOUD5);
        let pos0 = e.clouds[0].pos;
        let mut status = EffectStatus::Running;
        for _ in 0..600 {
            status = step(&mut e, 1.0);
        }
        assert_eq!(status, EffectStatus::Running, "persistent atmosphere");
        assert!(e.clouds[0].pos[0] != pos0[0], "airplane wind drifts +x");
        let mut e2 = CloudEffect::new([0.0, 0.0, 0.0], CLOUD3);
        step(&mut e2, CLOUD3.ramp_frames);
        let side_a = match &draws(&e2)[0] {
            EffectPrimitiveDraw::Billboard { size, .. } => size[0],
            _ => unreachable!(),
        };
        step(&mut e2, 90.0);
        let side_b = match &draws(&e2)[0] {
            EffectPrimitiveDraw::Billboard { size, .. } => size[0],
            _ => unreachable!(),
        };
        assert!((side_a - side_b).abs() > 1e-3, "size breathes over time");
    }
}
