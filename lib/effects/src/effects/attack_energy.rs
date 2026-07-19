//! `EF_HALFSPHERE` (436) / `EF_ATTACKENERGY` (437) / `EF_ATTACKENERGY2` (438).

use std::f32::consts::PI;

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["white02.bmp", "ring_blue.tga", "cloud11.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;
const RING_SEGMENTS: usize = 20;

const ROT_OFFSET_DEG: f32 = 180.0;

pub const HALFSPHERE_DURATION_MS: u32 = 3000;
pub const ATTACKENERGY_DURATION_MS: u32 = 3000;
pub const ATTACKENERGY2_DURATION_MS: u32 = 1000;

const DOME_SCALE: f32 = 0.75;
const RIBBON_SCALE: f32 = 0.45;

fn cos_deg(d: f32) -> f32 {
    d.to_radians().cos()
}
fn sin_deg(d: f32) -> f32 {
    d.to_radians().sin()
}

fn rot_start_deg(caster_yaw: Option<f32>) -> f32 {
    caster_yaw.map(f32::to_degrees).unwrap_or(0.0) + ROT_OFFSET_DEG
}

struct DomeLayer {
    world_pos: [f32; 3],
    alpha_b: f32,
    process: u32,
}

const DOME_DISTANCE: f32 = 8.0;
const DOME_MAX_HEIGHT: f32 = 8.0;
const DOME_VECT_MAG: f32 = 5.0;
const DOME_TINT: [f32; 3] = [250.0 / 255.0, 250.0 / 255.0, 1.0];

impl DomeLayer {
    fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            alpha_b: 0.0,
            process: 0,
        }
    }

    fn step(&mut self) {
        self.process += 1;
        if self.process <= 50 && self.alpha_b < 120.0 {
            self.alpha_b = (self.alpha_b + 5.0).min(120.0);
        }
    }

    fn collect(&self, rot_start_deg: f32, out: &mut EffectDrawList) {
        let alpha = self.alpha_b / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let vect_angle = rot_start_deg + 90.0;
        let vx = cos_deg(vect_angle) * DOME_VECT_MAG;
        let vz = sin_deg(vect_angle) * DOME_VECT_MAG;
        let center = [
            self.world_pos[0] + vx * DOME_SCALE,
            self.world_pos[1] - DOME_MAX_HEIGHT * DOME_SCALE,
            self.world_pos[2] + vz * DOME_SCALE,
        ];
        out.push(EffectPrimitiveDraw::Sphere {
            center,
            radius: DOME_DISTANCE * DOME_SCALE,
            sides_lat: 10,
            sides_lon: 10,
            longitude_offset: PI,
            longitude_arc: PI,
            uv_repeat: [1.0, 1.0],
            texture: "white02.bmp",
            color: [DOME_TINT[0], DOME_TINT[1], DOME_TINT[2], alpha],
            blend: BlendKind::Alpha,
            no_depth: false,
        });
    }
}

pub struct HalfSphereEffect {
    dome: DomeLayer,
    caster_yaw: Option<f32>,
    age_frames: f32,
    last_frame: u32,
}

impl HalfSphereEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            dome: DomeLayer::new(world_pos),
            caster_yaw: None,
            age_frames: 0.0,
            last_frame: 0,
        }
    }
}

impl Effect for HalfSphereEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.caster_yaw = ctx.caster_yaw;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        while self.last_frame < target {
            self.dome.step();
            self.last_frame += 1;
        }
        if self.age_frames * 1000.0 / FRAMES_PER_SECOND >= HALFSPHERE_DURATION_MS as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        self.dome.collect(rot_start_deg(self.caster_yaw), out);
    }
}

const COMET_DISTANCE: f32 = 8.0;
const COMET_MAX_HEIGHT: f32 = 8.0;
const COMET_HEIGHT0: f32 = 20.0;
const COMET_VECT_MAG: f32 = 5.0;

#[derive(Clone, Copy)]
struct CometLayer {
    rise_angle_deg: f32,
    spins: bool,
    alpha_b: f32,
}

impl CometLayer {
    fn new(ec: usize) -> Self {
        Self {
            rise_angle_deg: ec as f32 * 45.0,
            spins: ec < 2,
            alpha_b: 0.0,
        }
    }

