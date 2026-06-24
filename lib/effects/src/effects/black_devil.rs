//! `Blackdevil` (487) — `ring_black.tga` dark casting circles.
//!
//! A begin-spell of expanding dark casting circles. Seeds two
//! waves of 4 rings at staggered radii
//! (radius 2.5/5/7.5/10 and 2.7/5.2/…), each a flared cone. The per-frame
//! step makes every ring a *ripple*: the radius
//! grows +0.1/frame and wraps `13 → 3`, the arch height bells up then down,
//! the alpha rises while expanding then drains, and the rise angle flattens from
//! 90° to 40°. The four phase-offset rings therefore read as continuous dark
//! circles fanning outward from the caster.
//!
//! Each ring is the same flared `Frustum` cone the other casting rings use
//! (base radius = current radius, arch = arch height at the rise angle). Like the
//! dark casting circles, it uses a (50,50,50)
//! vertex tint composited with **alpha blending** — the ripples genuinely
//! darken the ground under them (additive dark would be invisible).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TEXTURE: &str = "ring_black.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const RING_SIDES: u32 = 20;
const ALPHA_PEAK: f32 = 150.0;
/// Vertex tint — dark gray, alpha-blended.
const TINT: f32 = 50.0 / 255.0;

/// Radius / spin-phase seeds for the two ring launch waves.
const SEEDS: [(f32, f32); 8] = [
    (2.5, 270.0),
    (5.0, 0.0),
    (7.5, 90.0),
    (10.0, 180.0),
    (2.7, 271.0),
    (5.2, 1.0),
    (7.7, 91.0),
    (10.2, 181.0),
];

struct Ring {
    distance: f32,
    alpha_b: f32,
    max_height: f32,
    rise_deg: f32,
    rot_start_deg: f32,
}

impl Ring {
    fn new(distance: f32, rot_start_deg: f32) -> Self {
        let mut r = Self {
            distance,
            alpha_b: 0.0,
            max_height: 0.0,
            rise_deg: 90.0,
            rot_start_deg,
        };
        r.recompute();
        r
    }

    fn step(&mut self) {
        self.distance += 0.1;
        if self.distance >= 13.0 {
            self.distance = 3.0;
            self.alpha_b = 0.0;
        }
        self.recompute();
    }

    fn recompute(&mut self) {
        let d = self.distance - 3.0;
        let mut m = if d > 5.0 {
            self.alpha_b -= 2.0;
            10.0 - d
        } else {
            self.alpha_b += 2.0;
            d
        };
        m = (m * 2.0).max(0.0);
        self.max_height = m;
        self.alpha_b = self.alpha_b.clamp(0.0, ALPHA_PEAK);
        self.rise_deg = 90.0 - d * 5.0;
    }
}

pub struct BlackDevilEffect {
    world_pos: [f32; 3],
    rings: Vec<Ring>,
    age: f32,
    frame: u32,
}

impl BlackDevilEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            rings: SEEDS.iter().map(|&(d, rot)| Ring::new(d, rot)).collect(),
            age: 0.0,
            frame: 0,
        }
    }
}

impl Effect for BlackDevilEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = (self.age * FRAMES_PER_SECOND).floor();
        self.age += ctx.delta;
        let after = (self.age * FRAMES_PER_SECOND).floor();
        for _ in 0..(after - before).max(0.0) as u32 {
            self.frame += 1;
            for r in &mut self.rings {
                r.step();
            }
        }
        // Persistent begin-spell aura; the holder despawns it via the duration
        // table.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for ring in &self.rings {
            if ring.alpha_b <= 0.0 || ring.max_height <= 0.0 {
                continue;
            }
            let (sn, cs) = ring.rise_deg.to_radians().sin_cos();
            let bottom = ring.distance;
            let height = sn * ring.max_height;
            let top = ring.distance + cs * ring.max_height;
            let alpha = (ring.alpha_b / 255.0).clamp(0.0, 1.0);
            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
                base: self.world_pos,
                bottom_size: bottom,
                top_size: top,
                height,
                sides: RING_SIDES,
                arc_angle_deg: 360.0,
                rotation: ring.rot_start_deg.to_radians(),
                uv_repeat: 1.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: TEXTURE,
                color: [TINT, TINT, TINT, alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: 1.0 / FRAMES_PER_SECOND,
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

    fn cones(e: &BlackDevilEffect) -> Vec<(f32, f32, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Frustum { bottom_size, height, color, blend, .. } => {
                    assert!(
                        color[0] < 0.3 && color[1] < 0.3 && color[2] < 0.3,
                        "ripples are dark-gray tinted, got {color:?}"
                    );
                    assert_eq!(*blend, BlendKind::Alpha, "dark rings composite, not add");
                    (*bottom_size, *height, color[3])
                }
                other => panic!("expected Frustum, got {other:?}"),
            })
            .collect()
    }

    fn tick(e: &mut BlackDevilEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&ctx());
        }
    }

    #[test]
    fn emits_nested_black_ring_cones_of_increasing_radius() {
        // Sociable: after the rings have ramped in, several flared black cones
        // are visible at distinct radii (the staggered ripples), all additive.
        let mut e = BlackDevilEffect::new([0.0, 0.0, 0.0]);
        tick(&mut e, 30);
        let c = cones(&e);
        assert!(c.len() >= 3, "multiple ripples visible, got {}", c.len());
        let mut radii: Vec<f32> = c.iter().map(|(b, _, _)| *b).collect();
        radii.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(
            radii.last().unwrap() - radii.first().unwrap() > 2.0,
            "rings span a range of radii: {radii:?}"
        );
    }

    #[test]
    fn a_ring_ripples_outward_then_resets() {
        // Track ring 0's radius: it grows over time and wraps back near 3.
        let mut e = BlackDevilEffect::new([0.0; 3]);
        let start = e.rings[0].distance;
        tick(&mut e, 40);
        assert!(e.rings[0].distance > start, "ring expands outward");
        tick(&mut e, 200);
        assert!(e.rings[0].distance < 13.0, "radius stays within the ripple band");
    }

    #[test]
    fn ring_alpha_bells_up_then_drains_over_the_ripple() {
        // Ring 0 (distance 2.5): alpha grows while it expands to distance ~8,
        // peaks, then drains as the cone flattens toward the wrap at 13.
        let mut e = BlackDevilEffect::new([0.0; 3]);
        tick(&mut e, 20);
        let early = e.rings[0].alpha_b; // still climbing
        tick(&mut e, 35); // ~distance 8, near the peak
        let peak = e.rings[0].alpha_b;
        tick(&mut e, 45); // well into the drain phase
        let late = e.rings[0].alpha_b;
        assert!(early > 0.0, "alpha ramps in as the ring grows");
        assert!(peak > early, "alpha climbs to a peak ({early} → {peak})");
        assert!(late < peak, "alpha drains as the ring flattens ({peak} → {late})");
    }
}
