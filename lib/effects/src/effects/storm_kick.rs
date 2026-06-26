//! `EF_STORMKICK` family — spinning vortex + gust rings (ids 435, 459, 460,
//! 461, 464, 465).
//!
//! Each variant runs a spinning funnel
//! plus two gust rings, and for
//! the first two variants spins the caster's sprite.
//!
//! The funnel is a single
//! emitter drawn as 10 stacked
//! square plates forming an inverted shape — widest at the top
//! (radius 40, highest), narrowing toward the base
//! (`radius -= decay*(2.2 - 0.2*i)`) while each plate steps down
//! (height += 5). Every plate sits at `rotation + i*45°` and the
//! whole funnel spins (`rotation += 3..5°/frame`). Alpha ramps in over the
//! first 40 frames then fades
//! after frame 60. `F1` selects the tint.
//!
//! The two gust rings are composed verbatim
//! from [`super::portal_wind`] (configs `PORTAL_WIND2` / `PORTAL_WIND3`).

use super::portal_wind::{PORTAL_WIND2, PORTAL_WIND3, PortalWindEffect};
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["storm2.tga", "cloud11.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;
/// The funnel fades out by ~frame 100 (alpha ramps to 40 over frames 0..40,
/// holds, then −1/frame from frame 60); the gust rings outlast it slightly.
/// Pin the wall-clock end to the funnel + gust lifetime, not the looping gif.
pub const TOTAL_DURATION_MS: u32 = 1800;
const TOTAL_FRAMES: f32 = (TOTAL_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;

const STORM_TEXTURE: &str = "storm2.tga";

/// The funnel is 10 plates.
const PLATES: usize = 10;

// The original's effect literals are in world units 1:1 with ours (effect
// dimensions add onto actor positions in the same GND-zoom world space), so the
// values (radius 40, top height 50, height step 5,
// decay 2) are used directly to match the original client's vortex size.
// One factor still preserves every *ratio* (radius decay, height step).
const WORLD_SCALE: f32 = 1.0;
/// Radius of the topmost (widest) plate.
const TOP_RADIUS: f32 = 40.0 * WORLD_SCALE;
/// Radius-decay scale per plate.
const RADIUS_DECAY: f32 = 2.0 * WORLD_SCALE;
/// The topmost plate's height (negated: −Y = up).
const TOP_HEIGHT: f32 = 50.0 * WORLD_SCALE;
/// Vertical step from one plate down to the next.
const HEIGHT_STEP: f32 = 5.0 * WORLD_SCALE;
/// Spin `3 + random(3)` per frame — fixed mid-range value for
/// determinism (visually a steady spin).
const ROT_SPEED_DEG: f32 = 4.0;

fn tint(rgb: [u8; 3], alpha: f32) -> [f32; 4] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        alpha,
    ]
}

#[derive(Clone, Copy)]
pub struct StormKickConfig {
    /// Variant selector — preserved for reference / debugging.
    pub f1: u8,
    /// Per-variant RGB tint.
    pub tint_rgb: [u8; 3],
    /// Caster sprite spin (degrees/frame); 0 disables the spin.
    pub spin_per_frame: f32,
    /// Frame through which the spin is applied.
    pub spin_until_frame: i32,
}

pub const STORMKICK0: StormKickConfig = StormKickConfig {
    f1: 0,
    tint_rgb: [50, 50, 255],
    spin_per_frame: 30.0,
    spin_until_frame: 47,
};
pub const STORMKICK1: StormKickConfig = StormKickConfig {
    f1: 1,
    tint_rgb: [164, 136, 55],
    spin_per_frame: 14.0,
    spin_until_frame: 50,
};
pub const STORMKICK2: StormKickConfig = StormKickConfig {
    f1: 2,
    tint_rgb: [155, 255, 155],
    spin_per_frame: 0.0,
    spin_until_frame: 0,
};
pub const STORMKICK3: StormKickConfig = StormKickConfig {
    f1: 3,
    tint_rgb: [130, 130, 255],
    spin_per_frame: 0.0,
    spin_until_frame: 0,
};
pub const STORMKICK6: StormKickConfig = StormKickConfig {
    f1: 6,
    tint_rgb: [255, 255, 255],
    spin_per_frame: 0.0,
    spin_until_frame: 0,
};
pub const STORMKICK7: StormKickConfig = StormKickConfig {
    f1: 7,
    tint_rgb: [255, 125, 255],
    spin_per_frame: 0.0,
    spin_until_frame: 0,
};

pub struct StormKickEffect {
    world_pos: [f32; 3],
    cfg: StormKickConfig,
    age_frames: f32,
    process: i32,
    rot_start_deg: f32,
    alpha_b: f32,
    /// Accumulated caster spin (radians), applied via [`Effect::body_yaw`].
    spin_accum_deg: f32,
    wind2: PortalWindEffect,
    wind3: PortalWindEffect,
}

impl StormKickEffect {
    pub fn new(world_pos: [f32; 3], cfg: StormKickConfig) -> Self {
        Self {
            world_pos,
            cfg,
            age_frames: 0.0,
            process: 0,
            rot_start_deg: 0.0,
            alpha_b: 0.0,
            spin_accum_deg: 0.0,
            wind2: PortalWindEffect::new(world_pos, PORTAL_WIND2),
            wind3: PortalWindEffect::new(world_pos, PORTAL_WIND3),
        }
    }

