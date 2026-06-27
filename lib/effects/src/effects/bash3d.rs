//! `EF_BASH3D` family (ids 364/375/397/398/626) — Bash speed-line fan burst.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::{RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot};

pub const TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: u32 = 200;
pub const TOTAL_DURATION_MS: u32 = ((TOTAL_FRAMES as f32) / FRAMES_PER_SECOND * 1000.0) as u32;

const APEX_Y_OFFSET: f32 = -12.0;
const DISTANCE_INITIAL: f32 = 2.0;
const RISE_ANGLE_STEP_PER_SLOT_DEG: f32 = 90.0;
const MAX_SUB_INSTANCES: usize = 12;

/// Per-frame distance update law.
#[derive(Clone, Copy)]
pub enum DistanceGrowth {
    /// `distance *= factor` per frame (e.g. 1.15 → exponential).
    Multiplicative(f32),
    /// `distance += delta` per frame.
    Additive(f32),
}

#[derive(Clone, Copy)]
pub struct BashParams {
    pub sub_instances: usize,
    /// Negative values create a silent wind-up (per-frame physics is gated by `process > 0`).
    pub process_initial: i32,
    pub distance_growth: DistanceGrowth,
    pub alpha_ramp_step_8bit: f32,
    pub fade_after_frame: i32,
    pub alpha_fade_step_8bit: f32,
    pub inner_half_spread_deg: f32,
    pub outer_half_spread_deg: f32,
    pub inner_color_8bit: [f32; 3],
    pub outer_color_8bit: [f32; 3],
    pub flatten_to_horizontal: bool,
    pub rise_angle_step_per_f1_deg: f32,
    pub str_overlay: Option<&'static str>,
    pub body_tint_8bit: Option<[f32; 3]>,
    pub body_tint_window: (u32, u32),
    pub body_light_window: Option<(u32, u32)>,
}

impl BashParams {
    fn alpha_cap(&self) -> f32 {
        (self.alpha_ramp_step_8bit * 10.0 / 255.0).min(1.0)
    }

    fn ramp_step(&self) -> f32 {
        self.alpha_ramp_step_8bit / 255.0
    }

    fn fade_step(&self) -> f32 {
        self.alpha_fade_step_8bit / 255.0
    }

    fn inner_color(&self) -> [f32; 3] {
        [
            self.inner_color_8bit[0] / 255.0,
            self.inner_color_8bit[1] / 255.0,
            self.inner_color_8bit[2] / 255.0,
        ]
    }

    fn outer_color(&self) -> [f32; 3] {
        [
            self.outer_color_8bit[0] / 255.0,
            self.outer_color_8bit[1] / 255.0,
            self.outer_color_8bit[2] / 255.0,
        ]
    }
}

pub const BASH3D: BashParams = BashParams {
    sub_instances: 5,
    process_initial: -24,
    distance_growth: DistanceGrowth::Multiplicative(1.15),
    alpha_ramp_step_8bit: 20.0,
    fade_after_frame: 12,
    alpha_fade_step_8bit: 15.0,
    inner_half_spread_deg: 2.0,
    outer_half_spread_deg: 5.0,
    inner_color_8bit: [0.0, 250.0, 250.0],
    outer_color_8bit: [250.0, 0.0, 0.0],
    flatten_to_horizontal: false,
    rise_angle_step_per_f1_deg: 22.0,
    str_overlay: Some("bash3d"),
    body_tint_8bit: Some([255.0, 200.0, 200.0]),
    body_tint_window: (20, 40),
    body_light_window: None,
};

pub const BASH3D2: BashParams = BashParams {
    sub_instances: 8,
    process_initial: 0,
    distance_growth: DistanceGrowth::Additive(3.0),
    alpha_ramp_step_8bit: 10.0,
    fade_after_frame: 11,
    alpha_fade_step_8bit: 3.0,
    inner_half_spread_deg: 0.3,
    outer_half_spread_deg: 0.7,
    inner_color_8bit: [0.0, 0.0, 250.0],
    outer_color_8bit: [250.0, 250.0, 0.0],
    flatten_to_horizontal: false,
    rise_angle_step_per_f1_deg: 22.0,
    str_overlay: Some("bash3d"),
    body_tint_8bit: None,
    body_tint_window: (0, 0),
    body_light_window: Some((5, 35)),
};

