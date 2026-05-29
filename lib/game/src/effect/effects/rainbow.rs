//! `EF_RAINBOW` — rainbow arch (enum id 410).
//!
//! A 180° arch on `alpha_center.tga`, swept in segments and built from **seven
//! concentric colour bands** (red→violet), each at a slightly smaller radius
//! (`× (1 - band·0.05)`). The arch's height is the radius **doubled**
//! (`sin · distance · 2`), giving a tall semicircle, and the whole thing is
//! yawed to face the camera. The arch ramps alpha in, holds, then fades out.
//!
//! Reference: `ro-effects/effects/imgs/400-450/410.gif`.
//!
//! Reproduced as one `WorldQuad` strip per band: each band is a thin ribbon
//! following the arch between its inner and outer radius, tinted with its
//! spectrum colour and blended additively.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const ALPHA_CENTER_TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[ALPHA_CENTER_TEXTURE];

const FPS: f32 = 60.0;
const TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;
const FADE_IN_FRAMES: f32 = 50.0;
const FADE_OUT_FRAMES: f32 = 50.0;
const PEAK_ALPHA: f32 = 160.0 / 255.0;

/// Outermost band radius (world units). The original game's `distance = 50` is engine
/// scale; the gif arch reads ~3 characters wide, so scale down.
const BASE_RADIUS: f32 = 16.0;
/// Each band sits 5% inside the previous (`1 - band·0.05`).
const BAND_RADIUS_STEP: f32 = 0.05;
/// The arch is twice as tall as its radius (`sin · distance · 2`).
const HEIGHT_FACTOR: f32 = 2.0;
/// Arch sweep segments over the 180°.
const SEGMENTS: usize = 24;

/// Seven spectrum bands, outer (red) to inner (violet) — the rainbow colour
/// table.
const BAND_COLORS: [[f32; 3]; 7] = [
    [255.0 / 255.0, 0.0, 0.0],
    [255.0 / 255.0, 126.0 / 255.0, 0.0],
    [255.0 / 255.0, 255.0 / 255.0, 0.0],
    [0.0, 255.0 / 255.0, 0.0],
    [0.0, 0.0, 255.0 / 255.0],
    [115.0 / 255.0, 50.0 / 255.0, 200.0 / 255.0],
    [255.0 / 255.0, 0.0, 255.0 / 255.0],
];

pub struct RainbowEffect {
    center: [f32; 3],
    age_frames: f32,
}

impl RainbowEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            center: world_pos,
            age_frames: 0.0,
        }
    }

    fn alpha(&self) -> f32 {
        if self.age_frames < FADE_IN_FRAMES {
            PEAK_ALPHA * (self.age_frames / FADE_IN_FRAMES)
        } else if self.age_frames < TOTAL_FRAMES - FADE_OUT_FRAMES {
            PEAK_ALPHA
        } else {
            PEAK_ALPHA * (1.0 - (self.age_frames - (TOTAL_FRAMES - FADE_OUT_FRAMES)) / FADE_OUT_FRAMES).clamp(0.0, 1.0)
        }
    }

    /// Yaw that turns the arch plane to face the camera horizontally.
    fn facing_yaw(&self, ctx: &EffectRenderCtx) -> f32 {
        let dx = ctx.camera.eye[0] - self.center[0];
        let dz = ctx.camera.eye[2] - self.center[2];
        dx.atan2(dz)
    }

    /// Arch point in world space: local `(cos t · r, -sin t · r · 2, 0)`
    /// (native RO `-Y` up) yawed around world Y to face the camera.
    fn arch_point(&self, t: f32, radius: f32, yaw: f32) -> [f32; 3] {
        let lx = t.cos() * radius;
        let ly = -t.sin() * radius * HEIGHT_FACTOR;
        let (sy, cy) = yaw.sin_cos();
        [
            self.center[0] + lx * cy,
            self.center[1] + ly,
            self.center[2] - lx * sy,
        ]
    }
}

impl Effect for RainbowEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let yaw = self.facing_yaw(ctx);
        let step = std::f32::consts::PI / SEGMENTS as f32;
        for (band, color) in BAND_COLORS.iter().enumerate() {
            let outer = BASE_RADIUS * (1.0 - band as f32 * BAND_RADIUS_STEP);
            let inner = BASE_RADIUS * (1.0 - (band as f32 + 1.0) * BAND_RADIUS_STEP);
            for i in 0..SEGMENTS {
                let t0 = i as f32 * step;
                let t1 = (i + 1) as f32 * step;
                let corners = [
                    self.arch_point(t0, outer, yaw),
                    self.arch_point(t1, outer, yaw),
                    self.arch_point(t1, inner, yaw),
                    self.arch_point(t0, inner, yaw),
                ];
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners,
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: ALPHA_CENTER_TEXTURE,
                    color: [color[0], color[1], color[2], alpha],
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut RainbowEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &RainbowEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_seven_colour_bands_as_additive_quad_strips() {
        let mut e = RainbowEffect::new([0.0; 3]);
        step(&mut e, 60.0); // past fade-in
        let prims = draws(&e);
        assert_eq!(prims.len(), 7 * SEGMENTS, "7 bands × segments");

        // Each band's segments share one colour; collect the distinct colours.
        let mut colors = Vec::new();
        for chunk in prims.chunks(SEGMENTS) {
            if let EffectPrimitiveDraw::WorldQuad { color, blend, .. } = &chunk[0] {
                assert_eq!(*blend, BlendKind::Additive);
                colors.push([color[0], color[1], color[2]]);
            }
        }
        assert_eq!(colors.len(), 7);
        // Red band first, violet/magenta last.
        assert_eq!(colors[0], [1.0, 0.0, 0.0]);
        assert_eq!(colors[6], [1.0, 0.0, 1.0]);
    }

    #[test]
    fn arch_rises_above_the_base_at_its_apex() {
        // The apex (t = 90°) sits well above the base plane (native -Y up).
        let e = RainbowEffect::new([0.0, 0.0, 0.0]);
        let apex = e.arch_point(std::f32::consts::FRAC_PI_2, BASE_RADIUS, 0.0);
        assert!(apex[1] < -BASE_RADIUS, "apex height doubled: {}", apex[1]);
    }

    #[test]
    fn alpha_ramps_in_then_out_and_dies() {
        let mut e = RainbowEffect::new([0.0; 3]);
        step(&mut e, 1.0);
        let a0 = e.alpha();
        step(&mut e, FADE_IN_FRAMES);
        let a1 = e.alpha();
        assert!(a0 < a1 && (a1 - PEAK_ALPHA).abs() < 1e-3);
        assert_eq!(step(&mut e, TOTAL_FRAMES), EffectStatus::Dead);
    }
}
