//! EF_REVIVE — yellow rings expanding around a revived player.
//!
//! Reference: `ro-effects/effects/imgs/100-150/140.gif`.
//!
//! Spawns one `Cylinder` ring every 25 frames at the caster. The very first
//! spawn is the "hero" ring that holds full size for its entire 60-frame
//! lifetime; later spawns shrink inward as they age, so the burst reads as a
//! steady leading ring trailed by contracting echoes.
//!
//! Hero ring (frame 0):
//!   * outer 11.0, inner 7.5, height 7.5
//!   * alpha 240/255
//!   * spin 1.5°/frame (no acceleration)
//!   * inner/outer speeds = 0 → ring keeps its shape
//!   * lifetime is long in the source; we use 60 like every other ring
//!     spawn, since the hero ring is visually identical except it doesn't
//!     shrink
//!   * fades only at the end (last tenth of its life)
//!
//! Follow-up rings (frame 25, 50, …):
//!   * outer 10.5, inner 7.5, height 7.5
//!   * alpha 240/255
//!   * spin 1.5°/frame with +0.03 accel → spin accelerates
//!   * inner/outer speed -0.12 with +0.003 accel → ring shrinks then
//!     decelerates
//!   * 60-frame lifetime, fades over the second half
//!
//! Both ring types use `ring_yellow.tga`.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SIDES: u32 = 24;

const SPAWN_INTERVAL_FRAMES: f32 = 25.0;
const RING_LIFETIME_FRAMES: f32 = 60.0;
const RING_HEIGHT: f32 = 7.5;
const ALPHA_MAX: f32 = 240.0 / 255.0;

const HERO_OUTER: f32 = 11.0;
const HERO_INNER: f32 = 7.5;
const HERO_LONG_SPEED_DEG: f32 = 1.5;

const FOLLOWUP_OUTER: f32 = 10.5;
const FOLLOWUP_INNER: f32 = 7.5;
const FOLLOWUP_LONG_SPEED_DEG: f32 = 1.5;
const FOLLOWUP_LONG_ACCEL_DEG: f32 = 0.03;
const FOLLOWUP_INNER_SPEED: f32 = -0.12;
const FOLLOWUP_INNER_ACCEL: f32 = 0.003;
const FOLLOWUP_OUTER_SPEED: f32 = -0.12;
const FOLLOWUP_OUTER_ACCEL: f32 = 0.003;

/// Parent emitter lifetime — Revive's table duration is 2500 ms (~150 frames).
const PARENT_FRAMES: u32 = 150;
/// Total visible time = last spawn (at frame 125) + ring lifetime (60).
pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_FRAMES as f32 + RING_LIFETIME_FRAMES) * 1000.0 / FRAMES_PER_SECOND) as u32;

#[derive(Clone, Copy, Debug)]
struct Ring {
    spawn_frame: f32,
    outer0: f32,
    inner0: f32,
    long0_rad: f32,
    long_speed_per_frame: f32,
    long_accel_per_frame: f32,
    outer_speed: f32,
    outer_accel: f32,
    inner_speed: f32,
    inner_accel: f32,
    fade_out_at_frame: f32,
}

impl Ring {
    fn hero(spawn_frame: f32) -> Self {
        Self {
            spawn_frame,
            outer0: HERO_OUTER,
            inner0: HERO_INNER,
            long0_rad: 0.0,
            long_speed_per_frame: HERO_LONG_SPEED_DEG.to_radians(),
            long_accel_per_frame: 0.0,
            outer_speed: 0.0,
            outer_accel: 0.0,
            inner_speed: 0.0,
            inner_accel: 0.0,
            fade_out_at_frame: RING_LIFETIME_FRAMES - RING_LIFETIME_FRAMES / 10.0,
        }
    }

    fn followup(spawn_frame: f32) -> Self {
        Self {
            spawn_frame,
            outer0: FOLLOWUP_OUTER,
            inner0: FOLLOWUP_INNER,
            long0_rad: 0.0,
            long_speed_per_frame: FOLLOWUP_LONG_SPEED_DEG.to_radians(),
            long_accel_per_frame: FOLLOWUP_LONG_ACCEL_DEG.to_radians(),
            outer_speed: FOLLOWUP_OUTER_SPEED,
            outer_accel: FOLLOWUP_OUTER_ACCEL,
            inner_speed: FOLLOWUP_INNER_SPEED,
            inner_accel: FOLLOWUP_INNER_ACCEL,
            fade_out_at_frame: RING_LIFETIME_FRAMES / 2.0,
        }
    }

    fn alive_at(&self, parent_frame: f32) -> Option<f32> {
        let local = parent_frame - self.spawn_frame;
        if local < 0.0 || local >= RING_LIFETIME_FRAMES {
            None
        } else {
            Some(local)
        }
    }

