//! Effect stress-test harness shared by the `viewer` and `effect-viewer` tools.
//!
//! Spawns many effects at once at random on-screen ground locations so the
//! renderer's effect path can be profiled under load. A [`StressRunner`] keeps
//! an active set re-seeding on a fixed cadence to sustain the population; the
//! caller clears the holder and re-spawns the set on each [`StressTick::Reseed`].

use models::enums::EnumWithNumberValue;
use models::enums::effect_id::EffectId;
use ragnarok_game::effect::{
    EffectQueue, EffectSpec, body_attached, effect_spec, is_count_point_effect, is_link_effect,
    is_trail_effect,
};
use ragnarok_renderer::Camera;

use crate::viewer_common::screen_to_ground;

/// Default bolt count for count-point effects (Fire/Cold Bolt) under stress.
const DEFAULT_HIT_COUNT: u8 = 5;
/// `+Z` offset for the far endpoint of trail/link effects when stress-spawned.
const TRAIL_LEN: f32 = 22.0;
/// How often an active set is re-seeded (seconds). Short enough that brief
/// effects (Fire/Cold Bolt) stay continuously visible.
const RESPAWN_INTERVAL: f32 = 0.5;
/// Fraction of each screen dimension kept clear of effect spawns, so spawned
/// effects stay comfortably inside the viewport.
const SCREEN_MARGIN: f32 = 0.12;

/// A named collection of effects to spawn together, with per-effect counts.
pub struct StressSet {
    pub name: String,
    pub entries: Vec<(EffectId, usize)>,
}

impl StressSet {
    /// Total number of effect instances this set spawns per seed.
    pub fn total(&self) -> usize {
        self.entries.iter().map(|(_, n)| *n).sum()
    }
}

/// Browser row label for a set: name + total instance count.
pub fn stress_label(set: &StressSet) -> String {
    format!("{} ({} fx)", set.name, set.total())
}

/// Every valid non-`Noop` effect id, in numeric order. Mirrors the viewers'
/// `build_effect_list`.
fn all_effect_ids() -> Vec<EffectId> {
    (0..=2027usize)
        .filter_map(|v| EffectId::try_from_value(v).ok())
        .filter(|id| !matches!(effect_spec(*id), Some(EffectSpec::Noop) | None))
        .collect()
}

/// The stress-test set registry. Index 0 is always "All effects (×1)"; the rest
/// are hand-defined. Add or edit sets here — they are plain data.
pub fn stress_sets() -> Vec<StressSet> {
    vec![
        StressSet {
            name: "All effects (x1)".to_string(),
            entries: all_effect_ids().into_iter().map(|id| (id, 1)).collect(),
        },
        StressSet {
            name: "Level 99 auras".to_string(),
            entries: vec![
                (EffectId::Level99, 200),
                (EffectId::Level992, 200),
                (EffectId::Level993, 200),
            ],
        },
        StressSet {
            name: "Caster AoE mix".to_string(),
            entries: vec![
                (EffectId::Warpzone2, 10),
                (EffectId::Stormgust, 10),
                (EffectId::Meteorstorm, 10),
                (EffectId::BottomPoembragi, 20),
                (EffectId::Magnificat, 10),
                (EffectId::Blessing, 10),
                (EffectId::Firearrow, 10), // Fire Bolt
                (EffectId::Soulbreaker, 10),
                (EffectId::Icearrow, 10), // Cold Bolt
            ],
        },
    ]
}

