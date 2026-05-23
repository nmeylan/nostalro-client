//! Bottom_Magnus family — square vertical pillars (Magnus Exorcismus,
//! Fogwall).
//!
//! Each variant draws a vertical box from one cell of
//! geometry: the 4 side faces of a box anchored at the
//! actor's feet,
//! base square in the XZ plane, top square at `y = -height` (native RO
//! `-Y` = up). Per variant the side-half-extents,
//! total height and tint differ.
//!
//! Per-variant parameters (observed behavior):
//!
//! | var| label   | half-XZ | height | tint (RGB) | blend            |
//! |----|---------|---------|--------|------------|------------------|
//! | 0  | Magnus  | 5.0     | 50.0   | 255/255/255| 0 (alpha blend)  |
//! | 1  | Sanctuary| 2.5    | 16.0   | 235/255/235| 1 (additive)     |
//! | 2  | Fogwall | 2.5     | 32.0   | 80/80/80   | 1 (additive)     |
//!
//! Sanctuary (variant 1) is already covered by
//! [`super::bottom_sanctuary_pillar`] which renders a 24-sided cylinder
//! — that's the visible Sanctuary pillar in the original game. This
//! module covers Magnus (`EF_BOTTOM_MAG`) and Fogwall
//! (`EF_BOTTOM_FOGWALL`) using a 4-sided `Frustum` (square prism).
//!
//! In the original game, variant 2 (Fogwall) also draws two diagonal
//! cross-faces inside the box at double alpha; we collapse to the box
//! alone — the cross-faces are nearly co-planar with the sides and
//! read as a slight alpha boost rather than visible geometry.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

#[derive(Clone, Copy, Debug)]
pub struct BottomMagnusParams {
    pub texture: &'static str,
    /// Half-extent of the square base in world units. (5.0 for Magnus,
    /// 2.5 for Fogwall.) The base is square, so both XZ half-extents are
    /// equal.
    pub half_extent: f32,
    /// Total pillar height in world units.
    pub height: f32,
    /// RGB tint applied per variant.
    pub tint_rgb: [f32; 3],
    /// Blend mode. Magnus uses alpha blend;
    /// Fogwall/Sanctuary use additive.
    pub blend: BlendKind,
}

const FRAMES_PER_SECOND: f32 = 60.0;
/// Fade-in window. The original game sets the alpha directly (no fade), but the visible
/// effect spawns abruptly — a short fade matches the gif timing.
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// Base alpha as a fraction of full. The original game uses alpha 30 for
/// Magnus, `100` for Fogwall; we use a single 0.7 for both — the
/// per-variant alpha difference is largely swamped by the tint contrast
/// (white vs near-black), and a per-variant alpha requires a new
/// param field that's not worth its weight.
const BASE_ALPHA: f32 = 0.7;

/// `EF_BOTTOM_MAG` (`effect\\ring_red.tga`, variant 0).
/// Tall (50-unit) white pillar.
pub const MAGNUS: BottomMagnusParams = BottomMagnusParams {
    texture: "ring_red.tga",
    half_extent: 5.0,
    height: 50.0,
    tint_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
};

/// `EF_BOTTOM_FOGWALL` (`effect\\ring_white.tga`, variant 2).
/// Medium (32-unit) dark-grey pillar — the visible "wall of fog".
pub const FOGWALL: BottomMagnusParams = BottomMagnusParams {
    texture: "ring_white.tga",
    half_extent: 2.5,
    height: 32.0,
    tint_rgb: [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0],
    blend: BlendKind::Additive,
};

pub const TEXTURES: &[&str] = &["ring_red.tga", "ring_white.tga"];

pub struct BottomMagnusEffect {
    world_pos: [f32; 3],
    params: BottomMagnusParams,
    age: f32,
}

impl BottomMagnusEffect {
    pub fn new(world_pos: [f32; 3], params: BottomMagnusParams) -> Self {
        Self {
            world_pos,
            params,
            age: 0.0,
        }
    }
}

impl Effect for BottomMagnusEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fade = (self.age / FADE_IN_SECS).clamp(0.0, 1.0);
        let alpha = BASE_ALPHA * fade;
        let [r, g, b] = self.params.tint_rgb;
        out.push(EffectPrimitiveDraw::Frustum {
            base: self.world_pos,
            // Square cross-section: bottom and top radii equal,
            // `sides = 4` gives a 4-faced prism that matches the
            // original game box exactly (4 side faces; no top
            // or bottom face — same as the original).
            bottom_size: self.params.half_extent,
            top_size: self.params.half_extent,
            height: self.params.height,
            sides: 4,
            rotation: 0.0,
            uv_repeat: 1.0,
            uv_scroll: [0.0, 0.0],
            wave_amplitude: 0.0,
            wave_frequency: 0.0,
            wave_phase: 0.0,
            wave_mode: FrustumWaveMode::Sine,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            cull_back: false,
            texture: self.params.texture,
            color: [r, g, b, alpha],
            blend: self.params.blend,
        });
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

    fn step(effect: &mut BottomMagnusEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None });
    }

    fn draws(effect: &BottomMagnusEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn magnus_emits_four_sided_alpha_pillar() {
        // Sociable test: MAGNUS spawns a Frustum with sides=4 (square
        // prism), white tint, alpha blend, height 50 — matches the
        // visible Magnus Exorcismus pillar.
        let mut e = BottomMagnusEffect::new([0.0, 0.0, 0.0], MAGNUS);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Frustum {
                sides,
                bottom_size,
                top_size,
                height,
                blend,
                color,
                texture,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!((bottom_size - 5.0).abs() < f32::EPSILON);
                assert!((top_size - 5.0).abs() < f32::EPSILON);
                assert!((height - 50.0).abs() < f32::EPSILON);
                assert_eq!(*blend, BlendKind::Alpha);
                assert_eq!(*texture, "ring_red.tga");
                assert!((color[0] - 1.0).abs() < 1e-4);
            }
            other => panic!("expected Frustum, got {other:?}"),
        }
    }

    #[test]
    fn fogwall_emits_additive_dark_pillar() {
        // Sociable test: FOGWALL is a smaller (32-unit) prism rendered
        // additively with a dark grey tint (80/255).
        let mut e = BottomMagnusEffect::new([0.0, 0.0, 0.0], FOGWALL);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Frustum {
                sides,
                height,
                blend,
                color,
                texture,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!((height - 32.0).abs() < f32::EPSILON);
                assert_eq!(*blend, BlendKind::Additive);
                assert_eq!(*texture, "ring_white.tga");
                assert!((color[0] - 80.0 / 255.0).abs() < 1e-4);
                assert!((color[2] - 80.0 / 255.0).abs() < 1e-4);
            }
            other => panic!("expected Frustum, got {other:?}"),
        }
    }
}