    fn step(&mut self) {
        if self.spins {
            self.rise_angle_deg = (self.rise_angle_deg + 1.0).rem_euclid(360.0);
        }
        if self.alpha_b < 40.0 {
            self.alpha_b += 5.0;
        }
    }

    fn collect(&self, rot_start_deg: f32, world_pos: [f32; 3], out: &mut EffectDrawList) {
        let alpha = self.alpha_b / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let (cos_rot, sin_rot) = (cos_deg(rot_start_deg), sin_deg(rot_start_deg));
        let vect_angle = rot_start_deg + 90.0;
        let vx = cos_deg(vect_angle) * COMET_VECT_MAG;
        let vz = sin_deg(vect_angle) * COMET_VECT_MAG;
        let dist_o = COMET_DISTANCE + COMET_HEIGHT0 * COMET_DISTANCE * 0.1;

        let mut prev: Option<([f32; 3], [f32; 3])> = None;
        for i in 0..=RING_SEGMENTS {
            let count = (i * (360 / RING_SEGMENTS)) as f32;
            let ang = (count + self.rise_angle_deg).rem_euclid(360.0);
            let (c1, s1) = (cos_deg(ang), sin_deg(ang));

            let inner = self.world(
                c1 * COMET_DISTANCE,
                cos_rot * (s1 * COMET_DISTANCE) + vx,
                sin_rot * (s1 * COMET_DISTANCE) + vz,
                world_pos,
            );
            let outer = self.world(
                c1 * dist_o,
                cos_rot * (s1 * dist_o) + vx * -COMET_HEIGHT0,
                sin_rot * (s1 * dist_o) + vz * -COMET_HEIGHT0,
                world_pos,
            );

            if let Some((pi, po)) = prev {
                out.push(ribbon_quad(pi, inner, outer, po, "ring_blue.tga", alpha));
            }
            prev = Some((inner, outer));
        }
    }

    fn world(&self, y_local: f32, x_local: f32, z_local: f32, world_pos: [f32; 3]) -> [f32; 3] {
        [
            world_pos[0] + x_local * RIBBON_SCALE,
            world_pos[1] + (y_local - COMET_MAX_HEIGHT) * RIBBON_SCALE,
            world_pos[2] + z_local * RIBBON_SCALE,
        ]
    }
}

pub struct AttackEnergyEffect {
    world_pos: [f32; 3],
    dome: DomeLayer,
    layers: [CometLayer; 4],
    caster_yaw: Option<f32>,
    age_frames: f32,
    last_frame: u32,
}

impl AttackEnergyEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            dome: DomeLayer::new(world_pos),
            layers: [
                CometLayer::new(0),
                CometLayer::new(1),
                CometLayer::new(2),
                CometLayer::new(3),
            ],
            caster_yaw: None,
            age_frames: 0.0,
            last_frame: 0,
        }
    }
}

impl Effect for AttackEnergyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.caster_yaw = ctx.caster_yaw;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        while self.last_frame < target {
            self.dome.step();
            for l in &mut self.layers {
                l.step();
            }
            self.last_frame += 1;
        }
        if self.age_frames * 1000.0 / FRAMES_PER_SECOND >= ATTACKENERGY_DURATION_MS as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let rot_start = rot_start_deg(self.caster_yaw);
        self.dome.collect(rot_start, out);
        for l in &self.layers {
            l.collect(rot_start, self.world_pos, out);
        }
    }
}

const RING_DISTANCE: f32 = 8.0;
const RING_MAX_HEIGHT: f32 = 8.0;
const RING_VECT_MAG: f32 = 1.0;
const RING_BLUE_TINT: [f32; 3] = [100.0 / 255.0, 100.0 / 255.0, 1.0];

#[derive(Clone, Copy)]
struct RingLayer {
    rise_angle_deg: f32,
    height1_deg: f32,
    alpha_b: f32,
    process: u32,
    started: bool,
    spawn_frame: u32,
}

impl RingLayer {
    fn new(ec: usize, spawn_frame: u32) -> Self {
        Self {
            rise_angle_deg: ec as f32 * 45.0,
            height1_deg: ec as f32 * 22.0,
            alpha_b: 0.0,
            process: 0,
            started: false,
            spawn_frame,
        }
    }

