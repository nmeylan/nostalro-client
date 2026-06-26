//! EF_WARP — yellow shockwave ring portal. Reference: original game
//!
//! Behaviour mirrors the original:
//!   * the parent emits one ring every 20 frames while it lives
//!     (80-frame lifetime → spawns at frames 0, 20, 40, 60 → **4 rings**);
//!   * each ring lives 80 frames on its own, so the last one is still
//!     fading at frame 140 even after the parent has died;
//!   * one ring grows from radius 2 with
//!     speed 1.25 / frame and
//!     deceleration `-(speed / duration) / 2`,
//!     reaching ~77 by frame 80;
//!   * a thickness cap of 10 — until the radius
//!     exceeds it the ring is a filled disc;
//!   * the `ring_yellow.tga` texture tiles 4× around the circumference
//!     (0.25 per step, wrapping at 1.0), with `v = 0` outer, `v = 1`
//!     inner;
//!   * alpha holds at 200/255 then fades linearly from
//!     mid-duration to zero.
//!
//! The orbiting sparks the original game also emits remain out of
//! scope; they'll come back when a particle primitive lands.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const WARP_TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[WARP_TEXTURE];

/// Native engine framerate — the original game ticks at the
/// renderer's frame rate, which the official client locks to 60 fps. Mapping
/// 80 ring-frames to 1.33 s wall-clock matches the
/// `ro-effects/effects/imgs/0-50/8.gif` reference timing.
const FRAMES_PER_SECOND: f32 = 60.0;
/// Lifetime of one ring.
const RING_DURATION_FRAMES: f32 = 80.0;
const RING_DURATION_S: f32 = RING_DURATION_FRAMES / FRAMES_PER_SECOND;
/// Parent lifetime — keeps spawning new rings while still alive.
const PARENT_DURATION_FRAMES: f32 = 80.0;
const PARENT_DURATION_S: f32 = PARENT_DURATION_FRAMES / FRAMES_PER_SECOND;
/// Interval between successive ring spawns (every 20 frames).
const SPAWN_INTERVAL_FRAMES: f32 = 20.0;
const SPAWN_INTERVAL_S: f32 = SPAWN_INTERVAL_FRAMES / FRAMES_PER_SECOND;
/// Total wall-clock lifetime: last ring spawns near the parent's death and
/// then lives one full ring-duration on its own.
pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + RING_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

const INITIAL_RADIUS: f32 = 2.0;
const RADIUS_SPEED_PER_FRAME: f32 = 1.25;
const RADIUS_ACCEL_PER_FRAME2: f32 = -(RADIUS_SPEED_PER_FRAME / RING_DURATION_FRAMES) / 2.0;
const RING_THICKNESS: f32 = 10.0;
const PEAK_ALPHA: f32 = 200.0 / 255.0;
const FADE_START_FRAMES: f32 = RING_DURATION_FRAMES / 2.0;
const UV_REPEAT: f32 = 4.0;

#[derive(Clone, Copy)]
struct Ring {
    /// Seconds elapsed since this ring spawned.
    age: f32,
}

impl Ring {
    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, RING_DURATION_FRAMES)
    }

    fn alive(&self) -> bool {
        self.age < RING_DURATION_S
    }

    fn outer_radius(&self) -> f32 {
        let n = self.frame();
        INITIAL_RADIUS + n * RADIUS_SPEED_PER_FRAME + RADIUS_ACCEL_PER_FRAME2 * n * (n + 1.0) / 2.0
    }

    fn alpha(&self) -> f32 {
        let n = self.frame();
        if n <= FADE_START_FRAMES {
            PEAK_ALPHA
        } else {
            let fade = ((n - FADE_START_FRAMES) / (RING_DURATION_FRAMES - FADE_START_FRAMES))
                .clamp(0.0, 1.0);
            PEAK_ALPHA * (1.0 - fade)
        }
    }
}

pub struct WarpEffect {
    world_pos: [f32; 3],
    /// Wall-clock age of the parent emitter.
    parent_age: f32,
    /// Time accumulator for the next ring spawn.
    next_spawn_at: f32,
    rings: Vec<Ring>,
}

impl WarpEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        // First ring spawns at parent_age = 0 (frame 0).
        Self {
            world_pos,
            parent_age: 0.0,
            next_spawn_at: 0.0,
            rings: Vec::with_capacity(4),
        }
    }
}