    /// Spin, then fade-in (≤40) / fade-out (>60).
    fn step_one_frame(&mut self) {
        self.process += 1;
        self.rot_start_deg = (self.rot_start_deg + ROT_SPEED_DEG).rem_euclid(360.0);
        if self.process > 60 {
            self.alpha_b = (self.alpha_b - 1.0).max(0.0);
        }
        if self.process <= 40 {
            self.alpha_b += 1.0;
        }
        if self.process <= self.cfg.spin_until_frame {
            self.spin_accum_deg += self.cfg.spin_per_frame;
        }
    }

    fn current_frame_int(&self) -> i32 {
        self.age_frames.floor() as i32
    }
}

impl Effect for StormKickEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age_frames;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let steps = (self.age_frames.floor() - before.floor()).max(0.0) as i32;
        for _ in 0..steps {
            self.step_one_frame();
        }
        self.wind2.update(ctx);
        self.wind3.update(ctx);
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        let alpha = self.alpha_b / 255.0;
        if alpha > 0.0 {
            // Walk the 10 plates top→down: plate 0 is
            // the widest/highest, each later plate shrinks its radius and steps
            // down. y is negated (native −Y = up).
            let mut radius = TOP_RADIUS;
            let mut height = -TOP_HEIGHT;
            for i in 0..PLATES {
                if i > 0 {
                    height += HEIGHT_STEP;
                    radius -= RADIUS_DECAY * (2.2 - 0.2 * i as f32);
                }
                let r = radius.max(0.0);
                // Four corners 90° apart from rotation + i*45°. The first pair
                // sits one world-unit higher than the second,
                // scaled with the rest of the funnel.
                let band = 1.0 * WORLD_SCALE;
                let base_angle = self.rot_start_deg + (i as f32) * 45.0;
                let corner = |deg: f32, y: f32| {
                    let (s, c) = deg.to_radians().sin_cos();
                    [
                        self.world_pos[0] + c * r,
                        self.world_pos[1] + y,
                        self.world_pos[2] + s * r,
                    ]
                };
                let corners = [
                    corner(base_angle, height - band),
                    corner(base_angle + 90.0, height - band),
                    corner(base_angle + 180.0, height),
                    corner(base_angle + 270.0, height),
                ];
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners,
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: STORM_TEXTURE,
                    color: tint(self.cfg.tint_rgb, alpha),
                    blend: BlendKind::Additive,
                    no_depth: false,
                });
            }
        }
        self.wind2.collect_draws(out, ctx);
        self.wind3.collect_draws(out, ctx);
    }

    fn body_yaw(&self) -> Option<f32> {
        if self.cfg.spin_per_frame != 0.0 && self.current_frame_int() <= self.cfg.spin_until_frame {
            Some(self.spin_accum_deg.to_radians())
        } else {
            None
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
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut StormKickEffect, n: u32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    fn draws(e: &StormKickEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn plates(prims: &[EffectPrimitiveDraw]) -> Vec<[[f32; 3]; 4]> {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { corners, .. } => Some(*corners),
                _ => None,
            })
            .collect()
    }

    fn plate_radius(c: &[[f32; 3]; 4]) -> f32 {
        // Corner 0 is at radius `r` from world origin in XZ.
        (c[0][0] * c[0][0] + c[0][2] * c[0][2]).sqrt()
    }

    #[test]
    fn emits_ten_plates_plus_two_gust_rings() {
        let mut e = StormKickEffect::new([0.0; 3], STORMKICK0);
        step_frames(&mut e, 12);
        let prims = draws(&e);
        assert_eq!(plates(&prims).len(), 10, "10 funnel plates");
        let frustums = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        // Two composed PortalWinds, 4 slots each = up to 8 gust frustums.
        assert!(frustums > 0, "composed gust rings present: {frustums}");
    }

    #[test]
    fn funnel_is_inverted_and_spins() {
        let mut e = StormKickEffect::new([0.0; 3], STORMKICK2);
        step_frames(&mut e, 20);
        let p = plates(&draws(&e));
        assert!(
            plate_radius(&p[0]) > plate_radius(&p[9]),
            "top plate wider than bottom (inverted funnel)"
        );
        let rot_a = e.rot_start_deg;
        step_frames(&mut e, 5);
        assert!(e.rot_start_deg != rot_a, "funnel rotates over time");
    }

    #[test]
    fn alpha_ramps_then_fades() {
        let mut e = StormKickEffect::new([0.0; 3], STORMKICK3);
        step_frames(&mut e, 20);
        let early = e.alpha_b;
        assert!(early > 0.0, "alpha ramped in");
        step_frames(&mut e, 80); // past fade-out start (frame 60)
        assert!(e.alpha_b < early, "alpha fades after frame 60");
    }

    #[test]
    fn body_yaw_only_during_spin_window() {
        let mut spin = StormKickEffect::new([0.0; 3], STORMKICK0);
        step_frames(&mut spin, 10);
        assert!(spin.body_yaw().is_some(), "caster spins during window");
        step_frames(&mut spin, 60); // past spin_until_frame (47)
        assert!(spin.body_yaw().is_none(), "spin ends after window");

        let mut no_spin = StormKickEffect::new([0.0; 3], STORMKICK2);
        step_frames(&mut no_spin, 10);
        assert!(
            no_spin.body_yaw().is_none(),
            "STORMKICK2 never spins caster"
        );
    }
}