    fn step(&mut self, global_frame: u32) {
        if !self.started {
            if global_frame < self.spawn_frame {
                return;
            }
            self.started = true;
        }
        self.process += 1;

        if self.height1_deg < 90.0 {
            self.height1_deg += 1.0;
            self.alpha_b = (self.alpha_b + 10.0).min(60.0);
        } else {
            self.alpha_b = 10.0;
            self.height1_deg = 0.0;
        }
        if self.process > 30 {
            let cap = 60.0 - ((self.process - 30) as f32 * 4.0);
            if self.alpha_b >= cap {
                self.alpha_b = cap.max(0.0);
            }
        }
        self.rise_angle_deg = (self.rise_angle_deg + 1.0).rem_euclid(360.0);
    }

    fn collect(&self, rot_start_deg: f32, world_pos: [f32; 3], out: &mut EffectDrawList) {
        let alpha = self.alpha_b / 255.0;
        if !self.started || alpha <= 0.0 {
            return;
        }
        let (cos_rot, sin_rot) = (cos_deg(rot_start_deg), sin_deg(rot_start_deg));
        let vect_angle = rot_start_deg + 90.0;
        let vx = cos_deg(vect_angle) * RING_VECT_MAG;
        let vz = sin_deg(vect_angle) * RING_VECT_MAG;

        let inner_dist = RING_DISTANCE * sin_deg(self.height1_deg);
        let cos_h1 = cos_deg(self.height1_deg);
        let ang3 = (self.height1_deg + 10.0).min(90.0);
        let outer_dist = RING_DISTANCE * sin_deg(ang3);
        let cos_h1o = cos_deg(ang3);

        let push_inner = 3.0 + cos_h1 * RING_MAX_HEIGHT;
        let push_outer = 3.0 + cos_h1o * RING_MAX_HEIGHT;

        let mut prev: Option<([f32; 3], [f32; 3])> = None;
        for i in 0..=RING_SEGMENTS {
            let count = (i * (360 / RING_SEGMENTS)) as f32;
            let ang = (count + self.rise_angle_deg).rem_euclid(360.0);
            let (c1, s1) = (cos_deg(ang), sin_deg(ang));

            let inner = self.world(
                c1 * inner_dist,
                cos_rot * (s1 * inner_dist) + vx * push_inner,
                sin_rot * (s1 * inner_dist) + vz * push_inner,
                world_pos,
            );
            let outer = self.world(
                c1 * outer_dist,
                cos_rot * (s1 * outer_dist) + vx * push_outer,
                sin_rot * (s1 * outer_dist) + vz * push_outer,
                world_pos,
            );

            if let Some((pi, po)) = prev {
                out.push(ribbon_quad(pi, inner, outer, po, "cloud11.tga", alpha));
            }
            prev = Some((inner, outer));
        }
    }

    fn world(&self, y_local: f32, x_local: f32, z_local: f32, world_pos: [f32; 3]) -> [f32; 3] {
        [
            world_pos[0] + x_local * RIBBON_SCALE,
            world_pos[1] + (y_local - RING_MAX_HEIGHT) * RIBBON_SCALE,
            world_pos[2] + z_local * RIBBON_SCALE,
        ]
    }
}

pub struct AttackEnergy2Effect {
    world_pos: [f32; 3],
    layers: Vec<RingLayer>,
    caster_yaw: Option<f32>,
    age_frames: f32,
    last_frame: u32,
}

impl AttackEnergy2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut layers = Vec::with_capacity(8);
        for &spawn in &[0u32, 4u32] {
            for ec in 0..4 {
                layers.push(RingLayer::new(ec, spawn));
            }
        }
        Self {
            world_pos,
            layers,
            caster_yaw: None,
            age_frames: 0.0,
            last_frame: 0,
        }
    }
}

impl Effect for AttackEnergy2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.caster_yaw = ctx.caster_yaw;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        while self.last_frame < target {
            let frame = self.last_frame;
            for l in &mut self.layers {
                l.step(frame);
            }
            self.last_frame += 1;
        }
        if self.age_frames * 1000.0 / FRAMES_PER_SECOND >= ATTACKENERGY2_DURATION_MS as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let rot_start = rot_start_deg(self.caster_yaw);
        for l in &self.layers {
            l.collect(rot_start, self.world_pos, out);
        }
    }
}

