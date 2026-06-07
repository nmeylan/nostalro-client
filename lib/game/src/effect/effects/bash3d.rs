//! `EF_BASH3D` family — Knight's Bash impact speed-lines (and siblings).
//!
//! References:
//! * `ro-effects/effects/imgs/350-400/364.gif` (`EF_BASH3D`)
//! * `350-400/375.gif` (`EF_BASH3D2`)
//! * `350-400/397.gif` (`EF_BASH3D3`)
//! * `350-400/398.gif` (`EF_BASH3D4`)
//! * `600-650/626.gif` (`EF_BASH3D5`)
//!
//! Not a ring like Defender/Wind — each "slot" renders two layered
//! triangular fan blades: apex above the caster, two outer points at
//! exponentially-growing distance, fading after a brief alpha pulse.
//!
//! The original game spawns N sub-instances
//! per cast (one per F1 index in a loop). Each sub-instance holds 4 fan
//! blades pointing in different directions (`rise_angle = 90·ec + F1·step`).
//! Per variant:
//!
//! | Effect       | F2 | N | Distance law       | Alpha ramp/fade   | Inner / outer tint      |
//! |--------------|----|---|--------------------|-------------------|-------------------------|
//! | `EF_BASH3D`  | 0  | 5 | `× 1.15`           | `+20 / −15`       | cyan / red              |
//! | `EF_BASH3D2` | 2  | 8 | `+ 3.0`            | `+10 / −3`        | blue / yellow           |
//! | `EF_BASH3D3` | 4  | 6 | `× 1.15`           | `+20 / −15`       | blue / yellow           |
//! | `EF_BASH3D4` | 5  | 6 | `× 1.15`           | `+20 / −15`       | grey / white            |
//! | `EF_BASH3D5` | 5  | 6 | `× 1.15`           | `+20 / −15`       | grey / white            |
//!
//! F2 = 0/4/5 also start `process` at -24 (24-frame silent wind-up); F2 = 2
//! starts at 0 (immediate). Both blades per slot use a slightly different
//! half-spread per variant; we approximate with shared `inner/outer_half_spread_deg`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::radial_emitter::{RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot};

pub const TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// All five family members run for 200 frames, like the original game.
const TOTAL_FRAMES: u32 = 200;
pub const TOTAL_DURATION_MS: u32 =
    ((TOTAL_FRAMES as f32) / FRAMES_PER_SECOND * 1000.0) as u32;

/// Apex Y offset above the
/// caster's feet. Native RO `-Y = up`, so apex is `|APEX_Y_OFFSET|` units up.
const APEX_Y_OFFSET: f32 = -12.0;

/// Initial fan radius before growth kicks in.
const DISTANCE_INITIAL: f32 = 2.0;

/// `rise_angle = 90·ec + F1·step`. The 0/2/4/5 variants use 22°/F1; the
/// Truesight variant uses 7°/F1 — see [`BashParams::rise_angle_step_per_f1_deg`].
const RISE_ANGLE_STEP_PER_SLOT_DEG: f32 = 90.0;

/// Maximum sub-instances any variant uses (Truesight = 12).
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
    /// Number of sub-instances spawned per cast (the loop count the
    /// original game's dispatcher uses).
    pub sub_instances: usize,
    /// `process` initial value. Negative values create a silent wind-up
    /// (per-frame physics is gated by `process > 0`).
    pub process_initial: i32,
    pub distance_growth: DistanceGrowth,
    /// Alpha gained per frame (8-bit) for `process ∈ 1..=10`.
    pub alpha_ramp_step_8bit: f32,
    /// Fade kicks in when `process > fade_after_frame`.
    pub fade_after_frame: i32,
    /// Alpha lost per frame (8-bit) after the hold window.
    pub alpha_fade_step_8bit: f32,
    /// Half-spread of the inner blade in degrees.
    pub inner_half_spread_deg: f32,
    /// Half-spread of the outer blade in degrees.
    pub outer_half_spread_deg: f32,
    /// 8-bit RGB tint for the inner blade (matches the inner-blade
    /// colour the original game draws).
    pub inner_color_8bit: [f32; 3],
    /// 8-bit RGB tint for the outer blade.
    pub outer_color_8bit: [f32; 3],
    /// If `true`, every fan's spin axis is locked to 0° — the
    /// out-of-plane axis (which would otherwise rotate the spike's
    /// direction out of the XZ plane and toward vertical) is disabled,
    /// and `rise_angle` alone sweeps the horizontal plane. Produces a
    /// flat 2D starburst silhouette in the XZ plane. The original game
    /// randomises the spin axis on every variant; we depart
    /// here for `EF_BASH3D2` whose reference gif shows uniformly
    /// horizontal needles.
    pub flatten_to_horizontal: bool,
    /// `rise_angle = 90·ec + F1·this`. 22° for the 0/2/4/5 family, 7°
    /// for Truesight — its 12 sub-instances pack into a tighter
    /// fan so the detection ring reads as a dense burst, not 12 sparse
    /// spokes.
    pub rise_angle_step_per_f1_deg: f32,
    /// STR layer played alongside the primitives (`Effect::str_overlay`).
    /// The 0/2/4/5 family ships as a hybrid with `bash3d.str`;
    /// Truesight is pure-procedural and sets `None`.
    pub str_overlay: Option<&'static str>,
}

