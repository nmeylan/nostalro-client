use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 99990;

pub const TEXTURES: &[&str] = &["thunder_center.bmp"];

const WORLD_SCALE: f32 = 1.0;
const SQRT2: f32 = std::f32::consts::SQRT_2;

pub const MAX_ORBS: usize = 5;
const ORBIT_DEG_PER_FRAME: f32 = 1.0;
const ALPHA_RAMP_PER_FRAME: f32 = 1.0 / 255.0;
const ALPHA_PEAK: f32 = 200.0 / 255.0;

#[derive(Clone, Copy)]
pub struct ChookgiParams {
    pub outer: [f32; 3],
    pub inner: [f32; 3],
    pub orbit_radius: f32,
    pub lift: f32,
    pub quad_distance: f32,
    pub jitter: f32,
    pub half_speed: bool,
    pub even_distribution: bool,
}

pub const CHOOKGI: ChookgiParams = ChookgiParams {
    outer: [0.0, 0.0, 1.0],
    inner: [1.0, 1.0, 0.0],
    orbit_radius: 7.0 * WORLD_SCALE,
    lift: 15.0 * WORLD_SCALE,
    quad_distance: 2.0 * WORLD_SCALE,
    jitter: 1.0 * WORLD_SCALE,
    half_speed: false,
    even_distribution: false,
};

pub const CHOOKGI2: ChookgiParams = ChookgiParams {
    outer: [0.0, 0.0, 1.0],
    inner: [1.0, 1.0, 0.0],
    orbit_radius: 5.5 * WORLD_SCALE,
    lift: 14.0 * WORLD_SCALE,
    quad_distance: 1.3 * WORLD_SCALE,
    jitter: 0.7 * WORLD_SCALE,
    half_speed: false,
    even_distribution: false,
};

pub const CHOOKGI3: ChookgiParams = ChookgiParams {
    outer: [68.0 / 255.0, 42.0 / 255.0, 30.0 / 255.0],
    inner: [1.0, 1.0, 1.0],
    orbit_radius: 7.0 * WORLD_SCALE,
    lift: 15.0 * WORLD_SCALE,
    quad_distance: 1.0 * WORLD_SCALE,
    jitter: 0.5 * WORLD_SCALE,
    half_speed: true,
    even_distribution: true,
};

struct Orb {
    orbit_angle: f32,
    jit_x: f32,
    jit_y: f32,
    jit_z: f32,
    pulse: f32,
}

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn deg(&mut self) -> f32 {
        (self.next_u32() % 360) as f32
    }
}

pub struct ChookgiEffect {
    world_pos: [f32; 3],
    params: ChookgiParams,
    orbs: Vec<Orb>,
    alpha: f32,
}

impl ChookgiEffect {
    pub fn new(world_pos: [f32; 3], params: ChookgiParams, count: usize) -> Self {
        let count = count.clamp(1, MAX_ORBS);
        let seed = (world_pos[0] * 73.0 + world_pos[2] * 131.0) as i64 as u32 ^ 0x9E37_79B9;
        let mut rng = Rng(seed | 1);
        let orbs = (0..count)
            .map(|i| Orb {
                orbit_angle: if params.even_distribution {
                    i as f32 * 360.0 / count as f32
                } else {
                    i as f32 * 72.0
                },
                jit_x: rng.deg(),
                jit_y: rng.deg(),
                jit_z: rng.deg(),
                pulse: rng.deg(),
            })
            .collect();
        Self {
            world_pos,
            params,
            orbs,
            alpha: 0.0,
        }
    }
}

