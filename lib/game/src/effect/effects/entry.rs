//! EF_ENTRY — actor materialization effect (portal-in).
//!
//! Two cylinders launched together at frame 0, both 55-frame lifetime,
//! both `ring_blue.tga`. Each cylinder's local frame is: bottom ring at
//! `y = 0` with the bottom radius, top ring at `y = -height` with the top
//! radius — so when the top radius exceeds the bottom radius the cone
//! **flares outward going up** (chalice shape), and the flare grows when
//! the top radius is still widening.
//!
//! * Outer flared cone (bottom radius 5, top radius `6 + 0.08·frame`,
//!   height `6.5 - 0.1·frame`) spins clockwise (-10°/frame,
//!   i.e. negative angular speed). Its top lip widens from 6 → ~10.4 over
//!   the lifetime while the base stays at 5 — that's the slanted silhouette
//!   visible in the gif. Alpha fades in from 5 → 245 over the first ~24
//!   frames, holds, then fades out the last fifth of the lifetime.
//! * Inner 8-strip cylinder (bottom radius = top radius = 4.5,
//!   45° arc per quad) — the 45° arc slices the ring into 8 vertical
//!   strips, which read as the visible "light columns" on the gif. Height
//!   grows from 0 at 2.5/frame with a quadratic deceleration
//!   that returns it near zero by the end of the lifetime. Spins the
//!   opposite direction (+10°/frame). Alpha is a flat 160 with a
//!   fade-out from 2/3 of the lifetime onward.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 55.0;
const DURATION_S: f32 = DURATION_FRAMES / FRAMES_PER_SECOND;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

// Outer flared cone — bottom radius 5 (constant), top radius 6 (widens
// over the lifetime) growing at 0.08/frame. Height shrinks at -0.1/frame
// so the cone settles flatter as it spins out.
const OUTER_BOTTOM_RADIUS: f32 = 5.0;
const OUTER_TOP_RADIUS_INIT: f32 = 6.0;
const OUTER_TOP_RADIUS_SPEED: f32 = 0.08;
const OUTER_HEIGHT_INIT: f32 = 6.5;
const OUTER_HEIGHT_SPEED: f32 = -0.1;
const OUTER_SPIN_DEG_PER_FRAME: f32 = -10.0;
const OUTER_ALPHA_INIT: f32 = 5.0 / 255.0;
const OUTER_ALPHA_MAX: f32 = 245.0 / 255.0;
const OUTER_ALPHA_SPEED_PER_FRAME: f32 = 10.0 / 255.0;
const OUTER_FADE_OUT_AT: f32 = DURATION_FRAMES - DURATION_FRAMES / 5.0;

// Inner segmented cylinder — height swells then collapses; ring of 8 strips.
const INNER_RADIUS: f32 = 4.5;
const INNER_HEIGHT_SPEED: f32 = 2.5;
// Height deceleration: -(height_speed / duration) * 2.
const INNER_HEIGHT_ACCEL: f32 = -(INNER_HEIGHT_SPEED / DURATION_FRAMES) * 2.0;
const INNER_SPIN_DEG_PER_FRAME: f32 = 10.0;
const INNER_ALPHA: f32 = 160.0 / 255.0;
const INNER_FADE_OUT_AT: f32 = DURATION_FRAMES - DURATION_FRAMES / 3.0;
/// 45° arc per strip → 360 / 45 = 8 segments.
const INNER_SIDES: u32 = 8;
/// Outer ring is the smooth full cylinder — no arc subdivision, so it
/// uses the default ~15° step, i.e. 24 segments.
const OUTER_SIDES: u32 = 24;

fn alpha_fade_out(frame: f32, peak: f32, fade_out_at: f32) -> f32 {
    if frame < fade_out_at {
        peak
    } else {
        let t = ((frame - fade_out_at) / (DURATION_FRAMES - fade_out_at)).clamp(0.0, 1.0);
        peak * (1.0 - t)
    }
}

fn outer_alpha(frame: f32) -> f32 {
    let fade_in_target = (OUTER_ALPHA_INIT + OUTER_ALPHA_SPEED_PER_FRAME * frame)
        .min(OUTER_ALPHA_MAX);
    alpha_fade_out(frame, fade_in_target, OUTER_FADE_OUT_AT)
}

fn inner_height(frame: f32) -> f32 {
    INNER_HEIGHT_SPEED * frame + INNER_HEIGHT_ACCEL * frame * (frame + 1.0) / 2.0
}

pub struct EntryEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl EntryEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age: 0.0 }
    }

    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, DURATION_FRAMES)
    }
}