impl Effect for WarpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta;
        self.parent_age += dt;
        for ring in &mut self.rings {
            ring.age += dt;
        }

        // Catch up every scheduled spawn that landed during the elapsed
        // window — one every 20 frames. The
        // ring's initial age compensates for the spawn happening at a frame
        // boundary that's slightly behind `parent_age`.
        while self.next_spawn_at <= PARENT_DURATION_S && self.next_spawn_at <= self.parent_age {
            let initial_age = (self.parent_age - self.next_spawn_at).max(0.0);
            self.rings.push(Ring { age: initial_age });
            self.next_spawn_at += SPAWN_INTERVAL_S;
        }

        self.rings.retain(|r| r.alive());

        if self.parent_age >= PARENT_DURATION_S && self.rings.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for ring in &self.rings {
            let outer = ring.outer_radius();
            if outer <= 0.0 {
                continue;
            }
            let thickness = outer.min(RING_THICKNESS);
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: outer,
                thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: UV_REPEAT,
                texture: WARP_TEXTURE,
                color: [1.0, 1.0, 1.0, ring.alpha()],
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

    fn draws(effect: &WarpEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut WarpEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn first_ring_spawns_immediately() {
        let mut warp = WarpEffect::new([0.0, 0.0, 0.0]);
        step(&mut warp, 0.0);
        assert_eq!(draws(&warp).len(), 1);
    }

    #[test]
    fn ring_emits_full_tiled_disc() {
        let mut warp = WarpEffect::new([0.0, 0.0, 0.0]);
        step(&mut warp, 0.0);
        match draws(&warp).remove(0) {
            EffectPrimitiveDraw::GroundDisc {
                arc_angle_deg,
                uv_repeat,
                ..
            } => {
                assert!((arc_angle_deg - 360.0).abs() < f32::EPSILON);
                assert!((uv_repeat - 4.0).abs() < f32::EPSILON);
            }
            other => panic!("expected GroundDisc, got {other:?}"),
        }
    }

    #[test]
    fn spawns_four_rings_over_parent_lifetime() {
        let mut warp = WarpEffect::new([0.0, 0.0, 0.0]);
        // First spawn at t=0.
        step(&mut warp, 0.0);
        assert_eq!(draws(&warp).len(), 1, "1st ring at frame 0");

        // Just past 20-frame mark (~0.67 s) → 2nd ring spawns.
        step(&mut warp, SPAWN_INTERVAL_S + 0.01);
        assert_eq!(draws(&warp).len(), 2, "2nd ring after 20 frames");

        step(&mut warp, SPAWN_INTERVAL_S);
        assert_eq!(draws(&warp).len(), 3, "3rd ring after 40 frames");

        step(&mut warp, SPAWN_INTERVAL_S);
        assert_eq!(draws(&warp).len(), 4, "4th ring after 60 frames");

        // Past parent death (frame 80) → no more spawns even as rings age.
        step(&mut warp, SPAWN_INTERVAL_S);
        assert!(
            draws(&warp).len() <= 4,
            "parent stops spawning past frame 80"
        );
    }

    #[test]
    fn ring_radius_grows_then_fades() {
        let mut warp = WarpEffect::new([0.0, 0.0, 0.0]);
        step(&mut warp, 0.0);
        let (r0, a0) = match &draws(&warp)[0] {
            EffectPrimitiveDraw::GroundDisc { radius, color, .. } => (*radius, color[3]),
            _ => unreachable!(),
        };

        // Halfway through ring life — radius bigger, alpha still at peak.
        step(&mut warp, RING_DURATION_S * 0.45);
        let (r_mid, a_mid) = match &draws(&warp)[0] {
            EffectPrimitiveDraw::GroundDisc { radius, color, .. } => (*radius, color[3]),
            _ => unreachable!(),
        };
        assert!(r_mid > r0);
        assert!((a_mid - PEAK_ALPHA).abs() < 1e-4);

        // Deep into fade-out — alpha drops.
        step(&mut warp, RING_DURATION_S * 0.5);
        let a_late = match &draws(&warp)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_late < a0);
    }

    #[test]
    fn early_ring_is_filled_disc_late_ring_is_band() {
        let mut warp = WarpEffect::new([0.0, 0.0, 0.0]);
        step(&mut warp, 0.0);
        match &draws(&warp)[0] {
            EffectPrimitiveDraw::GroundDisc {
                radius, thickness, ..
            } => {
                assert!(
                    *thickness >= *radius - f32::EPSILON,
                    "early ring fills its disc (r={radius} t={thickness})"
                );
            }
            _ => unreachable!(),
        }
        // Age past the moment radius crosses 10 → band cap engages.
        step(&mut warp, RING_DURATION_S * 0.5);
        match &draws(&warp)[0] {
            EffectPrimitiveDraw::GroundDisc { thickness, .. } => {
                assert!(
                    (*thickness - RING_THICKNESS).abs() < f32::EPSILON,
                    "thickness caps at innerSize (got {thickness})"
                );
            }
            _ => unreachable!(),
        }
    }
}
