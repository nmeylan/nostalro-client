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
//! The `height` column below is the variant's *peak*: the pillar rises from
//! the ground over ~90 frames and then breathes between 0.30× and 1.0× of it
//! (averaging ~0.65×), so the steady look is much shorter than the peak.
//!
//! Per-variant parameters (observed behavior):
//!
//! | var| label   | half-XZ | peak height | tint (RGB) | blend       |
//! |----|---------|---------|-------------|------------|-------------|
//! | 0  | Magnus  | 5.0     | 20.0        | 255/255/255| additive    |
//! | 1  | Sanctuary| 2.5    | 16.0        | 235/255/235| additive    |
//! | 2  | Fogwall | 2.5     | 32.0        | 80/80/80   | additive    |
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

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

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
    /// Alpha at the top of the pillar (fully visible crown).
    pub alpha_top: f32,
    /// Alpha at the base of the pillar. Set low so the caster sprite shows
    /// through; the GPU interpolates from base to crown. Original game uses
    /// `alphaB = 30/255 ≈ 0.12` for Magnus, uniform for Fogwall.
    pub alpha_bottom: f32,
    /// Blend mode. Magnus uses alpha blend;
    /// Fogwall/Sanctuary use additive.
    pub blend: BlendKind,
}

const FRAMES_PER_SECOND: f32 = 60.0;
/// Fade-in window for alpha.
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// Frames over which the pillar rises from the ground to its full height.
const HEIGHT_RAMP_FRAMES: f32 = 90.0;
/// The pillar height breathes: it pulses around `max_height * STEADY_FRAC`
/// with amplitude `max_height * PULSE_FRAC`, so it ranges
/// `[max_height*(STEADY-PULSE), max_height*(STEADY+PULSE)]` =
/// `[0.30, 1.0] * max_height`, averaging `0.65 * max_height`. The angle
/// advances 1°/frame (a ~6 s breath cycle).
const STEADY_FRAC: f32 = 0.65;
const PULSE_FRAC: f32 = 0.35;

/// The animated pillar height at a given age. `max_height` is the variant's
/// peak height (the table value); the steady look is shorter (~65 %) and the
/// pillar rises from the ground over the first [`HEIGHT_RAMP_FRAMES`].
/// `phase_deg` offsets the breath so concurrent pillars don't pulse in
/// lockstep. Shared with the Sanctuary pillar, which breathes identically.
pub(crate) fn animated_height(max_height: f32, age: f32, phase_deg: f32) -> f32 {
    let frame = age * FRAMES_PER_SECOND;
    let angle = (phase_deg + frame).rem_euclid(360.0);
    let mut h = max_height * angle.to_radians().sin() * PULSE_FRAC + max_height * STEADY_FRAC;
    if frame < HEIGHT_RAMP_FRAMES {
        h *= frame.to_radians().sin();
    }
    h
}

/// `EF_BOTTOM_MAG` (`effect\\ring_red.tga`, variant 0).
/// A stocky red glow column at the caster's feet. Rendered ADDITIVELY (the
/// original game blends this `PW==0` face additively), so the bright base
/// glows over the caster without ever hiding the sprite. The alpha fades from
/// a bright base to a transparent top, concentrating the visible mass low and
/// letting the column wisp out — the stocky look of the original. The peak
/// height is scaled into our world units (the original's raw 50 reads as ~10
/// cells, far too tall here).
pub const MAGNUS: BottomMagnusParams = BottomMagnusParams {
    texture: "ring_red.tga",
    half_extent: 5.0,
    height: 20.0,
    tint_rgb: [1.0, 1.0, 1.0],
    alpha_top: 0.0,
    alpha_bottom: 0.7,
    blend: BlendKind::Additive,
};

/// `EF_BOTTOM_FOGWALL` (`effect\\ring_white.tga`, variant 2).
/// Medium (32-unit) dark-grey pillar — the visible "wall of fog".
pub const FOGWALL: BottomMagnusParams = BottomMagnusParams {
    texture: "ring_white.tga",
    half_extent: 2.5,
    height: 32.0,
    tint_rgb: [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0],
    alpha_top: 0.7,
    alpha_bottom: 0.7,
    blend: BlendKind::Additive,
};

pub const TEXTURES: &[&str] = &["ring_red.tga", "ring_white.tga"];

