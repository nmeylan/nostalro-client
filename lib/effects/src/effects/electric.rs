use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TEXTURE: &str = "elec1.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const SEGMENTS: usize = 20;
const HALF_WIDTH: f32 = 0.35;
/// 3-face tube overdraw folded into a single ribbon alpha.
const FACE_OVERDRAW: f32 = 2.5;

#[derive(Clone, Copy)]
enum SpeedLaw {
    PhaseRamp,
    Linear,
}

#[derive(Clone, Copy, PartialEq)]
enum ColorLaw {
    BlueGradient,
    Flicker,
}

#[derive(Clone, Copy)]
enum AlphaLaw {
    TipBiased,
    MidPeaked,
}

#[derive(Clone, Copy)]
struct Variant {
    distance_grow: f32,
    vertical_scale: Option<f32>,
    baseline_y: f32,
    speed_law: SpeedLaw,
    color_law: ColorLaw,
    alpha_law: AlphaLaw,
    phase_step: f32,
    phase_wrap: f32,
    ramp_frames: i32,
    ramp_rate: f32,
    ramp_rand: u32,
    drain_rate: f32,
    /// `true` → drain gates on `distance > 15`; `false` → on `process > 20`.
    drain_on_distance: bool,
    scroll_phase: bool,
    jitter: f32,
}

const RING: Variant = Variant {
    distance_grow: 0.4,
    vertical_scale: None,
    baseline_y: -2.0,
    speed_law: SpeedLaw::PhaseRamp,
    color_law: ColorLaw::BlueGradient,
    alpha_law: AlphaLaw::TipBiased,
    phase_step: 9.0,
    phase_wrap: 180.0,
    ramp_frames: 25,
    ramp_rate: 5.0,
    ramp_rand: 4,
    drain_rate: 2.0,
    drain_on_distance: true,
    scroll_phase: false,
    jitter: 0.3,
};

const AIMED: Variant = Variant {
    distance_grow: 0.6,
    vertical_scale: Some(15.0),
    baseline_y: -8.0,
    speed_law: SpeedLaw::Linear,
    color_law: ColorLaw::Flicker,
    alpha_law: AlphaLaw::MidPeaked,
    phase_step: 18.0,
    phase_wrap: 360.0,
    ramp_frames: 10,
    ramp_rate: 15.0,
    ramp_rand: 6,
    drain_rate: 5.0,
    drain_on_distance: false,
    scroll_phase: true,
    jitter: 0.5,
};

struct Arc {
    rise_angle_deg: f32,
    width: f32,
    distance: f32,
    alpha_b: f32,
    process: i32,
    height: [f32; SEGMENTS + 2],
}

impl Arc {
    fn new(rise_angle_deg: f32, width: f32, phase0: f32, launch_delay_frames: i32) -> Self {
        let mut height = [0.0; SEGMENTS + 2];
        height[0] = phase0;
        Self {
            rise_angle_deg,
            width,
            distance: 0.0,
            alpha_b: 0.0,
            process: -launch_delay_frames,
            height,
        }
    }

    fn build_ramp(&mut self, v: &Variant) {
        for _ in 0..=SEGMENTS {
            self.height[0] += v.phase_step;
            if self.height[0] > v.phase_wrap {
                self.height[0] = v.phase_wrap;
            } else {
                for i in (1..=SEGMENTS).rev() {
                    self.height[i] = self.height[i - 1];
                }
            }
        }
    }

    fn step(&mut self, v: &Variant, rng: u32) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
        self.distance += v.distance_grow;
        if self.process == 1 {
            self.build_ramp(v);
        } else if v.scroll_phase {
            for h in self.height.iter_mut() {
                *h = (*h + 8.0).rem_euclid(360.0);
            }
        }