    /// Discrete integration of `speed += accel; size += speed` for `frames`
    /// steps starting from `size0` / `speed0`. Closed form:
    /// `size = size0 + speed0 * f + accel * f * (f + 1) / 2`.
    fn integrate(size0: f32, speed: f32, accel: f32, frame: f32) -> f32 {
        size0 + speed * frame + accel * frame * (frame + 1.0) * 0.5
    }

    fn outer(&self, local: f32) -> f32 {
        Self::integrate(self.outer0, self.outer_speed, self.outer_accel, local).max(0.0)
    }
    fn inner(&self, local: f32) -> f32 {
        Self::integrate(self.inner0, self.inner_speed, self.inner_accel, local).max(0.0)
    }
    fn long_rad(&self, local: f32) -> f32 {
        Self::integrate(
            self.long0_rad,
            self.long_speed_per_frame,
            self.long_accel_per_frame,
            local,
        )
    }
    fn alpha(&self, local: f32) -> f32 {
        if local < self.fade_out_at_frame {
            ALPHA_MAX
        } else {
            let span = (RING_LIFETIME_FRAMES - self.fade_out_at_frame).max(1.0);
            (ALPHA_MAX * (RING_LIFETIME_FRAMES - local) / span).max(0.0)
        }
    }
}

pub struct ReviveEffect {
    world_pos: [f32; 3],
    age: f32,
    rings: Vec<Ring>,
    next_spawn_frame: f32,
}

impl ReviveEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
            rings: vec![Ring::hero(0.0)],
            next_spawn_frame: SPAWN_INTERVAL_FRAMES,
        }
    }
}

impl Effect for ReviveEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let frame = self.age * FRAMES_PER_SECOND;

        while frame >= self.next_spawn_frame && self.next_spawn_frame < PARENT_FRAMES as f32 {
            self.rings.push(Ring::followup(self.next_spawn_frame));
            self.next_spawn_frame += SPAWN_INTERVAL_FRAMES;
        }
        // Drop rings that have aged out so the Vec stays bounded.
        self.rings.retain(|r| r.alive_at(frame).is_some());

        if frame >= PARENT_FRAMES as f32 + RING_LIFETIME_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        for ring in &self.rings {
            let Some(local) = ring.alive_at(frame) else {
                continue;
            };
            let alpha = ring.alpha(local);
            if alpha <= 0.0 {
                continue;
            }
            let outer = ring.outer(local);
            let inner = ring.inner(local);
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: inner,
                top_size: outer,
                height: RING_HEIGHT,
                sides: SIDES,
                rotation: ring.long_rad(local),
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut ReviveEffect, dt: f32) {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None, caster_yaw: None,
        });
    }

    fn cylinders(e: &ReviveEffect) -> Vec<(f32, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Cylinder {
                    bottom_size,
                    top_size,
                    ..
                } => Some((*bottom_size, *top_size)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn spawns_hero_then_followups_at_25_frame_cadence() {
        // Sociable test: covers update spawning logic + draw emission.
        let mut e = ReviveEffect::new([0.0; 3]);
        // Frame 0: only the hero ring.
        step(&mut e, 0.0);
        let c0 = cylinders(&e);
        assert_eq!(c0.len(), 1);
        // Hero ring uses HERO_OUTER (11.0); the inner is HERO_INNER (7.5).
        assert!((c0[0].1 - HERO_OUTER).abs() < 1e-3);

        // Frame ~26 → second ring spawned, first still alive.
        step(&mut e, 26.0 / FRAMES_PER_SECOND);
        let c1 = cylinders(&e);
        assert_eq!(c1.len(), 2);

        // Frame ~76 → fourth ring spawned, hero just expiring.
        step(&mut e, 50.0 / FRAMES_PER_SECOND);
        let rings_at_76 = cylinders(&e).len();
        assert!(
            (3..=4).contains(&rings_at_76),
            "expected 3-4 alive rings at frame ~76, got {rings_at_76}"
        );
    }

    #[test]
    fn followup_rings_shrink_over_time() {
        let mut e = ReviveEffect::new([0.0; 3]);
        // Advance past the hero ring's slot so only followups remain.
        step(&mut e, (RING_LIFETIME_FRAMES + 5.0) / FRAMES_PER_SECOND);
        let outers_early: Vec<f32> = cylinders(&e).iter().map(|(_, o)| *o).collect();
        step(&mut e, 20.0 / FRAMES_PER_SECOND);
        let outers_late: Vec<f32> = cylinders(&e).iter().map(|(_, o)| *o).collect();
        // The newest followup at frame ~85 is contracting.
        assert!(
            outers_late.iter().any(|&o| o < FOLLOWUP_OUTER),
            "followups should contract; saw outers {outers_late:?}"
        );
        let _ = outers_early; // covers integration path
    }

    #[test]
    fn dies_after_last_ring_finishes() {
        let mut e = ReviveEffect::new([0.0; 3]);
        let total_s = TOTAL_DURATION_MS as f32 / 1000.0;
        let s = e.update(&EffectUpdateCtx {
            delta: total_s + 0.5,
            camera_target: None, caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