pub struct BottomMagnusEffect {
    world_pos: [f32; 3],
    params: BottomMagnusParams,
    age: f32,
    phase_deg: f32,
}

impl BottomMagnusEffect {
    pub fn new(world_pos: [f32; 3], params: BottomMagnusParams) -> Self {
        let key = (world_pos[0].to_bits() ^ world_pos[2].to_bits()) as f32 * 1.6180339;
        Self {
            world_pos,
            params,
            age: 0.0,
            phase_deg: key.rem_euclid(360.0),
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
        let height = animated_height(self.params.height, self.age, self.phase_deg);
        let [r, g, b] = self.params.tint_rgb;
        out.push(EffectPrimitiveDraw::Cylinder {
            base: self.world_pos,
            // Square cross-section: bottom and top radii equal,
            // `sides = 4` gives a 4-faced prism that matches the
            // original game box exactly (4 side faces; no top
            // or bottom face — same as the original).
            bottom_size: self.params.half_extent,
            top_size: self.params.half_extent,
            height,
            sides: 4,
            rotation: 0.0,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            uv_scroll: [0.0, 0.0],
            texture: self.params.texture,
            color: [r, g, b, self.params.alpha_top * fade],
            alpha_bottom: self.params.alpha_bottom * fade,
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
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomMagnusEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn magnus_emits_four_sided_additive_pillar_fading_to_transparent_top() {
        // Sociable test: MAGNUS spawns a 4-sided additive prism whose base is
        // bright and whose top is transparent, so the visible mass sits low
        // (the stocky look) and the bright base glows over the caster without
        // hiding the sprite.
        let mut e = BottomMagnusEffect::new([0.0, 0.0, 0.0], MAGNUS);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder {
                sides,
                bottom_size,
                top_size,
                blend,
                color,
                alpha_bottom,
                texture,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!((bottom_size - 5.0).abs() < f32::EPSILON);
                assert!((top_size - 5.0).abs() < f32::EPSILON);
                assert_eq!(*blend, BlendKind::Additive);
                assert_eq!(*texture, "ring_red.tga");
                assert!((color[0] - 1.0).abs() < 1e-4, "white tint");
                assert!(*alpha_bottom > color[3], "base brighter than the top");
                assert!(color[3] < 1e-4, "top fades to transparent: {}", color[3]);
            }
            other => panic!("expected Cylinder, got {other:?}"),
        }
    }

    #[test]
    fn pillar_rises_from_ground_then_breathes_below_peak() {
        // The pillar ramps up from near zero over the first 90 frames and,
        // once steady, breathes within [0.30, 1.0]·max_height — never sitting
        // at the static peak the table value implies.
        let max = MAGNUS.height;
        let early = animated_height(max, 5.0 / FRAMES_PER_SECOND, 0.0);
        assert!(early < 0.3 * max, "still rising from the ground: {early}");

        // Sample a full breath cycle (≈360 frames) past the ramp and check
        // bounds + that the average is well under the peak.
        let (mut lo, mut hi, mut sum, mut n) = (f32::MAX, 0.0_f32, 0.0, 0);
        for f in 90..=450 {
            let h = animated_height(max, f as f32 / FRAMES_PER_SECOND, 0.0);
            lo = lo.min(h);
            hi = hi.max(h);
            sum += h;
            n += 1;
        }
        assert!(lo >= 0.30 * max - 0.5 && hi <= max + 0.5, "bounds: {lo}..{hi}");
        let avg = sum / n as f32;
        assert!(
            (avg - 0.65 * max).abs() < 0.05 * max,
            "average breath ~0.65·max, got {avg}"
        );
    }

    #[test]
    fn fogwall_emits_additive_dark_pillar() {
        // Sociable test: FOGWALL is a smaller (32-unit) prism rendered
        // additively with a dark grey tint (80/255).
        let mut e = BottomMagnusEffect::new([0.0, 0.0, 0.0], FOGWALL);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder {
                sides,
                height,
                blend,
                color,
                texture,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!(*height <= FOGWALL.height + 0.5, "never exceeds peak height");
                assert_eq!(*blend, BlendKind::Additive);
                assert_eq!(*texture, "ring_white.tga");
                assert!((color[0] - 80.0 / 255.0).abs() < 1e-4);
                assert!((color[2] - 80.0 / 255.0).abs() < 1e-4);
            }
            other => panic!("expected Cylinder, got {other:?}"),
        }
    }
}