impl Effect for EntryEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();

        // Outer flared cone — bottom narrow + top widening over time, spin CCW.
        let outer_height = (OUTER_HEIGHT_INIT + OUTER_HEIGHT_SPEED * frame).max(0.0);
        let outer_top = OUTER_TOP_RADIUS_INIT + OUTER_TOP_RADIUS_SPEED * frame;
        if outer_height > 0.0 {
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: OUTER_BOTTOM_RADIUS,
                top_size: outer_top,
                height: outer_height,
                sides: OUTER_SIDES,
                rotation: (frame * OUTER_SPIN_DEG_PER_FRAME).to_radians(),
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, outer_alpha(frame)],
                blend: BlendKind::Additive,
            });
        }

        // Inner 8-strip cylinder — height swells then collapses, spin CW.
        let inner_height_val = inner_height(frame).max(0.0);
        if inner_height_val > 0.0 {
            let inner_alpha = alpha_fade_out(frame, INNER_ALPHA, INNER_FADE_OUT_AT);
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: INNER_RADIUS,
                top_size: INNER_RADIUS,
                height: inner_height_val,
                sides: INNER_SIDES,
                rotation: (frame * INNER_SPIN_DEG_PER_FRAME).to_radians(),
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, inner_alpha],
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

    fn step(effect: &mut EntryEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None });
    }

    fn draws(effect: &EntryEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_two_cylinders_with_opposing_spin_and_8strip_inner() {
        // Sociable test: cover the two-cylinder layout, the segmented
        // inner ring (sides=8 from the 45° arc per strip), the
        // additive blend, the opposite rotation directions, and the
        // outer cone's flare shape (bottom narrower than top — the
        // chalice silhouette that produces the slanted sides visible in
        // the reference gif).
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 10.0 / FRAMES_PER_SECOND);
        let prims = draws(&e);
        assert_eq!(prims.len(), 2);

        let (
            outer_bottom,
            outer_top,
            outer_sides,
            outer_rot,
            inner_bottom,
            inner_top,
            inner_sides,
            inner_rot,
        ) = match (&prims[0], &prims[1]) {
            (
                EffectPrimitiveDraw::Cylinder {
                    bottom_size: b0,
                    top_size: t0,
                    sides: s0,
                    rotation: r0,
                    blend: bl0,
                    ..
                },
                EffectPrimitiveDraw::Cylinder {
                    bottom_size: b1,
                    top_size: t1,
                    sides: s1,
                    rotation: r1,
                    blend: bl1,
                    ..
                },
            ) => {
                assert_eq!(*bl0, BlendKind::Additive);
                assert_eq!(*bl1, BlendKind::Additive);
                (*b0, *t0, *s0, *r0, *b1, *t1, *s1, *r1)
            }
            _ => panic!("expected two Cylinder prims"),
        };
        assert_eq!(outer_sides, OUTER_SIDES);
        assert_eq!(inner_sides, 8, "arcAngle=45 → 8 vertical strips");
        assert!(outer_rot < 0.0, "outer spins CCW (longSpeed=-10)");
        assert!(inner_rot > 0.0, "inner spins CW (longSpeed=+10)");
        assert!(
            outer_top > outer_bottom,
            "outer is a flared chalice (top wider than bottom): {outer_bottom} → {outer_top}",
        );
        assert!(
            (inner_top - inner_bottom).abs() < 1e-4,
            "inner is a true cylinder (top == bottom)"
        );
    }

    #[test]
    fn outer_top_widens_over_lifetime() {
        // Sociable test: the outer cone's top radius grows at 0.08/frame —
        // it must grow monotonically from frame 0 → end,
        // while the bottom stays pinned at 5.
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 0.0);
        let (b0, t0) = match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder { bottom_size, top_size, .. } => {
                (*bottom_size, *top_size)
            }
            _ => unreachable!(),
        };
        step(&mut e, DURATION_S * 0.8);
        let (b1, t1) = match draws(&e).first() {
            Some(EffectPrimitiveDraw::Cylinder { bottom_size, top_size, .. }) => {
                (*bottom_size, *top_size)
            }
            _ => panic!("outer cone disappeared too early"),
        };
        assert!((b1 - b0).abs() < 1e-4, "bottom stays at innerSize=5");
        assert!(t1 > t0, "top widens with outerSpeed");
    }

    #[test]
    fn inner_height_swells_then_returns_near_zero() {
        // Sociable test: integrated height-speed + deceleration formula —
        // mid-life height is positive (swell) and end-of-life returns
        // back near zero (collapse). Locks the quadratic shape without
        // pinning exact values that drift with framerate.
        let mut e = EntryEffect::new([0.0; 3]);
        // One tick in (frame ≈ 1), inner height has barely grown but is
        // emitted; capture as the "early" sample.
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        let h_early = match draws(&e).get(1) {
            Some(EffectPrimitiveDraw::Cylinder { height, .. }) => *height,
            _ => panic!("inner cylinder expected at frame 1"),
        };

        step(&mut e, DURATION_S * 0.5);
        let h_mid = match &draws(&e)[1] {
            EffectPrimitiveDraw::Cylinder { height, .. } => *height,
            _ => unreachable!(),
        };
        assert!(h_mid > h_early + 1.0, "height grows by mid-life");

        // Walk to near the end of the lifetime; height returns toward 0.
        step(&mut e, DURATION_S * 0.49);
        let h_late = match draws(&e).get(1) {
            Some(EffectPrimitiveDraw::Cylinder { height, .. }) => *height,
            // height may have hit zero — primitive is then skipped entirely
            None => 0.0,
            _ => unreachable!(),
        };
        assert!(h_late < h_mid, "height collapses by end of life");
    }

    #[test]
    fn outer_alpha_fades_in_then_out() {
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 0.0);
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        // ~30 frames in — past the alpha fade-in but before the fade-out window.
        step(&mut e, 30.0 / FRAMES_PER_SECOND);
        let a_peak = match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_peak > a0, "alpha rises during fade-in");
        // Deep into the fade-out window.
        step(&mut e, (DURATION_FRAMES - 30.0 - 2.0) / FRAMES_PER_SECOND);
        let a_late = match draws(&e).first() {
            Some(EffectPrimitiveDraw::Cylinder { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(a_late < a_peak, "alpha drops during fade-out");
    }

    #[test]
    fn dies_after_duration() {
        let mut e = EntryEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < DURATION_S * 2.0 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
