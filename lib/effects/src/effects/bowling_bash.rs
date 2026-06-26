//! EF_BOWLINGBASH — ground impact ring + two swept cylinder slashes.
//!
//! Recipe:
//!
//! Ground ring:
//! * texture `effect/ring_yellow.tga`, additive blend
//! * outer radius starts at 8.0 and grows by 0.7/frame
//! * deceleration `-(0.7 / 30) / 2 ≈ -0.0117 /frame²`
//! * peak alpha 45/255, fades after frame 35
//! * 50-frame visible lifetime
//!
//! Two cylinder slashes — one at parent frame 0, one at parent
//! frame 5, each `20 - count` frames long. The second slash is yawed
//! 100° relative to the first (yaw = base + count * 20°),
//! giving the characteristic two-blade sweep. Per-slash parameters:
//! * texture `effect/ring_blue.tga`
//! * outer radius `8 + 0.5 t - 0.015 t²` (bottom of the cone)
//! * inner radius `3 + 0.5 t - 0.015 t²` (top of the cone)
//! * height 3.5
//! * start alpha `(240 - count * 7) / 255`
//! * fade-out begins at duration/2
//!
//! `TOTAL_DURATION_MS` is the table's 2500 ms; both sub-primitives are
//! long-dead by then, so the parent simply caps the effect's lifetime.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_yellow.tga";
pub const SLASH_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE, SLASH_TEXTURE];
// Back-compat with callers (mod.rs, tests) that referenced the original
// single TEXTURE constant.
pub const TEXTURE: &str = RING_TEXTURE;

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 2_500;
const TOTAL_DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;

const RING_LIFE_FRAMES: f32 = 50.0;

const INITIAL_RADIUS: f32 = 8.0;
const RADIUS_SPEED_PER_FRAME: f32 = 0.7;
const RADIUS_ACCEL_PER_FRAME2: f32 = -(RADIUS_SPEED_PER_FRAME / 30.0) / 2.0;

const PEAK_ALPHA: f32 = 45.0 / 255.0;
const FADE_OUT_AT_FRAME: f32 = 35.0;

const THICKNESS: f32 = 4.0;
const UV_REPEAT: f32 = 4.0;

const SLASH_OUTER_INITIAL: f32 = 8.0;
const SLASH_INNER_INITIAL: f32 = 3.0;
const SLASH_RADIUS_SPEED_PER_FRAME: f32 = 0.5;
const SLASH_RADIUS_ACCEL_PER_FRAME2: f32 = -0.03;
const SLASH_HEIGHT: f32 = 3.5;
const SLASH_SIDES: u32 = 24;
/// Below this caster→target horizontal distance the trail anchor carries
/// no usable direction; fall back to a default facing.
const MIN_DIR_DISTANCE: f32 = 0.001;

/// Each slash's parameters fixed at spawn time, integrated forward as the
/// effect ages. Reproduces the two-launch pattern (frame 0
/// and frame 5) without an actual scheduler — the values live in the
/// struct so they can drive both `update`'s lifetime check and
/// `collect_draws`'s per-frame emission.
#[derive(Clone, Copy)]
struct SlashSpawn {
    /// Parent age at which this slash was launched.
    spawn_frame: f32,
    /// Per-slash lifetime (`20 - count`).
    life_frames: f32,
    /// Yaw around Y, radians.
    yaw_rad: f32,
    /// Peak alpha (`240 - count * 7`, normalised to 0..1).
    peak_alpha: f32,
}

impl SlashSpawn {
    fn alpha_at(&self, local_frame: f32) -> f32 {
        let fade_out = self.life_frames / 2.0;
        if local_frame < fade_out {
            self.peak_alpha
        } else {
            let t = ((local_frame - fade_out) / (self.life_frames - fade_out)).clamp(0.0, 1.0);
            self.peak_alpha * (1.0 - t)
        }
    }

    fn outer_at(local_frame: f32) -> f32 {
        SLASH_OUTER_INITIAL
            + SLASH_RADIUS_SPEED_PER_FRAME * local_frame
            + SLASH_RADIUS_ACCEL_PER_FRAME2 * local_frame * (local_frame + 1.0) / 2.0
    }

    fn inner_at(local_frame: f32) -> f32 {
        SLASH_INNER_INITIAL
            + SLASH_RADIUS_SPEED_PER_FRAME * local_frame
            + SLASH_RADIUS_ACCEL_PER_FRAME2 * local_frame * (local_frame + 1.0) / 2.0
    }
}

/// Build the two slash spawns aimed along `base_heading_rad`. One slash
/// launches at frame 0 and a second at frame 5, with the
/// second yawed 100° further around the swing (yaw = base +
/// count * 20°).
///
/// The renderer's `Cylinder` primitive flips Y rotation versus the source
/// brand-angle convention (same as `pierce.rs` and
/// `sonicblowhit.rs`), so we negate the heading to compensate.
fn make_slashes(base_heading_rad: f32) -> [SlashSpawn; 2] {
    let mk = |state_cnt: f32| SlashSpawn {
        spawn_frame: state_cnt,
        life_frames: 20.0 - state_cnt,
        yaw_rad: base_heading_rad + (state_cnt * 20.0).to_radians(),
        peak_alpha: (240.0 - state_cnt * 7.0) / 255.0,
    };
    [mk(0.0), mk(5.0)]
}

pub struct BowlingBashEffect {
    world_pos: [f32; 3],
    age: f32,
    slashes: [SlashSpawn; 2],
}

