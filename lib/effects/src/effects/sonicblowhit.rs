//! `EF_SONICBLOWHIT` — single horizontal impact cone fired once at the
//! target on hit.
//!
//! Reference gif: `100-150/122.gif`.
//!
//! Launches one horizontal cylinder on the spawn frame, with these
//! parameters:
//!   * latitude −90° + longitude `180 − target heading` → axis lies
//!     horizontal, yawed to face the strike heading.
//!   * a one-shot positional impulse along the strike axis added to the spawn
//!     point.
//!   * initial speed 0.4, decelerating → 12-frame slide.
//!   * outer radius 7, inner radius 4, height 3.5.
//!   * fade-out begins at frame 6 → fade in the second half.
//!   * Texture `effect/magic_red.tga`.
//!
//! Single-point anchor → heading defaults to 0 (same convention as Hit1/3/4).
//! Trail anchor supplies caster→target so the cone can aim correctly.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "magic_red.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SIDES: u32 = 10;

const CYLINDER_LIFETIME_FRAMES: f32 = 12.0;
const CYLINDER_INITIAL_SPEED: f32 = 0.4;
const CYLINDER_OUTER: f32 = 7.0;
const CYLINDER_INNER: f32 = 4.0;
const CYLINDER_HEIGHT: f32 = 3.5;
const FADE_OUT_START_FRAME: f32 = 6.0;
/// Pre-rotation Y offset for the cylinder centre. The original jitters the
/// centre between chest height (-11) and waist height
/// (-1); we match `pierce.rs` and pin at chest level — pure-Y offset, doesn't
/// rotate with heading (a jittered X component would
/// add inconsistent direction).
const CYLINDER_PIVOT_Y_OFFSET: f32 = -10.0;
const RING_ALPHA: f32 = 1.0;

pub const TOTAL_DURATION_MS: u32 =
    (CYLINDER_LIFETIME_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct SonicBlowHitEffect {
    pivot: [f32; 3],
    heading_rad: f32,
    age: f32,
}

impl SonicBlowHitEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self::new_with_trail(world_pos, world_pos)
    }

    pub fn new_with_trail(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let heading_rad = if dx.abs() < 1e-4 && dz.abs() < 1e-4 {
            0.0
        } else {
            dx.atan2(dz)
        };
        Self {
            pivot: [to[0], to[1] + CYLINDER_PIVOT_Y_OFFSET, to[2]],
            heading_rad,
            age: 0.0,
        }
    }
}

impl Effect for SonicBlowHitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age * FRAMES_PER_SECOND >= CYLINDER_LIFETIME_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        if frame >= CYLINDER_LIFETIME_FRAMES {
            return;
        }

        // Decelerating slide along the strike heading. Discrete
        // closed-form for `v0 - v0 * t / (2T)` integrated.
        let cap = CYLINDER_LIFETIME_FRAMES;
        let s = CYLINDER_INITIAL_SPEED * (frame - frame * (frame + 1.0) / (4.0 * cap));
        let (sin_h, cos_h) = self.heading_rad.sin_cos();
        let centre = [
            self.pivot[0] + s * sin_h,
            self.pivot[1],
            self.pivot[2] + s * cos_h,
        ];

        let alpha = if frame < FADE_OUT_START_FRAME {
            RING_ALPHA
        } else {
            let span = CYLINDER_LIFETIME_FRAMES - FADE_OUT_START_FRAME;
            RING_ALPHA * (1.0 - (frame - FADE_OUT_START_FRAME) / span).max(0.0)
        };

        // Same tilt/yaw convention as `pierce.rs`: tilt = +π/2 lays the
        // cylinder horizontal, yaw = -heading compensates the renderer's
        // rotation sign convention.
        out.push(EffectPrimitiveDraw::Cylinder {
            base: centre,
            bottom_size: CYLINDER_INNER,
            top_size: CYLINDER_OUTER,
            height: CYLINDER_HEIGHT,
            sides: SIDES,
            rotation: 0.0,
            tilt_x_rad: std::f32::consts::FRAC_PI_2,
            rotation_y_rad: -self.heading_rad,
            uv_scroll: [0.0, 0.0],
            texture: RING_TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Alpha,
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

    #[test]
    fn emits_single_cylinder_at_spawn_and_dies_at_12_frames() {
        // Sociable: spawn, step once, expect exactly one Cylinder primitive
        // with the expected tilt and texture; then advance past the lifetime
        // and confirm the effect reports Dead.
        let mut e = SonicBlowHitEffect::new([5.0, 0.0, 7.0]);
        e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. }))
            .count();
        assert_eq!(count, 1, "exactly one cylinder per spawn");

        let (tilt, tex) = list
            .primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Cylinder { tilt_x_rad, texture, .. } => {
                    Some((*tilt_x_rad, *texture))
                }
                _ => None,
            })
            .unwrap();
        assert!((tilt - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert_eq!(tex, RING_TEXTURE);

        let status = e.update(&EffectUpdateCtx {
            delta: TOTAL_DURATION_MS as f32 / 1000.0,
            camera_target: None, caster_yaw: None,
        });
        assert!(matches!(status, EffectStatus::Dead));
    }
}