pub const BASH3D3: BashParams = BashParams {
    sub_instances: 6,
    process_initial: -24,
    distance_growth: DistanceGrowth::Multiplicative(1.15),
    alpha_ramp_step_8bit: 20.0,
    fade_after_frame: 12,
    alpha_fade_step_8bit: 15.0,
    inner_half_spread_deg: 2.0,
    outer_half_spread_deg: 5.0,
    inner_color_8bit: [0.0, 0.0, 250.0],
    outer_color_8bit: [250.0, 250.0, 0.0],
    flatten_to_horizontal: false,
    rise_angle_step_per_f1_deg: 22.0,
    str_overlay: Some("bash3d"),
    body_tint_8bit: Some([255.0, 255.0, 200.0]),
    body_tint_window: (20, 50),
    body_light_window: None,
};

pub const BASH3D4: BashParams = BashParams {
    sub_instances: 6,
    process_initial: -24,
    distance_growth: DistanceGrowth::Multiplicative(1.15),
    alpha_ramp_step_8bit: 20.0,
    fade_after_frame: 12,
    alpha_fade_step_8bit: 15.0,
    inner_half_spread_deg: 2.0,
    outer_half_spread_deg: 5.0,
    inner_color_8bit: [50.0, 50.0, 50.0],
    outer_color_8bit: [250.0, 250.0, 250.0],
    flatten_to_horizontal: false,
    rise_angle_step_per_f1_deg: 22.0,
    str_overlay: Some("bash3d"),
    body_tint_8bit: Some([255.0, 255.0, 255.0]),
    body_tint_window: (20, 50),
    body_light_window: None,
};

pub const BASH3D5: BashParams = BashParams {
    body_tint_8bit: None,
    body_tint_window: (0, 0),
    ..BASH3D4
};

pub const TRUESIGHT: BashParams = BashParams {
    sub_instances: 12,
    process_initial: 0,
    distance_growth: DistanceGrowth::Additive(3.0),
    alpha_ramp_step_8bit: 6.0,
    fade_after_frame: 11,
    alpha_fade_step_8bit: 1.0,
    inner_half_spread_deg: 2.0,
    outer_half_spread_deg: 5.0,
    inner_color_8bit: [250.0, 250.0, 250.0],
    outer_color_8bit: [250.0, 250.0, 250.0],
    flatten_to_horizontal: false,
    rise_angle_step_per_f1_deg: 7.0,
    str_overlay: None,
    body_tint_8bit: None,
    body_tint_window: (0, 0),
    body_light_window: None,
};

pub struct Bash3dEffect {
    world_pos: [f32; 3],
    params: BashParams,
    age_frames: f32,
    last_processed_frame: u32,
    process: [[i32; RADIAL_EMITTER_SLOTS]; MAX_SUB_INSTANCES],
    emitters: [RadialEmitter; MAX_SUB_INSTANCES],
}

impl Bash3dEffect {
    pub fn new(world_pos: [f32; 3], params: BashParams) -> Self {
        let mut emitters = [RadialEmitter::empty(); MAX_SUB_INSTANCES];
        for f1 in 0..params.sub_instances {
            let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
            for ec in 0..RADIAL_EMITTER_SLOTS {
                let rise = (ec as f32) * RISE_ANGLE_STEP_PER_SLOT_DEG
                    + (f1 as f32) * params.rise_angle_step_per_f1_deg;
                let mut s = RadialEmitterSlot::spawn(DISTANCE_INITIAL, rise, 0.0);
                s.alpha_b = 0.0;
                s.rot_start_deg = if params.flatten_to_horizontal {
                    0.0
                } else {
                    fan_rot_start_deg(f1, ec, params.sub_instances)
                };
                slots[ec] = s;
            }
            emitters[f1] = RadialEmitter::from_slots(slots);
        }
        Self {
            world_pos,
            params,
            age_frames: 0.0,
            last_processed_frame: 0,
            process: [[params.process_initial; RADIAL_EMITTER_SLOTS]; MAX_SUB_INSTANCES],
            emitters,
        }
    }