impl BowlingBashEffect {
    /// Endpoint convention: `from` = source (caster), `to` = target (the
    /// struck entity). The swing centres on the **target** and faces back
    /// toward the **source** (`to → from`). `to == from` (single-point anchor)
    /// keeps the slashes on a default facing (+Z).
    pub fn new_with_direction(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = from[0] - to[0];
        let dz = from[2] - to[2];
        let base_heading_rad = if (dx * dx + dz * dz).sqrt() > MIN_DIR_DISTANCE {
            dx.atan2(dz)
        } else {
            0.0
        };
        Self {
            world_pos: to,
            age: 0.0,
            slashes: make_slashes(base_heading_rad),
        }
    }

    pub fn new(world_pos: [f32; 3]) -> Self {
        Self::new_with_direction(world_pos, world_pos)
    }
}

fn radius_at(frame: f32) -> f32 {
    INITIAL_RADIUS
        + RADIUS_SPEED_PER_FRAME * frame
        + RADIUS_ACCEL_PER_FRAME2 * frame * (frame + 1.0) / 2.0
}

fn alpha_at(frame: f32) -> f32 {
    if frame <= FADE_OUT_AT_FRAME {
        PEAK_ALPHA
    } else {
        let fade =
            ((frame - FADE_OUT_AT_FRAME) / (RING_LIFE_FRAMES - FADE_OUT_AT_FRAME)).clamp(0.0, 1.0);
        PEAK_ALPHA * (1.0 - fade)
    }
}

impl Effect for BowlingBashEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;

        // Ground impact ring — lives 50 frames from spawn.
        if frame < RING_LIFE_FRAMES {
            let ring_frame = frame.clamp(0.0, RING_LIFE_FRAMES);
            let radius = radius_at(ring_frame).max(0.0);
            if radius > 0.0 {
                let alpha = alpha_at(ring_frame);
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center: self.world_pos,
                    radius,
                    thickness: THICKNESS,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: UV_REPEAT,
                    texture: RING_TEXTURE,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Additive,
                });
            }
        }

        // Two swept cylinder slashes — each lives `20 - count` frames
        // from its own spawn.
        for s in &self.slashes {
            let local = frame - s.spawn_frame;
            if local < 0.0 || local >= s.life_frames {
                continue;
            }
            let alpha = s.alpha_at(local);
            if alpha <= 0.0 {
                continue;
            }
            let outer = SlashSpawn::outer_at(local).max(0.0);
            let inner = SlashSpawn::inner_at(local).max(0.0);
            // Project convention (matches `revive.rs`, `teleportation.rs`):
            // `bottom_size = inner`, `top_size = outer` — the cylinder
            // flares outward toward the top, concave at the base, so the
            // shockwave reads as expanding upward instead of forming an
            // inverted dome.
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: inner,
                top_size: outer,
                height: SLASH_HEIGHT,
                sides: SLASH_SIDES,
                rotation: 0.0,
                tilt_x_rad: 0.0,
                rotation_y_rad: s.yaw_rad,
                uv_scroll: [0.0, 0.0],
                texture: SLASH_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
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

    fn draws(effect: &BowlingBashEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut BowlingBashEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None, caster_yaw: None,
        })
    }

    #[test]
    fn emits_ring_and_first_slash_at_spawn_then_expires() {
        // Sociable test: frame 0 emits both the ground ring and the
        // first cylinder slash; the second slash hasn't spawned yet.
        // After 50 frames everything has died and the draw list is empty.
        let mut eff = BowlingBashEffect::new([0.0, 0.0, 0.0]);
        step(&mut eff, 0.0);
        let prims = draws(&eff);
        assert_eq!(prims.len(), 2, "ground ring + first cylinder slash");
        let mut saw_ring = false;
        let mut saw_cylinder = false;
        for prim in &prims {
            match prim {
                EffectPrimitiveDraw::GroundDisc {
                    radius,
                    texture,
                    blend,
                    ..
                } => {
                    assert!((*radius - INITIAL_RADIUS).abs() < 1e-3);
                    assert_eq!(*texture, RING_TEXTURE);
                    assert_eq!(*blend, BlendKind::Additive);
                    saw_ring = true;
                }
                EffectPrimitiveDraw::Cylinder {
                    texture,
                    bottom_size,
                    top_size,
                    ..
                } => {
                    assert_eq!(*texture, SLASH_TEXTURE);
                    // Concave-at-the-base wave: top is wider than bottom
                    // (renderer convention shared with revive.rs).
                    assert!(top_size > bottom_size, "cylinder flares outward toward the top");
                    saw_cylinder = true;
                }
                _ => panic!("unexpected primitive {:?}", prim),
            }
        }
        assert!(saw_ring && saw_cylinder);
        step(&mut eff, RING_LIFE_FRAMES / FRAMES_PER_SECOND + 0.01);
        assert_eq!(draws(&eff).len(), 0, "everything expires by frame 50");
    }


    #[test]
    fn ring_grows_then_fade_begins_after_frame_35() {
        let mut eff = BowlingBashEffect::new([0.0; 3]);
        // Frame 10 — well before fade.
        step(&mut eff, 10.0 / FRAMES_PER_SECOND);
        let (r_early, a_early) = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { radius, color, .. } => (*radius, color[3]),
            _ => unreachable!(),
        };
        assert!(r_early > INITIAL_RADIUS, "ring grows");
        assert!((a_early - PEAK_ALPHA).abs() < 1e-6, "still at peak alpha");

        // Frame 45 — deep into fade.
        step(&mut eff, 35.0 / FRAMES_PER_SECOND);
        let a_late = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_late < PEAK_ALPHA, "fade-out engaged");
    }

    #[test]
    fn parent_dies_after_total_duration() {
        let mut eff = BowlingBashEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_S * 1.5 {
            status = step(&mut eff, 1.0 / 60.0);
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
