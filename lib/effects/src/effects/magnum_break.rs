//! EF_MAGNUMBREAK — yellow ground shockwave + spherical explosion.
//!
//! The parent emitter launches three primitives at frame 0:
//!   * a ground ring tied to the parent's lifetime — `ring_yellow.tga`,
//!     radius speed 1.75, decel, thickness 12, alpha grows to peak
//!     in 15 frames then holds and fades the last 15;
//!   * a sphere (the explosion) — `bigbang.tga`, radius speed 1.15,
//!     same alpha curve capped at 180/255, slow texture rotation
//!     (3°/frame);
//!   * a second ground ring with a hardcoded 30-frame lifetime — the small
//!     "after-ring" that snaps into existence as the main ring is still growing.
//!
//! The explosion is a full UV sphere centred at the impact point. Its lower
//! hemisphere sits below the ground plane and is hidden by the depth test
//! against ground geometry — what reaches the screen reads as a dome
//! bursting upward, matching the original-game silhouette.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_yellow.tga";
pub const EXPLOSION_TEXTURE: &str = "bigbang.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE, EXPLOSION_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Parent lifetime. Matches the gif at `0-50/17.gif` (~10 capture frames at
/// 7 cs each ≈ 700 ms ≈ 42 frames at 60 fps).
const PARENT_DURATION_FRAMES: f32 = 42.0;
const PARENT_DURATION_S: f32 = PARENT_DURATION_FRAMES / FRAMES_PER_SECOND;
/// Hardcoded second-ring duration.
const SECOND_RING_DURATION_FRAMES: f32 = 30.0;
const SECOND_RING_DURATION_S: f32 = SECOND_RING_DURATION_FRAMES / FRAMES_PER_SECOND;

/// Wall-clock total: every sub-primitive spawns at frame 0; the parent-bound
/// ones live the parent's duration, the second ring lives 30 frames. The
/// longest of those is the parent.
pub const TOTAL_DURATION_MS: u32 =
    (PARENT_DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

// Ring (parent-bound) — numbers verbatim.
const RING_INITIAL_RADIUS: f32 = 2.0;
const RING_RADIUS_SPEED_PER_FRAME: f32 = 1.75;
const RING_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(RING_RADIUS_SPEED_PER_FRAME / PARENT_DURATION_FRAMES) / 2.0;
const RING_THICKNESS: f32 = 12.0;
const RING_PEAK_ALPHA: f32 = 1.0;
const FADE_IN_FRAMES: f32 = 15.0;
const RING_FADE_OUT_FRAMES: f32 = PARENT_DURATION_FRAMES - 15.0;
const RING_UV_REPEAT: f32 = 4.0;

// Explosion sphere — numbers verbatim.
const EXPLOSION_INITIAL_RADIUS: f32 = 2.0;
const EXPLOSION_RADIUS_SPEED_PER_FRAME: f32 = 1.15;
const EXPLOSION_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(EXPLOSION_RADIUS_SPEED_PER_FRAME / PARENT_DURATION_FRAMES) / 2.0;
const EXPLOSION_PEAK_ALPHA: f32 = 180.0 / 255.0;
/// Texture rotation in degrees per frame.
const EXPLOSION_ROT_DEG_PER_FRAME: f32 = 3.0;
/// Latitude segments — 36° arc → 180/36 = 5.
const EXPLOSION_SIDES_LAT: u32 = 5;
/// Longitude segments — 36° arc → 360/36 = 10.
const EXPLOSION_SIDES_LON: u32 = 10;
/// Fraction of the sphere's radius the centre is sunk below `world_pos`.
/// Native RO uses `-Y = up`, so sinking means adding a positive Y. Keeps
/// the visible silhouette dome-shaped even when `world_pos.y` doesn't align
/// with true ground level or the camera angle would otherwise expose the
/// lower hemisphere. `0.5` → equator sits at the impact-point plane.
const EXPLOSION_SINK_FRAC: f32 = 0.5;

// Second (hardcoded-30-frame) ring — same params as the parent ring but with
// a shorter lifetime.
const SECOND_RING_FADE_OUT_FRAMES: f32 = SECOND_RING_DURATION_FRAMES - 15.0;
const SECOND_RING_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(RING_RADIUS_SPEED_PER_FRAME / SECOND_RING_DURATION_FRAMES) / 2.0;

/// Linear fade-in to `peak` over `FADE_IN_FRAMES`, hold, then linear fade-out
/// from `fade_out_at` to `duration`.
fn alpha_curve(frame: f32, peak: f32, fade_out_at: f32, duration: f32) -> f32 {
    if frame <= FADE_IN_FRAMES {
        peak * (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
    } else if frame >= fade_out_at {
        let fade = ((frame - fade_out_at) / (duration - fade_out_at)).clamp(0.0, 1.0);
        peak * (1.0 - fade)
    } else {
        peak
    }
}

fn radius_at(initial: f32, speed: f32, accel: f32, frame: f32) -> f32 {
    initial + speed * frame + accel * frame * (frame + 1.0) / 2.0
}

pub struct MagnumBreakEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl MagnumBreakEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn parent_frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, PARENT_DURATION_FRAMES)
    }

    fn second_ring_frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, SECOND_RING_DURATION_FRAMES)
    }
}

