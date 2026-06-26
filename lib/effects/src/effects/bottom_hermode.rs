//! Bottom_Hermode (`EF_BOTTOM_HERMODE`) — a small rotating cube floating above the actor.
//!
//! A small rotating cube floating above the actor.
//! Builds 6 textured quad faces from 8 corners:
//!   * 4 upper corners on a circle of radius `distance·0.8 + Rx` at
//!     `y = height_offset + sin(rot_start°)·2` (small vertical wobble).
//!   * 4 lower corners offset further "down" in native -Y up:
//!     `y_lower = y_upper + distance` (so 1.6 units further from
//!     viewer; native -Y means positive offset is below).
//!
//! Note on coordinates: native -Y up means `y = -12` sits 12 units
//! *above* the actor's feet. We derive the lower corners by subtracting
//! `distance` (1.6) from the upper Y, which in native -Y-up
//! actually moves them *upward* by 1.6 units. So the cube sits between
//! `y = actor.y - 12 + snA` (upper) and `y = actor.y - 12 + snA - 1.6`
//! (lower). The cube is small (1.6 unit "height") and floats 12 units
//! above the actor.
//!
//! Per-face shading: all faces use the same texture, all tinted with
//! R = G = a per-spawn random 0..30, and per-face B
//! channel:
//!   * top  : B = 250
//!   * sides: B = 220 / 190 / 130 / 160 (clockwise around)
//!   * bot  : B = 250
//! So the cube is overall blue with darker R/G and varying side
//! brightness — a faceted look.
//!
//! Animation:
//!   * alpha `+= 2` until 140 → ~70-frame fade-in.
//!   * rise angle `+= 1` per frame → controls `Rx` breathing and snA.
//!   * spin angle `+= 1` per frame → cube spins around Y.
//!
//! Setup:
//!   * `distance = 1.6`.
//!   * rise angle = random(360), spin angle = random(360).
//!   * height offset = -12.0, R/G tint = random(31).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const CUBE_HEIGHT_OFFSET: f32 = -12.0;
const CUBE_DISTANCE: f32 = 1.6;
const ALPHA_MAX: f32 = 140.0;
const ALPHA_RAMP_PER_FRAME: f32 = 2.0;

/// Per-face B channel (R/G come from spawn random).
const FACE_B_TOP: f32 = 250.0;
const FACE_B_SIDE_0: f32 = 220.0;
const FACE_B_SIDE_1: f32 = 190.0;
const FACE_B_SIDE_2: f32 = 130.0;
const FACE_B_SIDE_3: f32 = 160.0;
const FACE_B_BOTTOM: f32 = 250.0;

#[derive(Clone, Copy, Debug)]
pub struct BottomHermodeParams {
    pub texture: &'static str,
}

/// `EF_BOTTOM_HERMODE` → cube textured with `white02.bmp`.
pub const HERMODE: BottomHermodeParams = BottomHermodeParams {
    texture: "white02.bmp",
};

pub const TEXTURES: &[&str] = &["white02.bmp"];

pub struct BottomHermodeEffect {
    world_pos: [f32; 3],
    params: BottomHermodeParams,
    age: f32,
    frames: u32,
    /// Frozen at spawn — initial spin phase.
    rot_start_init: f32,
    /// Frozen at spawn — initial breathing phase.
    rise_angle_init: f32,
    /// Frozen at spawn — per-spawn random RG channel (0..30 / 255).
    rg_tint: f32,
}

impl BottomHermodeEffect {
    pub fn new(world_pos: [f32; 3], params: BottomHermodeParams) -> Self {
        let seed = position_hash(&world_pos);
        Self {
            world_pos,
            params,
            age: 0.0,
            frames: 0,
            rot_start_init: rand_in_range(seed, 1, 0.0, 360.0),
            rise_angle_init: rand_in_range(seed, 2, 0.0, 360.0),
            rg_tint: rand_in_range(seed, 3, 0.0, 31.0) / 255.0,
        }
    }
}