fn ribbon_quad(
    prev_inner: [f32; 3],
    inner: [f32; 3],
    outer: [f32; 3],
    prev_outer: [f32; 3],
    texture: &'static str,
    alpha: f32,
) -> EffectPrimitiveDraw {
    let tint = if texture == "cloud11.tga" {
        RING_BLUE_TINT
    } else {
        [1.0, 1.0, 1.0]
    };
    EffectPrimitiveDraw::WorldQuad {
        corners: [prev_inner, inner, outer, prev_outer],
        uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        texture,
        color: [tint[0], tint[1], tint[2], alpha],
        blend: BlendKind::Additive,
        no_depth: false,
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

    fn step(e: &mut dyn Effect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn draws(e: &dyn Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn halfsphere_emits_a_half_dome_that_ramps_in() {
        let mut e = HalfSphereEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 10);
        let prims = draws(&e);
        let (arc, a_early) = prims
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Sphere {
                    longitude_arc,
                    color,
                    ..
                } => Some((*longitude_arc, color[3])),
                _ => None,
            })
            .expect("dome sphere");
        assert!((arc - PI).abs() < 1e-4, "longitude swept 180° (half dome)");
        step(&mut e, 40);
        let a_late = draws(&e)
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Sphere { color, .. } => Some(color[3]),
                _ => None,
            })
            .unwrap();
        assert!(
            a_late > a_early,
            "dome alpha ramps in over the first 50 frames"
        );
    }

    #[test]
    fn attackenergy_emits_dome_plus_four_comet_ribbons() {
        let mut e = AttackEnergyEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 8);
        let prims = draws(&e);
        let domes = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Sphere { .. }))
            .count();
        let quads = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { texture, .. } if *texture == "ring_blue.tga"))
            .count();
        assert_eq!(domes, 1, "one dome");
        // 4 layers × 20 segments.
        assert_eq!(
            quads,
            4 * RING_SEGMENTS,
            "four comet ribbons of 20 segments"
        );
    }

    #[test]
    fn attackenergy2_rings_expand_as_height1_grows() {
        let mut e = AttackEnergy2Effect::new([0.0, 0.0, 0.0]);
        step(&mut e, 6);
        let span_early = vertical_span(&draws(&e));
        step(&mut e, 12);
        let span_late = vertical_span(&draws(&e));
        assert!(
            span_late > span_early,
            "rings expand as height1 grows ({span_early} -> {span_late})"
        );
    }

    #[test]
    fn comet_orientation_tracks_caster_facing() {
        fn comet_corners(yaw: Option<f32>) -> Vec<[f32; 3]> {
            let mut e = AttackEnergyEffect::new([0.0, 0.0, 0.0]);
            for _ in 0..8 {
                e.update(&EffectUpdateCtx {
                    delta: 1.0 / FRAMES_PER_SECOND,
                    camera_target: None,
                    caster_yaw: yaw,
                });
            }
            draws(&e)
                .iter()
                .filter_map(|p| match p {
                    EffectPrimitiveDraw::WorldQuad {
                        corners, texture, ..
                    } if *texture == "ring_blue.tga" => Some(corners[1]),
                    _ => None,
                })
                .collect()
        }
        let front = comet_corners(None);
        let turned = comet_corners(Some(std::f32::consts::FRAC_PI_2));
        assert_eq!(front.len(), turned.len());
        let moved = front
            .iter()
            .zip(&turned)
            .any(|(a, b)| (a[0] - b[0]).abs() > 0.5 || (a[2] - b[2]).abs() > 0.5);
        assert!(moved, "comet ribbon must rotate with the caster's facing");
    }

    fn vertical_span(prims: &[EffectPrimitiveDraw]) -> f32 {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for p in prims {
            if let EffectPrimitiveDraw::WorldQuad { corners, .. } = p {
                for c in corners {
                    lo = lo.min(c[1]);
                    hi = hi.max(c[1]);
                }
            }
        }
        if hi < lo { 0.0 } else { hi - lo }
    }
}