    fn integrate_frames(&mut self, target_frame: u32) {
        let ramp_step = self.params.ramp_step();
        let fade_step = self.params.fade_step();
        let alpha_cap = self.params.alpha_cap();
        let fade_after = self.params.fade_after_frame;
        while self.last_processed_frame < target_frame {
            for f1 in 0..self.params.sub_instances {
                for ec in 0..RADIAL_EMITTER_SLOTS {
                    let slot = &mut self.emitters[f1].slots[ec];
                    if !slot.alive {
                        continue;
                    }
                    let p = &mut self.process[f1][ec];
                    *p += 1;
                    if *p <= 0 {
                        continue;
                    }

                    if *p <= 10 {
                        slot.alpha_b = (slot.alpha_b + ramp_step).min(alpha_cap);
                    } else if *p > fade_after {
                        slot.alpha_b = (slot.alpha_b - fade_step).max(0.0);
                    }
                    match self.params.distance_growth {
                        DistanceGrowth::Multiplicative(f) => slot.distance *= f,
                        DistanceGrowth::Additive(d) => slot.distance += d,
                    }
                }
            }
            self.last_processed_frame += 1;
        }
    }
}

fn fan_rot_start_deg(f1: usize, ec: usize, sub_instances: usize) -> f32 {
    let total = sub_instances * RADIAL_EMITTER_SLOTS;
    let index = (f1 * RADIAL_EMITTER_SLOTS + ec) as f32;
    (index * 360.0 / total as f32) % 360.0
}

fn fan_corners(
    center: [f32; 3],
    rise_angle_deg: f32,
    rot_start_deg: f32,
    distance: f32,
    half_spread_deg: f32,
) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let apex = [center[0], center[1] + APEX_Y_OFFSET, center[2]];
    let (sin_rs, cos_rs) = rot_start_deg.to_radians().sin_cos();
    let outer = |rise_offset_deg: f32| -> [f32; 3] {
        let (sin_r, cos_r) = (rise_angle_deg + rise_offset_deg).to_radians().sin_cos();
        [
            center[0] + cos_rs * cos_r * distance,
            apex[1] + sin_rs * cos_r * distance,
            center[2] + sin_r * distance,
        ]
    };
    (apex, outer(-half_spread_deg), outer(half_spread_deg))
}