impl Effect for BottomHermodeEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let f = self.frames as f32;
        let alpha_b = (f * ALPHA_RAMP_PER_FRAME).min(ALPHA_MAX);
        let alpha = alpha_b / 255.0;
        let rot_start = (self.rot_start_init + f) % 360.0;
        let rise_angle = (self.rise_angle_init + f) % 360.0;
        let rx = CUBE_DISTANCE * 0.1 * (rise_angle.to_radians().sin() + 1.0);
        let r = CUBE_DISTANCE * 0.8 + rx;
        let sn_a = rot_start.to_radians().sin() * 2.0;

        let y_upper = self.world_pos[1] + CUBE_HEIGHT_OFFSET + sn_a;
        let y_lower = y_upper - CUBE_DISTANCE;

        // 4 corners (upper ring), positioned at the 4 cardinal angles
        // relative to rot_start, ordered around the ring — angles
        // rot_start, +90, +180, +270.
        let mut upper = [[0.0_f32; 3]; 4];
        let mut lower = [[0.0_f32; 3]; 4];
        for i in 0..4 {
            let angle_deg = (rot_start + i as f32 * 90.0) % 360.0;
            let (s, c) = angle_deg.to_radians().sin_cos();
            let x = self.world_pos[0] + c * r;
            let z = self.world_pos[2] + s * r;
            upper[i] = [x, y_upper, z];
            lower[i] = [x, y_lower, z];
        }

        let uv = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let rg = self.rg_tint;
        // Top face: 4 upper corners. R/G channel = rg_tint, B = 250.
        push_face(out, upper, uv, self.params.texture, rg, FACE_B_TOP, alpha);
        // Side faces — connect each upper-lower pair forming the 4
        // vertical sides. Each side links two adjacent upper corners to
        // the two lower corners below them, walking the ring so the
        // faces wrap the cube; the lower ring supplies the bottom edges
        // of every side quad.
        push_face(
            out,
            [upper[0], upper[1], lower[1], lower[0]],
            uv,
            self.params.texture,
            rg,
            FACE_B_SIDE_0,
            alpha,
        );
        push_face(
            out,
            [upper[1], upper[2], lower[2], lower[1]],
            uv,
            self.params.texture,
            rg,
            FACE_B_SIDE_1,
            alpha,
        );
        push_face(
            out,
            [upper[2], upper[3], lower[3], lower[2]],
            uv,
            self.params.texture,
            rg,
            FACE_B_SIDE_2,
            alpha,
        );
        push_face(
            out,
            [upper[3], upper[0], lower[0], lower[3]],
            uv,
            self.params.texture,
            rg,
            FACE_B_SIDE_3,
            alpha,
        );
        // Bottom face: 4 lower corners.
        push_face(
            out,
            lower,
            uv,
            self.params.texture,
            rg,
            FACE_B_BOTTOM,
            alpha,
        );
    }
}

fn push_face(
    out: &mut EffectDrawList,
    corners: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    texture: &'static str,
    rg: f32,
    b_channel: f32,
    alpha: f32,
) {
    out.push(EffectPrimitiveDraw::WorldQuad {
        corners,
        uv,
        texture,
        color: [rg, rg, b_channel / 255.0, alpha],
        blend: BlendKind::Additive,
        no_depth: false,
    });
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

    fn step(effect: &mut BottomHermodeEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomHermodeEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn hermode_emits_six_additive_cube_faces_above_actor() {
        // Sociable test: 6 WorldQuad faces, all additive, all
        // referencing white02.bmp, sitting ~12 units above the actor
        // (native -Y up).
        let mut e = BottomHermodeEffect::new([50.0, 5.0, 30.0], HERMODE);
        // Step past ramp-in so alpha > 0.
        step(&mut e, 0.5);
        let prims = draws(&e);
        assert_eq!(prims.len(), 6, "6 cube faces per spawn");

        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad {
                corners,
                blend,
                texture,
                ..
            } = p
            else {
                panic!("expected WorldQuad");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert_eq!(*texture, "white02.bmp");
            // All corners should be roughly 12 units above the actor's y
            // (the cube is small: 1.6 unit height, so corners are in
            // a tight Y band).
            for c in corners {
                let dy = c[1] - 5.0; // actor's y = 5.0
                assert!(
                    dy < -10.0 && dy > -16.0,
                    "corner Y should sit ~12 units above master; got dy={dy}",
                );
            }
        }
    }

    #[test]
    fn hermode_top_and_bottom_faces_share_brightest_b_channel() {
        // The 250-B top + bottom faces should be the brightest of the 6.
        let mut e = BottomHermodeEffect::new([0.0, 0.0, 0.0], HERMODE);
        step(&mut e, 1.0);
        let prims = draws(&e);
        let b_channels: Vec<f32> = prims
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { color, .. } => color[2],
                _ => panic!(),
            })
            .collect();
        // Expect: top=250, side=220, side=190, side=130, side=160,
        // bot=250 — divided by 255. Highest should be 250/255.
        let max_b = b_channels.iter().copied().fold(0.0_f32, f32::max);
        assert!((max_b - 250.0 / 255.0).abs() < 1e-3);
        // At least 2 faces share the max value (top + bottom).
        let max_count = b_channels
            .iter()
            .filter(|b| (*b - max_b).abs() < 1e-3)
            .count();
        assert!(
            max_count >= 2,
            "expected top+bottom to share max B; got {max_count}"
        );
    }

    #[test]
    fn hermode_cube_spins_over_time() {
        // After enough frames, the first upper corner should have moved
        // on its XZ circle.
        let mut e = BottomHermodeEffect::new([0.0, 0.0, 0.0], HERMODE);
        step(&mut e, 0.5);
        let p_a = match &draws(&e)[0] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => corners[0],
            _ => panic!(),
        };
        step(&mut e, 1.5);
        let p_b = match &draws(&e)[0] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => corners[0],
            _ => panic!(),
        };
        let d = (p_a[0] - p_b[0]).hypot(p_a[2] - p_b[2]);
        assert!(
            d > 0.2,
            "expected cube rotation to displace corner; got d={d}"
        );
    }
}
