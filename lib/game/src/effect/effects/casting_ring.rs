//! `EF_LEVEL99` (#200) / `EF_LEVEL995` (#397) — the spinning flared light ring
//! of the level-99 / transcendant aura.
//!
//! Three flared rings stacked on the caster, each a closed strip walked
//! around the ring at a per-ring radius, the top rim pushed outward + up by
//! `(cos rise, sin rise) * height`. The three rings differ only in
//! start angle (0°/90°/180°), radius, rise angle and height, and
//! they spin at slightly different speeds (`+3/+4/+5°` per frame) so their
//! ray patterns shimmer out of phase.
//!
//! This is the same flared-cone shape the cast circles use, so we
//! map it the same way: a flared [`Frustum`] cone per ring with a uniform top
//! rim, the *rays* coming from the `ring_blue` / `ring_white` texture wrapped
//! once around the circumference (`uv_repeat = 1`) — exactly how
//! [`super::cast_circle`] paints its petals. The reference gif confirms
//! the same shape: a flared cone (narrow base, wide top) rendered with a
//! single ring texture, an unbroken ring of rays rather than procedural
//! peaks.
//!
//! Persistent effect: the aura lives until the server clears it
//! (`table.rs` ships these ids with `u32::MAX`), so there is no fade-out — only
//! a short alpha ramp-in (the original game ramps alpha over ~20 frames).
//!
//! [`Frustum`]: EffectPrimitiveDraw::Frustum

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Three stacked rings (the original game uses three active ring slots; the
/// 4th slot is dead).
const NUM_RINGS: usize = 3;

/// Segment count around each ring. The original game walks 20
/// divisions around the ring; matching it lands the texture's ray stripes
/// on the same per-segment cadence.
const RING_SIDES: u32 = 20;

/// Full closed ring. The original game's launcher leaves a 45° gap (a
/// 315° arc), but the reference gif — which outranks that — shows an
/// unbroken ring of rays all the way around, a full-circle cone. We draw
/// the closed ring and let the three rings' 0°/90°/180° starts spread the
/// texture's ray stripes evenly.
const RING_ARC_DEG: f32 = 360.0;

/// Texture wraps exactly once around the circumference, so the ray stripes in
/// `ring_blue` paint as evenly-spaced light rays.
const RING_UV_REPEAT: f32 = 1.0;

/// Per-ring base spin in degrees/frame: ring 0 → 3, ring 1 → 4, ring 2 → 5
/// (each ring spins `ec + 3` degrees per frame). Applied negative so the
/// ring spins clockwise viewed from above, matching the original game.
const RING_SPIN_BASE_DEG_PER_FRAME: f32 = 3.0;

/// Alpha ramp-in window (frames). The original game ramps alpha by +10/frame
/// to its peak over the first ~18-20 frames.
const FADE_IN_FRAMES: f32 = 20.0;

#[derive(Clone, Copy, Debug)]
pub struct CastingRingParams {
    pub texture: &'static str,
    /// RGB tint multiplied into every ring. `ring_blue`/`ring_white` already
    /// carry most of the colour; this just nudges the hue.
    pub color_rgb: [f32; 3],
    /// Bottom-rim radius of ring 0 (narrow base of the upward funnel).
    pub bottom_size: f32,
    /// Top-rim radius of ring 0 (the cone flares outward as it rises).
    pub top_size: f32,
    /// World-space height of ring 0's top rim above the caster's feet.
    pub height: f32,
    /// Per-ring peak alpha. Level-99 rings stack to ~0.30; the map-zone aura
    /// (`MAP_AURA`) is fainter (alpha 50/255 → ~0.20).
    pub alpha_max: f32,
}

/// `EF_LEVEL99` — blue level-99 ring (`ring_blue.tga`, size 4).
pub const LV99: CastingRingParams = CastingRingParams {
    texture: "ring_blue.tga",
    color_rgb: [0.55, 0.55, 1.00],
    bottom_size: 2.0,
    top_size: 7.0,
    height: 13.0,
    alpha_max: 0.30,
};

/// `EF_LEVEL995` — white transcendant ring (`ring_white.tga`, F1=1,
/// size 7 → wider base, white tint).
pub const LV995: CastingRingParams = CastingRingParams {
    texture: "ring_white.tga",
    color_rgb: [1.00, 1.00, 1.00],
    bottom_size: 2.5,
    top_size: 8.0,
    height: 14.0,
    alpha_max: 0.30,
};

/// The flared blue ring under
/// `EF_MAP_MAGICZONE` (#650). Wide low funnel (radius ≈ 12.9, rise 55°,
/// height 15) at alpha 50/255. Reused by [`super::mapzone`].
pub const MAP_AURA: CastingRingParams = CastingRingParams {
    texture: "ring_blue.tga",
    color_rgb: [0.55, 0.55, 1.00],
    bottom_size: 12.9,
    top_size: 18.0,
    height: 12.0,
    alpha_max: 50.0 / 255.0,
};

pub const TEXTURES: &[&str] = &["ring_blue.tga", "ring_white.tga"];

pub struct CastingRingEffect {
    params: CastingRingParams,
    world_pos: [f32; 3],
    age: f32,
}

impl CastingRingEffect {
    pub fn new(world_pos: [f32; 3], params: CastingRingParams) -> Self {
        Self {
            params,
            world_pos,
            age: 0.0,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for CastingRingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        // Persistent aura — the holder despawns it via the duration table.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let frame = self.frame();
        let alpha = self.params.alpha_max * (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }

        for i in 0..NUM_RINGS {
            let fi = i as f32;
            // Outer rings: slightly shorter and flared a touch wider, matching
            // the original game (each ring is a touch shorter and wider).
            let height = self.params.height - fi * 0.5;
            let bottom_size = self.params.bottom_size;
            let top_size = self.params.top_size + fi * 0.3;
            let rot_start = fi * std::f32::consts::FRAC_PI_2;
            let spin = -(frame * (RING_SPIN_BASE_DEG_PER_FRAME + fi)).to_radians();

            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size,
                top_size,
                height,
                sides: RING_SIDES,
                arc_angle_deg: RING_ARC_DEG,
                rotation: rot_start + spin,
                uv_repeat: RING_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: self.params.texture,
                color: [r, g, b, alpha],
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

    fn run_to(c: &mut CastingRingEffect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta, camera_target: None, caster_yaw: None });
        }
    }

    fn rings(c: &CastingRingEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_flared_rings_centered_on_caster() {
        let caster = [10.0, 5.0, 20.0];
        let mut c = CastingRingEffect::new(caster, LV99);
        run_to(&mut c, FADE_IN_FRAMES);
        let prims = rings(&c);
        assert_eq!(prims.len(), NUM_RINGS);
        for p in &prims {
            let EffectPrimitiveDraw::Frustum { base, top_size, bottom_size, arc_angle_deg, blend, .. } = p else {
                panic!("expected Frustum");
            };
            assert!((base[0] - caster[0]).abs() < 1e-4 && (base[2] - caster[2]).abs() < 1e-4);
            assert!(top_size > bottom_size, "ring should flare outward as it rises");
            assert_eq!(*arc_angle_deg, RING_ARC_DEG);
            assert_eq!(*blend, BlendKind::Additive);
        }
    }

    #[test]
    fn rings_spin_at_distinct_rates() {
        let mut c = CastingRingEffect::new([0.0; 3], LV99);
        run_to(&mut c, 30.0);
        let rotations: Vec<f32> = rings(&c)
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Frustum { rotation, .. } => *rotation,
                _ => panic!(),
            })
            .collect();
        // Different base spin + start angle ⇒ all three rotations differ.
        assert!((rotations[0] - rotations[1]).abs() > 1e-3);
        assert!((rotations[1] - rotations[2]).abs() > 1e-3);
    }

    #[test]
    fn alpha_ramps_in_then_holds() {
        let mut c = CastingRingEffect::new([0.0; 3], LV99);
        run_to(&mut c, 5.0);
        let early = ring_alpha(&c);
        run_to(&mut c, FADE_IN_FRAMES);
        let peak = ring_alpha(&c);
        run_to(&mut c, FADE_IN_FRAMES * 4.0);
        let held = ring_alpha(&c);
        assert!(peak > early, "alpha ramps in ({early} → {peak})");
        assert!((held - peak).abs() < 1e-4, "alpha holds after ramp-in");
    }

    fn ring_alpha(c: &CastingRingEffect) -> f32 {
        match &rings(c)[0] {
            EffectPrimitiveDraw::Frustum { color, .. } => color[3],
            _ => panic!(),
        }
    }

    #[test]
    fn variants_use_real_distinct_textures() {
        assert_ne!(LV99.texture, LV995.texture);
        for p in [LV99, LV995] {
            assert!(TEXTURES.contains(&p.texture));
        }
    }

    #[test]
    fn never_self_terminates() {
        let mut c = CastingRingEffect::new([0.0; 3], LV99);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1, camera_target: None, caster_yaw: None }), EffectStatus::Running);
        }
    }
}