impl Effect for Bash3dEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = (self.age_frames as u32).min(TOTAL_FRAMES);
        self.integrate_frames(target);

        if self.age_frames >= TOTAL_FRAMES as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        self.params.str_overlay
    }

    fn body_tint(&self) -> Option<BodyTint> {
        let frame = self.age_frames as u32;
        let (lo, hi) = self.params.body_tint_window;
        self.params
            .body_tint_8bit
            .filter(|_| (lo..=hi).contains(&frame))
            .map(|rgb| BodyTint {
                rgb: [rgb[0] as u8, rgb[1] as u8, rgb[2] as u8],
            })
    }

    fn body_additive(&self) -> bool {
        let frame = self.age_frames as u32;
        self.params
            .body_light_window
            .is_some_and(|(lo, hi)| (lo..=hi).contains(&frame))
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let inner_rgb = self.params.inner_color();
        let outer_rgb = self.params.outer_color();
        for f1 in 0..self.params.sub_instances {
            for (_ec, slot) in self.emitters[f1].active() {
                if slot.alpha_b <= 0.0 {
                    continue;
                }
                for (half_spread, rgb) in [
                    (self.params.inner_half_spread_deg, inner_rgb),
                    (self.params.outer_half_spread_deg, outer_rgb),
                ] {
                    let (apex, outer_lo, outer_hi) = fan_corners(
                        self.world_pos,
                        slot.rise_angle_deg,
                        slot.rot_start_deg,
                        slot.distance,
                        half_spread,
                    );
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners: [apex, outer_lo, outer_hi, apex],
                        uv: [[0.5, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.0]],
                        texture: TEXTURE,
                        color: [rgb[0], rgb[1], rgb[2], slot.alpha_b],
                        blend: BlendKind::Additive,
                        no_depth: false,
                    });
                }
            }
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

    fn step(e: &mut Bash3dEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &Bash3dEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn quad_alpha(prim: &EffectPrimitiveDraw) -> f32 {
        match prim {
            EffectPrimitiveDraw::WorldQuad { color, .. } => color[3],
            _ => panic!("expected WorldQuad, got {:?}", prim),
        }
    }

    #[test]
    fn bash3d_silent_then_full_starburst_two_layers() {
        let mut e = Bash3dEffect::new([5.0, 0.0, -3.0], BASH3D);
        step(&mut e, 10.0);
        assert!(draws(&e).is_empty(), "silent wind-up");

        step(&mut e, 20.0);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            BASH3D.sub_instances * RADIAL_EMITTER_SLOTS * 2,
            "5 × 4 × 2 = 40 quads at peak",
        );
    }

    #[test]
    fn bash3d2_starts_immediately_with_8_sub_instances_linear_growth() {
        let mut e = Bash3dEffect::new([0.0; 3], BASH3D2);
        step(&mut e, 1.0);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            BASH3D2.sub_instances * RADIAL_EMITTER_SLOTS * 2,
            "8 × 4 × 2 = 64 quads on frame 1 (no wind-up for F2=2)",
        );

        // Linear growth: after another frame, apex-to-outer increases by
        // a fixed amount per frame (not a fixed ratio).
        let apex_to_outer = |prim: &EffectPrimitiveDraw| -> f32 {
            match prim {
                EffectPrimitiveDraw::WorldQuad { corners, .. } => {
                    let d = [
                        corners[1][0] - corners[0][0],
                        corners[1][1] - corners[0][1],
                        corners[1][2] - corners[0][2],
                    ];
                    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                }
                _ => panic!(),
            }
        };
        let dist_1 = apex_to_outer(&prims[1]);
        step(&mut e, 1.0);
        let dist_2 = apex_to_outer(&draws(&e)[1]);
        let delta = dist_2 - dist_1;
        assert!(delta > 1.0, "additive growth visible: Δ = {delta}");
    }

    #[test]
    fn bash3d_alpha_pulses_then_fades() {
        let mut e = Bash3dEffect::new([0.0; 3], BASH3D);
        step(&mut e, (24 + 3) as f32);
        let alpha_ramp = quad_alpha(&draws(&e)[0]);
        assert!(
            alpha_ramp > 0.05 && alpha_ramp < BASH3D.alpha_cap(),
            "ramping alpha: {alpha_ramp}",
        );

        step(&mut e, 50.0);
        assert!(draws(&e).is_empty(), "fans fully faded");
    }

    #[test]
    fn bash3d_recolors_the_caster_body_inside_its_window() {
        let mut e = Bash3dEffect::new([0.0; 3], BASH3D);
        step(&mut e, 10.0);
        assert_eq!(e.body_tint(), None, "no tint before the window");
        step(&mut e, 20.0);
        assert_eq!(
            e.body_tint(),
            Some(BodyTint {
                rgb: [255, 200, 200]
            })
        );
        assert!(!e.body_additive());
        step(&mut e, 20.0);
        assert_eq!(e.body_tint(), None, "tint clears after the window");

        let mut g = Bash3dEffect::new([0.0; 3], BASH3D2);
        step(&mut g, 15.0);
        assert!(g.body_additive());
        assert_eq!(g.body_tint(), None);
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = Bash3dEffect::new([0.0; 3], BASH3D);
        let s = step(&mut e, TOTAL_FRAMES as f32 + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }

    #[test]
    fn truesight_immediate_12_sub_white_no_str() {
        let mut e = Bash3dEffect::new([0.0; 3], TRUESIGHT);
        assert_eq!(e.str_overlay(), None);
        step(&mut e, 2.0);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            TRUESIGHT.sub_instances * RADIAL_EMITTER_SLOTS * 2,
        );
        let EffectPrimitiveDraw::WorldQuad { color, .. } = prims[0] else {
            panic!("expected WorldQuad");
        };
        assert!(color[0] > 0.9 && color[1] > 0.9 && color[2] > 0.9);
    }
}
