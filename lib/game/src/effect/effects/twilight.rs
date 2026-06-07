//! `Twilight(F1)` — the Alchemist Twilight-Pharmacy visual: a
//! swarm of floating item-icon billboards that hover, gently drift, and fade in
//! then out around the caster. Twilight1/2/3 (573/574/575) differ only by the
//! icon texture(s) they pull from.
//!
//! The original game spawns the swarm 80 times per cast; each call launches a
//! batch of four sub-icons, so 320 icons appear at once. Every
//! icon is seeded with a random world offset around the caster
//! (`x,z ∈ ±80`, `y` a `5..71`-unit band *above* the caster — `−Y` is up), a
//! `3.0..4.0`-unit quad half-size, and random phases for its slow drift,
//! size shimmer, and on-screen spin. Each icon renders as
//! a camera-facing quad, alpha-blended, white tint —
//! i.e. an [`EffectPrimitiveDraw::Billboard`].
//!
//! Alpha climbs `+5/frame` for the first 50 frames (fade-in to ~250),
//! holds, then `−5/frame` over the final 50 frames (the parent runs 180 frames /
//! 3 s). All icons share the one timeline since they all spawn on
//! frame 0.
//!
//! Structurally this mirrors [`super::ghost`] (a caster-centred swarm emitting
//! one camera-facing primitive per member each frame), not the projectile path.

use std::f32::consts::SQRT_2;

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TOTAL_DURATION_MS: u32 = 3000;
const TOTAL_DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;
const DURATION_FRAMES: f32 = TOTAL_DURATION_MS as f32 / 1000.0 * FRAMES_PER_SECOND;

/// Alpha rises `+5/frame` to 250 (of 255) at frame 50, then `−5/frame` over
/// the last 50 frames back to 0.
const FADE_FRAMES: f32 = 50.0;
const MAX_ALPHA: f32 = 250.0 / 255.0;
const FADE_OUT_START: f32 = DURATION_FRAMES - FADE_FRAMES;

/// The original game launches 80 emitters × 4 icons (320). The reference gif —
/// which outranks that count — shows a far sparser, wider scatter, so we
/// thin it: fewer icons spread over a larger volume reproduces the gif's
/// in-frame density.
const ICON_COUNT: usize = 160;

/// The original game scatters icons across `±80` world units horizontally. Icon
/// *size* (`distance`) is a separate quantity from placement extent,
/// so it is kept ~1:1 (the projectile family uses the same `distance`
/// range raw) and only the scatter volume is scaled — wide enough that, like
/// the gif, much of the swarm sits outside the frame.
const WORLD_SCALE: f32 = 0.9;
const SPREAD_HALF: f32 = 80.0;
const V_BASE: f32 = 5.0;
const V_SPREAD: f32 = 66.0;

const DIST_MIN: f32 = 1.5;
const DIST_RANGE: f32 = 0.5;

/// Size shimmers by `sin(phase) * distance * 0.05`, phase `+1°/frame`.
const SIZE_PULSE: f32 = 0.05;
/// Gentle hover approximating the original game's wandering drift.
const DRIFT_AMP: f32 = 4.0;
const DRIFT_DEG_PER_FRAME: f32 = 1.5;

#[derive(Clone, Copy)]
pub struct TwilightParams {
    /// Icon textures; an icon picks one by its index (single entry for 1/2,
    /// three for the molotov/acid/alcohol mix of Twilight3).
    pub textures: &'static [&'static str],
}

const HAYAN_HERB: &str = "유저인터페이스/item/하얀허브.bmp";
const WHITE_SLIM_POTION: &str = "유저인터페이스/item/화이트슬림포션.bmp";
const MOLOTOV: &str = "유저인터페이스/item/화염병.bmp";
const ACID_BOTTLE: &str = "유저인터페이스/item/염산병.bmp";
const ALCOHOL: &str = "유저인터페이스/item/알코올.bmp";

pub const TWILIGHT1: TwilightParams = TwilightParams { textures: &[HAYAN_HERB] };
pub const TWILIGHT2: TwilightParams = TwilightParams { textures: &[WHITE_SLIM_POTION] };
pub const TWILIGHT3: TwilightParams = TwilightParams { textures: &[MOLOTOV, ACID_BOTTLE, ALCOHOL] };

pub const TEXTURES: &[&str] =
    &[HAYAN_HERB, WHITE_SLIM_POTION, MOLOTOV, ACID_BOTTLE, ALCOHOL];

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// Deterministic LCG — same constants as `stormgust` / `sandwind`. Avoids a
/// runtime `rand` dependency for the per-icon scatter.
fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