        if self.process <= v.ramp_frames {
            self.alpha_b += v.ramp_rate + (rng % v.ramp_rand) as f32;
        }
        let past_gate = if v.drain_on_distance {
            self.distance > 15.0
        } else {
            self.process > 20
        };
        if past_gate {
            self.alpha_b -= v.drain_rate;
        }
        self.alpha_b = self.alpha_b.clamp(0.0, 255.0);
    }

    fn point(&self, v: &Variant, j: usize, jitter: [f32; 3]) -> [f32; 3] {
        let phase = self.height[j];
        let speed = match v.speed_law {
            SpeedLaw::PhaseRamp => {
                if phase > 0.0 {
                    (phase / 90.0) * self.distance
                } else {
                    0.0
                }
            }
            SpeedLaw::Linear => (j as f32 / 5.0) * self.distance,
        };
        let (sn_rise, cs_rise) = self.rise_angle_deg.to_radians().sin_cos();
        let scale = v.vertical_scale.unwrap_or(self.distance);
        [
            speed * cs_rise + jitter[0],
            -scale * phase.to_radians().sin() * self.width + v.baseline_y + jitter[1],
            speed * sn_rise + jitter[2],
        ]
    }

    fn segment_color(&self, v: &Variant, j: usize, rng: u32) -> [f32; 4] {
        let a_raw = match v.alpha_law {
            AlphaLaw::TipBiased => self.alpha_b - (j as f32) * 5.0,
            AlphaLaw::MidPeaked => {
                if j < 10 {
                    self.alpha_b - ((9 - j) as f32) * 15.0
                } else {
                    self.alpha_b - ((j - 10) as f32) * 15.0
                }
            }
        };
        let a = ((a_raw / 255.0) * FACE_OVERDRAW).clamp(0.0, 1.0);
        match v.color_law {
            ColorLaw::BlueGradient => {
                let c = (120.0 - ((SEGMENTS - j) as f32) * 6.0).max(0.0) / 255.0;
                [c, c, 1.0, a]
            }
            ColorLaw::Flicker => [
                (rng % 121) as f32 / 255.0,
                (noise(rng) % 121) as f32 / 255.0,
                1.0,
                a,
            ],
        }
    }
}

pub struct ElectricEffect {
    anchor: [f32; 3],
    variant: Variant,
    arcs: Vec<Arc>,
    age: f32,
    frame: u32,
}

impl ElectricEffect {
    pub fn new_ring(anchor: [f32; 3]) -> Self {
        let mut arcs = Vec::with_capacity(32);
        for wave in 0..4u32 {
            for f2 in [0.0, 180.0] {
                for ec in 0..4u32 {
                    let r = noise(wave * 97 + ec * 13 + f2 as u32);
                    arcs.push(Arc::new(
                        ec as f32 * 45.0 + f2,
                        (r % 11) as f32 * 0.01,
                        0.0,
                        20 + wave as i32,
                    ));
                }
            }
        }
        Self::with(anchor, RING, arcs)
    }

    pub fn new_aimed(from: [f32; 3], to: [f32; 3]) -> Self {
        let angle = (to[2] - from[2]).atan2(to[0] - from[0]).to_degrees();
        let arcs = (0..8)
            .map(|k| {
                let r = noise(k as u32 ^ 0xE1EC);
                Arc::new(
                    angle,
                    0.2 + (r % 6) as f32 * 0.01,
                    if k % 2 == 0 { 0.0 } else { 180.0 },
                    0,
                )
            })
            .collect();
        Self::with(from, AIMED, arcs)
    }

    fn with(anchor: [f32; 3], variant: Variant, arcs: Vec<Arc>) -> Self {
        Self {
            anchor,
            variant,
            arcs,
            age: 0.0,
            frame: 0,
        }
    }

    fn any_alive(&self) -> bool {
        self.arcs.iter().any(|a| a.process <= 0 || a.alpha_b > 0.0)
    }
}

fn noise(seed: u32) -> u32 {
    let mut x = seed.wrapping_mul(0x9E3779B1).wrapping_add(0x7F4A7C15);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EBCA6B);
    x ^= x >> 13;
    x
}