impl Effect for MagnumBreakEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= PARENT_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let parent_frame = self.parent_frame();

        // -- Parent ring --
        let ring_outer = radius_at(
            RING_INITIAL_RADIUS,
            RING_RADIUS_SPEED_PER_FRAME,
            RING_RADIUS_ACCEL_PER_FRAME2,
            parent_frame,
        );
        if ring_outer > 0.0 {
            let ring_alpha = alpha_curve(
                parent_frame,
                RING_PEAK_ALPHA,
                RING_FADE_OUT_FRAMES,
                PARENT_DURATION_FRAMES,
            );
            let thickness = ring_outer.min(RING_THICKNESS);
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: ring_outer,
                thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: RING_UV_REPEAT,
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, ring_alpha],
                // The Magnum Break ring/sphere render
                // alpha-blended, not additive
                // — additive vanishes against a bright lightmap.
                blend: BlendKind::Alpha,
            });
        }

        // -- Explosion sphere --
        let explosion_radius = radius_at(
            EXPLOSION_INITIAL_RADIUS,
            EXPLOSION_RADIUS_SPEED_PER_FRAME,
            EXPLOSION_RADIUS_ACCEL_PER_FRAME2,
            parent_frame,
        );
        if explosion_radius > 0.0 {
            let explosion_alpha = alpha_curve(
                parent_frame,
                EXPLOSION_PEAK_ALPHA,
                RING_FADE_OUT_FRAMES,
                PARENT_DURATION_FRAMES,
            );
            let longitude_offset_rad =
                (parent_frame * EXPLOSION_ROT_DEG_PER_FRAME).to_radians();
            let sphere_center = [
                self.world_pos[0],
                self.world_pos[1] + explosion_radius * EXPLOSION_SINK_FRAC,
                self.world_pos[2],
            ];
            out.push(EffectPrimitiveDraw::Sphere {
                center: sphere_center,
                radius: explosion_radius,
                sides_lat: EXPLOSION_SIDES_LAT,
                sides_lon: EXPLOSION_SIDES_LON,
                longitude_offset: longitude_offset_rad,
                longitude_arc: std::f32::consts::TAU,
                uv_repeat: [1.0, 1.0],
                texture: EXPLOSION_TEXTURE,
                color: [1.0, 1.0, 1.0, explosion_alpha],
                blend: BlendKind::Alpha,
            });
        }

        // -- Second ring (30-frame hardcoded life) --
        if self.age < SECOND_RING_DURATION_S {
            let second_frame = self.second_ring_frame();
            let second_outer = radius_at(
                RING_INITIAL_RADIUS,
                RING_RADIUS_SPEED_PER_FRAME,
                SECOND_RING_RADIUS_ACCEL_PER_FRAME2,
                second_frame,
            );
            if second_outer > 0.0 {
                let second_alpha = alpha_curve(
                    second_frame,
                    RING_PEAK_ALPHA,
                    SECOND_RING_FADE_OUT_FRAMES,
                    SECOND_RING_DURATION_FRAMES,
                );
                let thickness = second_outer.min(RING_THICKNESS);
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center: self.world_pos,
                    radius: second_outer,
                    thickness,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: RING_UV_REPEAT,
                    texture: RING_TEXTURE,
                    color: [1.0, 1.0, 1.0, second_alpha],
                    blend: BlendKind::Alpha,
                });
            }
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

    fn draws(effect: &MagnumBreakEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut MagnumBreakEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None });
    }

    #[test]
    fn emits_three_primitives_at_start() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 0.0);
        let prims = draws(&mb);
        assert_eq!(prims.len(), 3, "ring + sphere + second ring");
        assert!(matches!(prims[0], EffectPrimitiveDraw::GroundDisc { blend: BlendKind::Alpha, .. }));
        assert!(matches!(prims[1], EffectPrimitiveDraw::Sphere { blend: BlendKind::Alpha, .. }));
        assert!(matches!(prims[2], EffectPrimitiveDraw::GroundDisc { blend: BlendKind::Alpha, .. }));
    }

    #[test]
    fn ring_and_sphere_grow_together() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 0.0);
        let (r0, s0) = match (&draws(&mb)[0], &draws(&mb)[1]) {
            (
                EffectPrimitiveDraw::GroundDisc { radius, .. },
                EffectPrimitiveDraw::Sphere { radius: sr, .. },
            ) => (*radius, *sr),
            _ => unreachable!(),
        };
        // Halfway into parent life.
        step(&mut mb, PARENT_DURATION_S * 0.5);
        let (r_mid, s_mid) = match (&draws(&mb)[0], &draws(&mb)[1]) {
            (
                EffectPrimitiveDraw::GroundDisc { radius, .. },
                EffectPrimitiveDraw::Sphere { radius: sr, .. },
            ) => (*radius, *sr),
            _ => unreachable!(),
        };
        assert!(r_mid > r0);
        assert!(s_mid > s0);
    }

    #[test]
    fn sphere_longitude_offset_advances_with_time() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 0.0);
        let off0 = match &draws(&mb)[1] {
            EffectPrimitiveDraw::Sphere {
                longitude_offset, ..
            } => *longitude_offset,
            _ => unreachable!(),
        };
        step(&mut mb, 10.0 / FRAMES_PER_SECOND);
        let off1 = match &draws(&mb)[1] {
            EffectPrimitiveDraw::Sphere {
                longitude_offset, ..
            } => *longitude_offset,
            _ => unreachable!(),
        };
        assert!(off1 > off0, "longitude_offset advances each frame");
    }

    #[test]
    fn second_ring_disappears_after_30_frames() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 0.0);
        assert_eq!(draws(&mb).len(), 3);
        // Past 30 frames the hardcoded second ring should be gone.
        step(&mut mb, SECOND_RING_DURATION_S + 0.01);
        assert_eq!(draws(&mb).len(), 2, "second ring expired");
    }

    #[test]
    fn alpha_fades_in_then_out() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 0.0);
        let a0 = match &draws(&mb)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        // Past fade-in (frame ~16).
        step(&mut mb, (FADE_IN_FRAMES + 1.0) / FRAMES_PER_SECOND);
        let a_peak = match &draws(&mb)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_peak > a0, "alpha grows during fade-in");
        // Deep into fade-out window.
        step(&mut mb, PARENT_DURATION_S * 0.6);
        let a_late = match &draws(&mb)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_late < a_peak, "alpha drops during fade-out");
    }

    #[test]
    fn dies_after_parent_duration() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < PARENT_DURATION_S * 2.0 {
            status = mb.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
