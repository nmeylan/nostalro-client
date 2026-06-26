//! `EF_DRAGONSMOKE` — chimney smoke column that rises from the source
//! and curves along a wind direction over its lifetime.
//!
//! The original game launches a single 3D particle
//! per call: random tilt + yaw + a `0.75`/frame roll-speed spin of
//! the direction matrix gives each puff a curving trajectory. The
//! `SprBurst` pipeline can't express any of that (no per-particle
//! direction matrix, no roll), so this id ships as a Custom trail
//! effect: the caster→target trail anchor's `from` is the chimney
//! source and `to - from` is the wind direction. Puffs spawn at the
//! source, rise upward (native RO `-Y = up`), and accelerate toward the
//! wind direction so the column reads as straight at the bottom and
//! leaning at the top.
//!
//! Per-emission cadence and per-particle lifetime are tuned to the
//! reference gif rather than the original game's literal numbers — the
//! gif shows a slow ~2 s climb, the original reports 500 ms
//! per particle, and pinning the parent at infinite-loop duration via
//! [`TOTAL_DURATION_MS`] keeps ambient chimneys puffing for the map's
//! lifetime.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const SPRITE: &str = "data/sprite/이팩트/굴뚝연기";
/// Preload list aggregated by [`crate::custom_effect_sprite_paths`]
/// so the renderer has the GRF sprite in its `EffectSpriteCache` before the
/// first puff is emitted (otherwise SpriteParticle entries are silently
/// skipped).
pub const SPRITES: &[&str] = &[SPRITE];
const FRAMES_PER_SECOND: f32 = 60.0;

/// Ambient loop — keeps emitting for the map's lifetime.
pub const TOTAL_DURATION_MS: u32 = u32::MAX;

/// Spawn one puff every `EMIT_PERIOD_S` seconds.
const EMIT_PERIOD_S: f32 = 0.35;
/// Each puff lives this long (seconds). The gif's puffs are visible for
/// ~1.5–2 s before fading out completely.
const PUFF_LIFETIME_S: f32 = 1.8;
/// Upward velocity at spawn — native RO coords, so negative Y rises.
const RISE_SPEED_PER_S: f32 = -6.0;
/// Horizontal wind acceleration, applied along the trail direction. A
/// non-zero value gives the column its bend: puffs born straight-up
/// gradually drift along the wind as they age.
const WIND_ACCEL_PER_S2: f32 = 4.0;
/// Below this horizontal trail distance there's no wind direction, so
/// puffs rise vertically.
const MIN_DIR_DISTANCE: f32 = 0.001;

const SIZE: f32 = 1.5;
const FADE_OUT_START_FRACTION: f32 = 2.0 / 3.0;

#[derive(Clone, Copy)]
struct Puff {
    spawn_age: f32,
}

pub struct DragonsmokeEffect {
    source: [f32; 3],
    /// Unit vector pointing along the wind direction in the XZ plane.
    /// Zero when the trail anchor is a single point.
    wind_dir: [f32; 2],
    age: f32,
    next_emit_at: f32,
    puffs: Vec<Puff>,
}

impl DragonsmokeEffect {
    /// `from` is the chimney source; `to - from` defines the wind
    /// direction. A single-point anchor (`from == to`) keeps the smoke
    /// rising straight up.
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let len = (dx * dx + dz * dz).sqrt();
        let wind_dir = if len > MIN_DIR_DISTANCE {
            [dx / len, dz / len]
        } else {
            [0.0, 0.0]
        };
        Self {
            source: from,
            wind_dir,
            age: 0.0,
            next_emit_at: 0.0,
            puffs: Vec::new(),
        }
    }

    /// Position of a puff at the current `self.age`. Vertical rise is
    /// linear (`pos.y = source.y + RISE * t`); horizontal drift is
    /// quadratic (`pos.xz = source.xz + 0.5 * WIND * t²`) so the
    /// trajectory looks straight at the bottom and bends over toward
    /// the wind direction as `t` grows.
    fn puff_position(&self, puff: Puff) -> [f32; 3] {
        let t = (self.age - puff.spawn_age).max(0.0);
        let horizontal = 0.5 * WIND_ACCEL_PER_S2 * t * t;
        [
            self.source[0] + self.wind_dir[0] * horizontal,
            self.source[1] + RISE_SPEED_PER_S * t,
            self.source[2] + self.wind_dir[1] * horizontal,
        ]
    }

    fn puff_alpha(&self, puff: Puff) -> f32 {
        let t = (self.age - puff.spawn_age) / PUFF_LIFETIME_S;
        if t < 0.0 || t > 1.0 {
            return 0.0;
        }
        if t < FADE_OUT_START_FRACTION {
            1.0
        } else {
            1.0 - (t - FADE_OUT_START_FRACTION) / (1.0 - FADE_OUT_START_FRACTION)
        }
    }
}