impl BashParams {
    /// Alpha cap = `ramp_step * 10` (10 ramp frames, then hold).
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
};

pub const BASH3D2: BashParams = BashParams {
    sub_instances: 8,
    process_initial: 0,
    distance_growth: DistanceGrowth::Additive(3.0),
    alpha_ramp_step_8bit: 10.0,
    fade_after_frame: 11,
    alpha_fade_step_8bit: 3.0,
    // The original game's literal ±1° / ±2° reads as fat blades at
    // our world scale because the linear `+3/frame` growth pushes distance
    // far enough that the angular spread sweeps a wide base. Tightened
    // here so the silhouette reads as the thin-needle starburst the
    // reference gif shows.
    inner_half_spread_deg: 0.3,
    outer_half_spread_deg: 0.7,
    inner_color_8bit: [0.0, 0.0, 250.0],
    outer_color_8bit: [250.0, 250.0, 0.0],
    // The original's mixed 3D direction is the right look — roughly half the
    // fans (those with `ec = 1` or `ec = 3`, where `cos(rise_angle) ≈ 0`)
    // naturally fall into the horizontal "middle" plane; the rest tilt
    // up/down per their spin-axis offset.
    flatten_to_horizontal: false,
    rise_angle_step_per_f1_deg: 22.0,
    str_overlay: Some("bash3d"),
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
};

/// `EF_BASH3D5` shares all visual parameters with `EF_BASH3D4` (in the
/// original game the only difference is the spawn sound).
pub const BASH3D5: BashParams = BASH3D4;

/// `EF_TRUESIGHT` — 12 sub-instances (`i=0..11`) of the speed-line fan.
/// 12 sub-instances × 4 sectors = 48 white speed-lines forming the
/// True Sight detection burst. It differs from the base family: process
/// starts at 0 (no wind-up), `distance += 3.0` per frame (additive, like
/// the linear-growth variant), a gentle alpha curve (`+6`/frame to 60 over 10 frames, then
/// `−1`/frame), a tight 7°/F1 rise-angle step, and white inner+outer blades.
/// Pure-procedural — no STR overlay.
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
};

pub struct Bash3dEffect {
    world_pos: [f32; 3],
    params: BashParams,
    age_frames: f32,
    last_processed_frame: u32,
    /// Per-fan signed process counter. We need a parallel signed array
    /// because the emitter slot's own counter is `u32` and the family seeds
    /// `process_initial = -24` for the multiplicative branch.
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

/// The original game randomises the spin axis per fan. We use a
/// deterministic hash of `(f1, ec)` over the total fan count so tests are
/// stable and the visual stays consistent; the even spread reads better
/// as a star burst than uniform random anyway.
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
        // The 0/2/4/5 family ships as a hybrid alongside `bash3d.str`;
        // Truesight is pure-procedural and returns `None`.
        self.params.str_overlay
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
            camera_target: None, caster_yaw: None,
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
        // Base variant: 24-frame wind-up, then 5 sub × 4 slots × 2 blades = 40 quads.
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
        // Linear-growth variant: process starts at 0 — first frame already ramps alpha and
        // grows distance. 8 sub × 4 slots × 2 blades = 64 quads.
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
        let dist_1 = apex_to_outer(&prims[1]); // outer blade of fan 0
        step(&mut e, 1.0);
        let dist_2 = apex_to_outer(&draws(&e)[1]);
        // Growth = distance increment per frame = 3.0, scaled by the
        // geometry (cos terms). dist_2 - dist_1 should be roughly 3 ×
        // (length per unit distance), > 1 in any case — not a 15% bump.
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
    fn dies_after_total_frames() {
        let mut e = Bash3dEffect::new([0.0; 3], BASH3D);
        let s = step(&mut e, TOTAL_FRAMES as f32 + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }

    #[test]
    fn truesight_immediate_12_sub_white_no_str() {
        // Truesight: process starts at 0 (no wind-up), so the first frame already
        // shows all 12 sub × 4 slots × 2 blades = 96 quads, tinted white,
        // and the effect declares no STR overlay (pure-procedural).
        let mut e = Bash3dEffect::new([0.0; 3], TRUESIGHT);
        assert_eq!(e.str_overlay(), None, "Truesight is pure-procedural");
        step(&mut e, 2.0);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            TRUESIGHT.sub_instances * RADIAL_EMITTER_SLOTS * 2,
            "12 × 4 × 2 = 96 quads with no wind-up",
        );
        // White blades: RGB all near 1.0.
        let EffectPrimitiveDraw::WorldQuad { color, .. } = prims[0] else {
            panic!("expected WorldQuad");
        };
        assert!(color[0] > 0.9 && color[1] > 0.9 && color[2] > 0.9, "white tint");
    }
}
