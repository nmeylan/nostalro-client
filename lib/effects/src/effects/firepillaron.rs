//! `EF_FIREPILLARON` (id 138) — the standing fire column of a lit Fire
//! Pillar trap.
//!
//! Matches the original game gif `100-150/138.gif` (a tall rotating column
//! of upward fire streaks). Three nested cones textured with
//! `magic_red.tga`, for a = 0..3:
//!
//! ```text
//! for (a = 0; a < 3; a++) {
//!     spin speed   = -3 deg/frame around the axis
//!     bottom radius = 4 + a
//!     top radius    = 2.5 + a              (taper)
//!     height        = 27.5 / 15.5 / 7.5
//!     alpha         = 240 - a*7
//!     texture       = effect\magic_red.tga
//! }
//! ```
//!
//! Each cone tapers (wide bottom radius, narrow top radius); the tall thin
//! inner cone + short wide outer cones stack into a flame silhouette.
//!
//! The trap is persistent — its long
//! 8000-frame ceiling is just a cap; the holder kills the effect
//! when the trap is sprung, so we never self-die.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "magic_red.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Persistent — matches the 99990 ms sentinel row in `table.rs`.
pub const TOTAL_DURATION_MS: u32 = 99_990;

// The reference is a towering fountain of vertical flame streaks (height ≈5×
// its width), so the original heights (up to 27.5) map ~1:1 to world units —
// the streak texture only resolves when the column is tall.
const WORLD_SCALE: f32 = 1.0;
const SIDES: u32 = 24;
/// -3 deg/frame: rotating the textured tube sweeps the jagged
/// `magic_red` streaks around, which is what makes the flames flicker. The
/// renderer's `rotation` is opposite-handed to the original's Y-rotation
/// (native-RO world), so the on-screen spin direction needs the sign flipped.
const SPIN_DEG_PER_FRAME: f32 = 3.0;
/// No vertical scroll. `magic_red` is a flame texture (bright base, spiky
/// transparent tips) and is *not* seamless top-to-bottom, so scrolling it in V
/// wraps the bright base around and pops a stray bright band off the tall
/// inner cone. The flame is a static streak texture; the rotation alone
/// animates it (matches the original).
const UV_RISE_PER_FRAME: f32 = 0.0;

/// Tint multiplied into the flame texture. A light pink: the dense base still
/// additively saturates to a white-hot core on its own, while the thinner
/// flame tips keep a pink-red colour — pulling green/blue down further (toward
/// orange/red) over-saturates the whole column and loses the hot core.
const FLAME_TINT: [f32; 3] = [1.0, 0.72, 0.68];

/// `(bottom_radius, top_radius, height, alpha)` per cone: the bottom ring is
/// the narrow radius (2.5/3.5/4.5) at y=0 and
/// the top ring is the wide radius (4/5/6) at y=−height. The bottom→top gap is
/// small over the tall height, so each is a near-cylinder that flares gently
/// upward (not a pronounced cone). The bright flame base (texture V=1) sits at
/// the narrow bottom; the wide top samples the transparent spiky tips (V=0).
const CONES: [(f32, f32, f32, f32); 3] = [
    (2.5, 4.0, 27.5, 240.0 / 255.0),
    (3.5, 5.0, 15.5, 233.0 / 255.0),
    (4.5, 6.0, 7.5, 226.0 / 255.0),
];

pub struct FirePillarOnEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl FirePillarOnEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }
}

impl Effect for FirePillarOnEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        // Persistent: the holder kills the trap; never self-die.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let rotation = (self.age_frames * SPIN_DEG_PER_FRAME).to_radians();
        let scroll_v = self.age_frames * UV_RISE_PER_FRAME;
        for (bottom, top, height, alpha) in CONES {
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: bottom * WORLD_SCALE,
                top_size: top * WORLD_SCALE,
                height: height * WORLD_SCALE,
                sides: SIDES,
                rotation,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, scroll_v],
                texture: TEXTURE,
                color: [FLAME_TINT[0], FLAME_TINT[1], FLAME_TINT[2], alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn emits_three_nested_cones() {
        let mut e = FirePillarOnEffect::new([0.0; 3]);
        e.update(&ctx(0.1));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let cones: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { texture, .. } if *texture == TEXTURE))
            .collect();
        assert_eq!(cones.len(), 3);
        // Inner cone is tallest, outer cone shortest (flame silhouette).
        let heights: Vec<f32> = cones
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Cylinder { height, .. } => *height,
                _ => unreachable!(),
            })
            .collect();
        assert!(heights[0] > heights[1] && heights[1] > heights[2]);
    }

    #[test]
    fn spins_over_time_and_stays_alive() {
        let mut e = FirePillarOnEffect::new([0.0; 3]);
        assert_eq!(e.update(&ctx(0.5)), EffectStatus::Running);
        let mut a = EffectDrawList::new();
        e.collect_draws(&mut a, &render_ctx());
        e.update(&ctx(0.5));
        let mut b = EffectDrawList::new();
        e.collect_draws(&mut b, &render_ctx());
        let rot = |l: &EffectDrawList| match l.primitives[0] {
            EffectPrimitiveDraw::Cylinder { rotation, .. } => rotation,
            _ => unreachable!(),
        };
        assert!((rot(&a) - rot(&b)).abs() > 1e-4, "seam spins over time");
        // Persistent: still running far past any normal effect life.
        assert_eq!(e.update(&ctx(60.0)), EffectStatus::Running);
    }
}