impl Effect for ElectricEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = (self.age * FRAMES_PER_SECOND).floor();
        self.age += ctx.delta;
        let after = (self.age * FRAMES_PER_SECOND).floor();
        let steps = (after - before).max(0.0) as u32;
        for _ in 0..steps {
            self.frame += 1;
            for (ai, arc) in self.arcs.iter_mut().enumerate() {
                arc.step(
                    &self.variant,
                    noise(self.frame.wrapping_mul(31).wrapping_add(ai as u32)),
                );
            }
        }
        if self.frame > 0 && !self.any_alive() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let v = &self.variant;
        for (ai, arc) in self.arcs.iter().enumerate() {
            if arc.process <= 0 || arc.alpha_b <= 0.0 {
                continue;
            }
            let mut points = Vec::with_capacity(SEGMENTS);
            let mut colors = Vec::with_capacity(SEGMENTS);
            for j in 1..SEGMENTS {
                let n = noise(
                    self.frame
                        .wrapping_mul(131)
                        .wrapping_add((ai * SEGMENTS + j) as u32),
                );
                let jit = |shift: u32| {
                    ((noise(n.wrapping_add(shift)) % 7) as f32 * 0.1 - 0.3) * (v.jitter / 0.3)
                };
                let local = arc.point(v, j, [jit(1), jit(2), jit(3)]);
                points.push([
                    self.anchor[0] + local[0],
                    self.anchor[1] + local[1],
                    self.anchor[2] + local[2],
                ]);
                colors.push(arc.segment_color(v, j, n));
            }
            if points.len() < 2 {
                continue;
            }
            let total_len: f32 = points
                .windows(2)
                .map(|w| {
                    let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
                    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                })
                .sum();
            let avg_seg = (total_len / (points.len() - 1) as f32).max(1e-3);
            let alpha = (arc.alpha_b / 255.0).clamp(0.0, 1.0);
            out.push(EffectPrimitiveDraw::LineStrip {
                points,
                uv_along: 1.0 / avg_seg,
                u_along: true,
                half_width: HALF_WIDTH,
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                colors: Some(colors),
                blend: BlendKind::Additive,
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

    fn tick(e: &mut ElectricEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx());
        }
        st
    }

    fn strips(e: &ElectricEffect) -> Vec<(Vec<[f32; 3]>, Vec<[f32; 4]>)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::LineStrip { points, colors, .. } => {
                    (points.clone(), colors.clone().expect("per-segment colors"))
                }
                other => panic!("expected LineStrip, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn ring_waves_fan_eight_blue_gradient_bolts_after_the_launch_delay() {
        let mut e = ElectricEffect::new_ring([10.0, 0.0, 20.0]);
        tick(&mut e, 15);
        assert!(strips(&e).is_empty(), "silent during the 20-frame delay");
        tick(&mut e, 15);
        let s = strips(&e);
        assert!(s.len() >= 8);
        let mut azimuths: Vec<f32> = s
            .iter()
            .map(|(pts, _)| {
                let t = pts.last().unwrap();
                (t[2] - 20.0).atan2(t[0] - 10.0)
            })
            .collect();
        azimuths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let spread = azimuths.last().unwrap() - azimuths.first().unwrap();
        assert!(
            spread > std::f32::consts::PI,
            "bolts cover most of the circle, got {spread}"
        );
        for (pts, colors) in &s {
            assert_eq!(pts.len(), colors.len());
            assert!(colors.iter().all(|c| c[2] > c[0]));
            assert!(colors.first().unwrap()[0] < colors.last().unwrap()[0]);
        }
    }

    #[test]
    fn aimed_bolts_point_at_the_target_and_peak_mid_bolt() {
        let from = [0.0, 0.0, 0.0];
        let to = [10.0, 0.0, 0.0];
        let mut e = ElectricEffect::new_aimed(from, to);
        tick(&mut e, 6);
        let s = strips(&e);
        assert_eq!(s.len(), 8);
        for (pts, colors) in &s {
            let tip = pts.last().unwrap();
            assert!(
                tip[0] > tip[2].abs(),
                "bolt {tip:?} should lead toward +X target"
            );
            let mid = colors[colors.len() / 2][3];
            assert!(mid >= colors.first().unwrap()[3] && mid >= colors.last().unwrap()[3]);
        }
    }

    #[test]
    fn bolt_grows_then_fades_and_dies() {
        let mut e = ElectricEffect::new_ring([0.0, 0.0, 0.0]);
        tick(&mut e, 25);
        let early: f32 = strips(&e)[0].1.iter().map(|c| c[3]).sum();
        tick(&mut e, 15);
        let peak: f32 = strips(&e)[0].1.iter().map(|c| c[3]).sum();
        assert!(peak >= early, "alpha ramps up ({early} → {peak})");
        let mut st = EffectStatus::Running;
        for _ in 0..400 {
            st = e.update(&ctx());
            if st == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(st, EffectStatus::Dead, "bolts fade out and the effect ends");
    }
}