impl Effect for DragonsmokeEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        while self.next_emit_at <= self.age {
            self.puffs.push(Puff {
                spawn_age: self.next_emit_at,
            });
            self.next_emit_at += EMIT_PERIOD_S;
        }
        // Reap dead puffs so the Vec doesn't grow unbounded over a
        // long-lived ambient effect.
        let cutoff = self.age - PUFF_LIFETIME_S;
        self.puffs.retain(|p| p.spawn_age > cutoff);
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for &puff in &self.puffs {
            let alpha = self.puff_alpha(puff);
            if alpha <= 0.0 {
                continue;
            }
            let position = self.puff_position(puff);
            let motion = ((self.age - puff.spawn_age) * FRAMES_PER_SECOND / 4.0) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: SPRITE,
                position,
                action_index: 0,
                motion_index: motion,
                size_scale: SIZE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
                aim_target: None,
                no_depth: false,
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
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut DragonsmokeEffect, dt: f32) {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn positions(e: &DragonsmokeEffect) -> Vec<[f32; 3]> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => Some(*position),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn puffs_rise_and_curve_along_wind_when_trail_present() {
        // Sociable test: cover update emission + curving position math.
        // Wind blows along +X.
        let mut e = DragonsmokeEffect::new([0.0; 3], [10.0, 0.0, 0.0]);
        step(&mut e, EMIT_PERIOD_S * 0.5); // first puff emitted at frame 0
        step(&mut e, 1.0); // age 1.5 * EMIT_PERIOD_S
        let pos: Vec<[f32; 3]> = positions(&e);
        assert!(!pos.is_empty());
        let oldest = pos[0];
        // Y rises (negative Y in native RO coords).
        assert!(oldest[1] < 0.0, "puff rises, got y = {}", oldest[1]);
        // X drifts in the wind direction (positive).
        assert!(
            oldest[0] > 0.0,
            "puff curves along +X wind, got x = {}",
            oldest[0]
        );
        // Z stays at 0 (wind blows purely along X).
        assert!(oldest[2].abs() < 1e-3, "no Z drift, got z = {}", oldest[2]);
    }

    #[test]
    fn puffs_rise_vertically_without_a_trail() {
        let mut e = DragonsmokeEffect::new([0.0; 3], [0.0; 3]);
        step(&mut e, 1.0);
        let pos = positions(&e);
        assert!(!pos.is_empty());
        for p in pos {
            assert!(
                p[0].abs() < 1e-3 && p[2].abs() < 1e-3,
                "no wind drift, got {:?}",
                p
            );
            assert!(p[1] <= 0.0, "rises only");
        }
    }

    #[test]
    fn dead_puffs_are_reaped() {
        // Run long enough for several emission cycles. The Vec should
        // not grow past the active-window count.
        let mut e = DragonsmokeEffect::new([0.0; 3], [10.0, 0.0, 0.0]);
        let total = 10.0; // 10 s
        let steps = (total * FRAMES_PER_SECOND) as u32;
        for _ in 0..steps {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        let active_window = (PUFF_LIFETIME_S / EMIT_PERIOD_S).ceil() as usize + 1;
        assert!(
            e.puffs.len() <= active_window,
            "puff count {} should stay near window {}",
            e.puffs.len(),
            active_window
        );
    }
}