/// Uniform `[0, 1)`.
fn lcg_unit(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

struct Icon {
    /// Base offset from the caster (scaled), `−Y` up.
    offset: [f32; 3],
    distance: f32,
    tex_index: usize,
    drift_phase_x: f32,
    drift_phase_z: f32,
    shimmer_phase: f32,
    rot_phase: f32,
}

pub struct TwilightEffect {
    anchor: [f32; 3],
    params: TwilightParams,
    age: f32,
    icons: Vec<Icon>,
}

impl TwilightEffect {
    pub fn new(anchor: [f32; 3], params: TwilightParams) -> Self {
        let mut state: u32 = 0xC0FF_EE17;
        let tex_count = params.textures.len();
        let icons = (0..ICON_COUNT)
            .map(|i| {
                let ox = (lcg_unit(&mut state) * 2.0 - 1.0) * SPREAD_HALF * WORLD_SCALE;
                let oz = (lcg_unit(&mut state) * 2.0 - 1.0) * SPREAD_HALF * WORLD_SCALE;
                // `−Y` is up: subtract to lift the icon above the caster.
                let oy = -(V_BASE + lcg_unit(&mut state) * V_SPREAD) * WORLD_SCALE;
                let distance = DIST_MIN + lcg_unit(&mut state) * DIST_RANGE;
                Icon {
                    offset: [ox, oy, oz],
                    distance,
                    tex_index: i % tex_count,
                    drift_phase_x: lcg_unit(&mut state) * 360.0,
                    drift_phase_z: lcg_unit(&mut state) * 360.0,
                    shimmer_phase: lcg_unit(&mut state) * 360.0,
                    rot_phase: lcg_unit(&mut state) * 360.0,
                }
            })
            .collect();
        Self { anchor, params, age: 0.0, icons }
    }

    /// Alpha curve normalised to `0..MAX_ALPHA`.
    fn alpha_envelope(age_frames: f32) -> f32 {
        let a = if age_frames < FADE_FRAMES {
            (age_frames / FADE_FRAMES) * MAX_ALPHA
        } else if age_frames < FADE_OUT_START {
            MAX_ALPHA
        } else {
            (1.0 - (age_frames - FADE_OUT_START) / FADE_FRAMES) * MAX_ALPHA
        };
        a.clamp(0.0, MAX_ALPHA)
    }
}

impl Effect for TwilightEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let age_frames = self.age * FRAMES_PER_SECOND;
        let alpha = Self::alpha_envelope(age_frames);
        if alpha <= 0.0 {
            return;
        }
        for icon in &self.icons {
            let drift_x = DRIFT_AMP
                * WORLD_SCALE
                * (icon.drift_phase_x + age_frames * DRIFT_DEG_PER_FRAME).to_radians().sin();
            let drift_z = DRIFT_AMP
                * WORLD_SCALE
                * (icon.drift_phase_z + age_frames * DRIFT_DEG_PER_FRAME).to_radians().sin();
            let pos = [
                self.anchor[0] + icon.offset[0] + drift_x,
                self.anchor[1] + icon.offset[1],
                self.anchor[2] + icon.offset[2] + drift_z,
            ];

            let shimmer = (icon.shimmer_phase + age_frames).to_radians().sin();
            let radius = icon.distance * (1.0 + SIZE_PULSE * shimmer);
            let side = radius * SQRT_2;

            let rot = (icon.rot_phase + age_frames).to_radians();

            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [side, side],
                uv: UNIT_UV,
                rotation: rot,
                texture: self.params.textures[icon.tex_index],
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut TwilightEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        s
    }

    fn billboards(e: &TwilightEffect) -> Vec<([f32; 3], &'static str, f32)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        l.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, texture, color, .. } => (*pos, *texture, color[3]),
                _ => panic!("twilight only emits Billboard"),
            })
            .collect()
    }

    #[test]
    fn swarm_scatters_around_anchor_with_texture_variety() {
        let anchor = [10.0, 0.0, -5.0];
        let mut e = TwilightEffect::new(anchor, TWILIGHT3);
        step(&mut e, 60); // mid-life, fully faded in
        let draws = billboards(&e);
        assert_eq!(draws.len(), ICON_COUNT);
        // Icons sit above the caster and spread out (not all at one point).
        let xs: Vec<f32> = draws.iter().map(|d| d.0[0]).collect();
        let spread = xs.iter().cloned().fold(f32::MIN, f32::max)
            - xs.iter().cloned().fold(f32::MAX, f32::min);
        assert!(spread > 1.0, "icons scatter horizontally: {spread}");
        assert!(draws.iter().all(|d| d.0[1] < anchor[1]), "icons hover above caster");
        // Twilight3 mixes all three textures.
        let textures: std::collections::BTreeSet<&str> = draws.iter().map(|d| d.1).collect();
        assert_eq!(textures.len(), 3);
    }

    #[test]
    fn alpha_fades_in_holds_then_out() {
        let mut e = TwilightEffect::new([0.0; 3], TWILIGHT1);
        step(&mut e, 1);
        let a_start = billboards(&e)[0].2;
        step(&mut e, 59); // ~frame 60: held at max
        let a_mid = billboards(&e)[0].2;
        let mut e2 = TwilightEffect::new([0.0; 3], TWILIGHT1);
        step(&mut e2, (DURATION_FRAMES as u32) - 5); // near the end of fade-out
        let a_end = billboards(&e2)[0].2;
        assert!(a_start < a_mid, "fades in: {a_start} < {a_mid}");
        assert!(a_end < a_mid, "fades out: {a_end} < {a_mid}");
    }

    #[test]
    fn icons_drift_and_effect_self_terminates() {
        let mut e = TwilightEffect::new([0.0; 3], TWILIGHT2);
        step(&mut e, 50);
        let p0 = billboards(&e)[0].0;
        step(&mut e, 30);
        let p1 = billboards(&e)[0].0;
        let moved = (p1[0] - p0[0]).abs() + (p1[2] - p0[2]).abs();
        assert!(moved > 1e-4, "icon drifts over time: {moved}");

        let status = step(&mut e, DURATION_FRAMES as u32);
        assert_eq!(status, EffectStatus::Dead);
    }
}