impl Effect for ChookgiEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        self.alpha = (self.alpha + ALPHA_RAMP_PER_FRAME * frames).min(ALPHA_PEAK);
        let orbit_step = ORBIT_DEG_PER_FRAME * if self.params.half_speed { 0.5 } else { 1.0 };
        for orb in &mut self.orbs {
            orb.orbit_angle = (orb.orbit_angle + orbit_step * frames) % 360.0;
            orb.jit_x = (orb.jit_x + frames) % 360.0;
            orb.jit_y = (orb.jit_y + frames) % 360.0;
            orb.jit_z = (orb.jit_z + frames) % 360.0;
            orb.pulse = (orb.pulse + 7.0 * frames) % 360.0;
        }
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.alpha <= 0.0 {
            return;
        }
        let [cx, cy, cz] = self.world_pos;
        let p = &self.params;
        for orb in &self.orbs {
            let oa = orb.orbit_angle.to_radians();
            let pos = [
                cx + p.jitter * orb.jit_x.to_radians().sin() + p.orbit_radius * oa.sin(),
                cy - p.lift + p.jitter * orb.jit_y.to_radians().sin(),
                cz + p.jitter * orb.jit_z.to_radians().sin() + p.orbit_radius * oa.cos(),
            ];
            let pulse = orb.pulse.to_radians().sin() * p.quad_distance * 0.05;
            let outer = (p.quad_distance + pulse) * SQRT2;
            let inner = (p.quad_distance * 0.5 + pulse) * SQRT2;
            let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
            let depth_pos = [pos[0], cy, pos[2]];
            let [or, og, ob] = self.params.outer;
            out.push(EffectPrimitiveDraw::BillboardDepthAnchored {
                pos,
                depth_pos,
                size: [outer, outer],
                uv,
                rotation: 0.0,
                texture: "thunder_center.bmp",
                color: [or, og, ob, self.alpha],
                blend: BlendKind::Additive,
            });
            let [ir, ig, ib] = self.params.inner;
            out.push(EffectPrimitiveDraw::BillboardDepthAnchored {
                pos,
                depth_pos,
                size: [inner, inner],
                uv,
                rotation: 0.0,
                texture: "thunder_center.bmp",
                color: [ir, ig, ib, self.alpha],
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

    fn tick(e: &mut ChookgiEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn billboards(e: &ChookgiEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
    }

    #[test]
    fn emits_dual_billboards_per_orb() {
        let mut e = ChookgiEffect::new([0.0; 3], CHOOKGI, MAX_ORBS);
        tick(&mut e, 5);
        assert_eq!(billboards(&e).len(), MAX_ORBS * 2);
    }

    #[test]
    fn honours_sphere_count() {
        let mut three = ChookgiEffect::new([0.0; 3], CHOOKGI, 3);
        tick(&mut three, 5);
        assert_eq!(billboards(&three).len(), 3 * 2);
        let mut clamped = ChookgiEffect::new([0.0; 3], CHOOKGI, 99);
        tick(&mut clamped, 5);
        assert_eq!(billboards(&clamped).len(), MAX_ORBS * 2);
    }

    #[test]
    fn orbs_orbit_over_time() {
        let mut e = ChookgiEffect::new([1.0, 0.0, 2.0], CHOOKGI, MAX_ORBS);
        tick(&mut e, 3);
        let a = first_pos(&e);
        tick(&mut e, 60);
        let b = first_pos(&e);
        assert!(
            (a[0] - b[0]).abs() + (a[2] - b[2]).abs() > 1e-3,
            "orb orbits"
        );
        assert!(b[1] < 0.0, "orb rides above the caster's feet");
    }

    #[test]
    fn alpha_ramps_in() {
        let mut e = ChookgiEffect::new([0.0; 3], CHOOKGI, MAX_ORBS);
        tick(&mut e, 3);
        let early = billboard_alpha(&e);
        tick(&mut e, 100);
        let late = billboard_alpha(&e);
        assert!(late > early, "alpha ramps in ({early} → {late})");
    }

    #[test]
    fn outer_and_inner_use_distinct_palette() {
        let mut e = ChookgiEffect::new([0.0; 3], CHOOKGI, MAX_ORBS);
        tick(&mut e, 5);
        let b = billboards(&e);
        let outer = match &b[0] {
            EffectPrimitiveDraw::BillboardDepthAnchored { color, size, .. } => (*color, size[0]),
            _ => panic!(),
        };
        let inner = match &b[1] {
            EffectPrimitiveDraw::BillboardDepthAnchored { color, size, .. } => (*color, size[0]),
            _ => panic!(),
        };
        assert!(outer.1 > inner.1, "outer quad is larger");
        assert_ne!(outer.0, inner.0, "blue outer vs yellow inner");
    }

    #[test]
    fn variants_differ_in_palette_and_orbit() {
        assert_eq!(CHOOKGI2.outer, CHOOKGI.outer);
        assert!(CHOOKGI2.orbit_radius < CHOOKGI.orbit_radius);
        // Chookgi3 is brown/white, evenly spread, half-speed.
        assert_ne!(CHOOKGI3.outer, CHOOKGI.outer);
        assert!(CHOOKGI3.even_distribution && CHOOKGI3.half_speed);

        let three = ChookgiEffect::new([0.0; 3], CHOOKGI3, 3);
        assert!((three.orbs[1].orbit_angle - 120.0).abs() < 1e-3);
        let three_fixed = ChookgiEffect::new([0.0; 3], CHOOKGI, 3);
        assert!((three_fixed.orbs[1].orbit_angle - 72.0).abs() < 1e-3);
    }

    #[test]
    fn orbs_follow_entity_after_set_position() {
        let mut e = ChookgiEffect::new([0.0; 3], CHOOKGI, MAX_ORBS);
        tick(&mut e, 5);
        e.set_position([40.0, 0.0, -12.0]);
        let p = first_pos(&e);
        assert!(
            (p[0] - 40.0).abs() < 8.0 && (p[2] + 12.0).abs() < 8.0,
            "orbs recenter on moved entity"
        );
    }

    fn first_pos(e: &ChookgiEffect) -> [f32; 3] {
        match &billboards(e)[0] {
            EffectPrimitiveDraw::BillboardDepthAnchored { pos, .. } => *pos,
            _ => panic!(),
        }
    }
    fn billboard_alpha(e: &ChookgiEffect) -> f32 {
        match &billboards(e)[0] {
            EffectPrimitiveDraw::BillboardDepthAnchored { color, .. } => color[3],
            _ => panic!(),
        }
    }
}