/// Minimal xorshift32 PRNG — avoids pulling in a random-number dependency for a
/// few effect positions. Seed varies per launch via a frame counter.
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self {
            state: seed | 1, // never zero
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform float in `[0, 1)`.
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform float in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// `n` random ground positions under random on-screen pixels (inside a margin),
/// intersected with the `y = plane_y` plane. Pixels whose ray misses the plane
/// (e.g. pointing at the sky) are retried; a missing position falls back to the
/// plane under screen center.
pub fn random_visible_ground_positions(
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
    plane_y: f32,
    n: usize,
    rng: &mut Rng,
) -> Vec<[f32; 3]> {
    let x_lo = screen_w * SCREEN_MARGIN;
    let x_hi = screen_w * (1.0 - SCREEN_MARGIN);
    let y_lo = screen_h * SCREEN_MARGIN;
    let y_hi = screen_h * (1.0 - SCREEN_MARGIN);
    let center = screen_to_ground(camera, screen_w * 0.5, screen_h * 0.5, screen_w, screen_h, plane_y)
        .unwrap_or([0.0, plane_y, 0.0]);

    (0..n)
        .map(|_| {
            for _ in 0..8 {
                let sx = rng.range(x_lo, x_hi);
                let sy = rng.range(y_lo, y_hi);
                if let Some(p) = screen_to_ground(camera, sx, sy, screen_w, screen_h, plane_y) {
                    return p;
                }
            }
            center
        })
        .collect()
}

/// Queue one effect at `pos`, routing by effect kind the same way the viewers
/// do for single spawns. `actor_id` is the entity body-attached effects follow;
/// pass `None` (no actor, e.g. the effect viewer) to spawn them at `pos`.
pub fn enqueue_effect(
    queue: &mut EffectQueue,
    id: EffectId,
    pos: [f32; 3],
    actor_id: Option<u32>,
) {
    if let (true, Some(aid)) = (body_attached(id), actor_id) {
        queue.spawn_on(id, aid);
    } else if is_trail_effect(id) || is_link_effect(id) {
        let to = [pos[0], pos[1], pos[2] + TRAIL_LEN];
        queue.spawn_trail(id, pos, to);
    } else if is_count_point_effect(id) {
        queue.spawn_at_with_count(id, pos, DEFAULT_HIT_COUNT);
    } else {
        queue.spawn_at(id, pos);
    }
}

/// What the caller should do this frame for the active stress set.
pub enum StressTick {
    /// Nothing to do.
    Idle,
    /// Clear the holder and re-spawn set `#index` at fresh random positions.
    Reseed(usize),
}

/// Drives continuous-respawn of an active stress set.
pub struct StressRunner {
    active: Option<usize>,
    since_respawn: f32,
    seed_counter: u32,
}

impl Default for StressRunner {
    fn default() -> Self {
        Self {
            active: None,
            since_respawn: 0.0,
            seed_counter: 0x9E37_79B9,
        }
    }
}

impl StressRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin running set `index`; the next [`tick`](Self::tick) reseeds it.
    pub fn launch(&mut self, index: usize) {
        self.active = Some(index);
        self.since_respawn = RESPAWN_INTERVAL; // force immediate reseed
    }

    /// Stop running; caller clears the holder.
    pub fn stop(&mut self) {
        self.active = None;
        self.since_respawn = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_set(&self) -> Option<usize> {
        self.active
    }

    /// Advance the cadence; returns [`StressTick::Reseed`] when it's time to
    /// re-spawn the active set.
    pub fn tick(&mut self, dt: f32) -> StressTick {
        let Some(index) = self.active else {
            return StressTick::Idle;
        };
        self.since_respawn += dt;
        if self.since_respawn >= RESPAWN_INTERVAL {
            self.since_respawn = 0.0;
            StressTick::Reseed(index)
        } else {
            StressTick::Idle
        }
    }

    /// A fresh RNG seed for the next reseed (varies frame-to-frame).
    pub fn next_seed(&mut self) -> u32 {
        self.seed_counter = self.seed_counter.wrapping_mul(2_654_435_761).wrapping_add(1);
        self.seed_counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_set_and_named_sets() {
        let sets = stress_sets();
        // Index 0 is the auto-built "all effects" set: non-empty, all non-Noop.
        assert_eq!(sets[0].name, "All effects (x1)");
        assert!(sets[0].total() > 100);
        for (id, count) in &sets[0].entries {
            assert_eq!(*count, 1);
            assert!(!matches!(effect_spec(*id), Some(EffectSpec::Noop) | None));
        }
        // Named "Caster AoE mix" carries the exact bolts and counts requested.
        let aoe = sets.iter().find(|s| s.name == "Caster AoE mix").unwrap();
        assert!(aoe.entries.contains(&(EffectId::Icearrow, 10))); // Cold Bolt
        assert!(aoe.entries.contains(&(EffectId::Firearrow, 10))); // Fire Bolt
        assert_eq!(aoe.entries.iter().find(|(id, _)| *id == EffectId::BottomPoembragi).unwrap().1, 20);
    }

    #[test]
    fn runner_reseeds_on_cadence_then_idles_after_stop() {
        let mut runner = StressRunner::new();
        assert!(matches!(runner.tick(0.016), StressTick::Idle)); // not launched

        runner.launch(2);
        assert!(matches!(runner.tick(0.0), StressTick::Reseed(2))); // immediate first seed
        assert!(matches!(runner.tick(0.1), StressTick::Idle)); // mid-interval
        assert!(matches!(runner.tick(RESPAWN_INTERVAL), StressTick::Reseed(2)));

        runner.stop();
        assert!(!runner.is_active());
        assert!(matches!(runner.tick(RESPAWN_INTERVAL), StressTick::Idle));
    }
}
