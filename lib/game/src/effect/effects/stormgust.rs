//! EF_STORMGUST — Wizard Storm Gust. Reference: original game
//!
//! Composition:
//!   * `stormgust.str` plays the cloud + base particle storm (handled by
//!     `str_overlay`);
//!   * additionally the parent spawns **QuadHorn** ice shards in pairs
//!     every 29 frames between frames 30 and 150 — 4 spawn events × 2
//!     shards = **8 falling ice spikes** over the burst's lifetime.
//!
//! Each spike (as observed in the original game):
//!   * spawns at a random world offset `(radius·sin(angle), 20, -radius·cos(angle))`
//!     from the caster, where `radius ∈ [3..27]`;
//!   * carries a `100°` tilt (mostly downward) and a random
//!     compass heading;
//!   * has a base half-width `∈ [4..5]`, length `20`;
//!   * moves along its apex direction at `1.5` world units per frame;
//!   * lives `215 − spawn_frame` frames, fading 10 frames before death.
//!
//! Native framerate is 60 fps (see plan doc lesson re: EF_WARP).
//! Total visible duration: last spike spawns at frame 145, lives 70 frames,
//! dies at frame 215 ≈ 3.58 s — matches the 51-frame gif at the gif's
//! sampling rate.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

pub const ICE_TEXTURE: &str = "ice.tga";
pub const TEXTURES: &[&str] = &[ICE_TEXTURE];

pub const STR_OVERLAY: &str = "stormgust";

const FRAMES_PER_SECOND: f32 = 60.0;
/// Parent emitter lifetime; spawns stop after this frame.
const SPAWN_END_FRAME: f32 = 150.0;
/// First spawn frame is 58 (29*2): the first 29-frame multiple past frame 30.
const SPAWN_START_FRAME: f32 = 58.0;
const SPAWN_INTERVAL_FRAMES: f32 = 29.0;
const SPAWN_INTERVAL_S: f32 = SPAWN_INTERVAL_FRAMES / FRAMES_PER_SECOND;
/// Spikes per spawn event (two per spawn).
const SPIKES_PER_SPAWN: usize = 2;
/// Parent frame at which the last surviving spike fades out.
const FINAL_FRAME: f32 = 215.0;
/// Per-spike duration = `215 - spawn_frame`. Last spike (frame 145)
/// gets the shortest life: 70 frames.
const SPIKE_FADE_FRAMES: f32 = 10.0;

/// Total wall-clock duration of the visible burst, ms. Used as the spec's
/// `Custom { duration_ms }` so the holder despawns at the right time even
/// though the duration table claims 9990 ms.
pub const TOTAL_DURATION_MS: u32 =
    ((FINAL_FRAME / FRAMES_PER_SECOND) * 1000.0) as u32;

/// Spike length apex-to-base (base literal 20). The gif silhouette
/// shows each shard taking maybe a quarter of the cloud's vertical extent
/// — far smaller than that literal in our coord scale. The plan doc's
/// general lesson applies (the source numbers run large vs the gif).
const SPIKE_HEIGHT: f32 = 6.0;
/// Base half-width `(random(100) + 400) / 100` → 4.0..5.0.
/// Scaled down with the height to keep the spike's aspect ratio.
const SPIKE_BASE_HALF_WIDTH_MIN: f32 = 1.0;
const SPIKE_BASE_HALF_WIDTH_MAX: f32 = 1.5;
/// Apex speed `1.5` per frame. Combined with
/// the 3-frame speed limit (see `SPEED_LIMIT_FRAMES`) this is a
/// brief 50 ms "stab" of motion (4.5 world units total), not a sustained
/// fall — an earlier implementation missed this and the spike kept falling
/// across its whole lifetime, burying itself in the ground.
const SPIKE_SPEED_PER_FRAME: f32 = 1.5;
const SPIKE_SPEED_PER_S: f32 = SPIKE_SPEED_PER_FRAME * FRAMES_PER_SECOND;
/// Speed drops to 0 once the spike has moved for 3 frames.
/// The spike falls for 3 frames then stops dead, fading in place for the
/// rest of its lifetime.
const SPEED_LIMIT_FRAMES: f32 = 3.0;
const SPEED_LIMIT_S: f32 = SPEED_LIMIT_FRAMES / FRAMES_PER_SECOND;
/// Placement radius `random(25) + 3` → 3..28. Spread around the caster.
/// Tightened so spikes cluster under the cloud rather than spreading
/// further than the cloud's own footprint.
const RADIUS_MIN: f32 = 12.0;
const RADIUS_MAX: f32 = 38.0;
/// How much of the placement-radius the X component gets, relative to Z.
/// the source spreads spikes on a uniform circle (`X_SPREAD_RATIO == 1.0`); the
/// gif looks instead like an elongated band along the camera's depth
/// axis (foreshortening makes Z spread less visible than X). 0.4
/// compresses the X spread so the visible pattern reads as front-to-back
/// rather than right-to-left.
const X_SPREAD_RATIO: f32 = 0.6;
/// Spawn elevation above caster (base literal `20`). With the
/// shard sticking UP from the ground (the user reported "renders at the
/// middle of the STR, while it should be on the ground"), the spike's
/// BASE should sit at caster-ground level; the apex extends upward into
/// the air on its own.
const SPAWN_HEIGHT: f32 = 0.0;
/// Tilt `100`. QuadHorn's local frame matches the source's
/// (apex along local +Z, base on local XY plane, row-vector rotations),
/// so this constant is the literal with no translation: 100° puts
/// the apex mostly UP (native RO -Y) with a slight backward lean.
const SPIKE_TILT_X_DEG: f32 = 100.0;
const PEAK_ALPHA: f32 = 250.0 / 255.0;

/// Deterministic LCG for per-spike randomness. We don't want runtime `rand`
/// dependency, and we want tests to be repeatable. Mixing in the spawn
/// counter makes spikes from successive spawns differ.
fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy)]
struct IceSpike {
    age: f32,
    duration: f32,
    base_pos: [f32; 3],
    velocity: [f32; 3],
    tilt_x_deg: f32,
    rotation_y_deg: f32,
    size: f32,
}

impl IceSpike {
    fn step(&mut self, dt: f32) {
        // Speed limit: motion only for the first 3 frames
        // (~50 ms), then the spike is frozen in place for the rest of its
        // lifetime while it fades out.
        let effective_dt = if self.age >= SPEED_LIMIT_S {
            0.0
        } else if self.age + dt > SPEED_LIMIT_S {
            SPEED_LIMIT_S - self.age
        } else {
            dt
        };
        self.age += dt;
        self.base_pos[0] += self.velocity[0] * effective_dt;
        self.base_pos[1] += self.velocity[1] * effective_dt;
        self.base_pos[2] += self.velocity[2] * effective_dt;
    }

    fn alive(&self) -> bool {
        self.age < self.duration
    }

    fn alpha(&self) -> f32 {
        let fade_start_s = (self.duration * FRAMES_PER_SECOND - SPIKE_FADE_FRAMES)
            .max(0.0)
            / FRAMES_PER_SECOND;
        if self.age <= fade_start_s {
            PEAK_ALPHA
        } else {
            let fade_dur = self.duration - fade_start_s;
            if fade_dur <= 0.0 {
                0.0
            } else {
                let t = ((self.age - fade_start_s) / fade_dur).clamp(0.0, 1.0);
                PEAK_ALPHA * (1.0 - t)
            }
        }
    }
}

pub struct StormgustEffect {
    origin: [f32; 3],
    age: f32,
    spikes: Vec<IceSpike>,
    next_spawn_at: f32,
    rng_state: u32,
    /// Monotonic counter used as the index into the golden-angle phyllotaxis
    /// pattern for spike placement. The source uses `random(360)` for each spike's
    /// angle, but with only 8 spikes per cast the small sample size makes
    /// random angles cluster visibly — the user reported spikes spreading
    /// along one axis rather than around the full circle. The 137.5°
    /// (golden-angle) increment between successive spikes guarantees
    /// uniform coverage no matter how few we spawn.
    spawn_index: u32,
}

impl StormgustEffect {
    pub fn new(attach: Attach) -> Self {
        let origin = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } | Attach::Trail { .. } => [0.0; 3],
        };
        Self {
            origin,
            age: 0.0,
            spikes: Vec::with_capacity(8),
            next_spawn_at: SPAWN_START_FRAME / FRAMES_PER_SECOND,
            // Seed mixes effect origin so concurrent Stormgusts at different
            // locations produce different spike patterns. Stable for a given
            // origin → reproducible in tests.
            rng_state: 0x9E37_79B9
                ^ (origin[0].to_bits())
                ^ (origin[2].to_bits().rotate_left(13)),
            spawn_index: 0,
        }
    }

    fn spawn_pair(&mut self, virtual_spawn_age: f32) {
        // Catch-up: if an update tick covers multiple spawn intervals at
        // once, the spike was logically born at `virtual_spawn_age` but is
        // being inserted at `self.age`. Initialize its `age` to the gap so
        // its lifecycle matches a real-time spawn would have.
        let initial_age = (self.age - virtual_spawn_age).max(0.0);
        for _ in 0..SPIKES_PER_SPAWN {
            let radius = RADIUS_MIN + lcg_float(&mut self.rng_state) * (RADIUS_MAX - RADIUS_MIN);
            // Deterministic golden-angle placement — `137.5° × spawn_index`
            // mod 360° spreads N successive spikes evenly around the circle
            // for any N (avoids the clustering of `random(360)` at small N).
            let golden_angle_deg: f32 = 137.507_76; // 360° · (1 − 1/φ)
            let placement_angle =
                (self.spawn_index as f32 * golden_angle_deg).to_radians();
            self.spawn_index = self.spawn_index.wrapping_add(1);
            // Spike's compass heading — random, independent
            // of placement.
            let heading_deg = lcg_float(&mut self.rng_state) * 360.0;
            let size = SPIKE_BASE_HALF_WIDTH_MIN
                + lcg_float(&mut self.rng_state)
                    * (SPIKE_BASE_HALF_WIDTH_MAX - SPIKE_BASE_HALF_WIDTH_MIN);
            // Spawn pattern around the caster on the ground plane. The gif
            // shows shards strung out in a band that runs predominantly
            // front-to-back (along world Z, the camera's depth axis), not
            // left-to-right — the literal full-circle `random(360)`
            // looked right-to-left in our viewer because the camera's
            // pitch foreshortens the Z axis on screen. We bias the spread
            // to an elongated ellipse: full radius along Z, ~40% along X.
            // Phyllotaxis still keeps successive spawns well-separated.
            let z_offset = radius * placement_angle.cos();
            let x_offset = radius * placement_angle.sin() * X_SPREAD_RATIO;
            let spawn_pos = [
                self.origin[0] + x_offset,
                self.origin[1] - SPAWN_HEIGHT,
                self.origin[2] + z_offset,
            ];
            // Velocity along the spike's apex direction
            // (`speed3d = (0, 0, speed)` rotated by the spike matrix). With QuadHorn now
            // matching that local frame, apex direction is the unit
            // vector `(0, 0, 1)` rotated by X-rot(tilt) then Y-rot(yaw):
            //   after X: (0, -sin_t, cos_t)
            //   after Y: (cos_t * sin_y, -sin_t, cos_t * cos_y)
            let yaw = heading_deg.to_radians();
            let tilt = SPIKE_TILT_X_DEG.to_radians();
            let (sin_t, cos_t) = tilt.sin_cos();
            let (sin_y, cos_y) = yaw.sin_cos();
            let dir_x = cos_t * sin_y;
            let dir_y = -sin_t;
            let dir_z = cos_t * cos_y;
            let velocity = [
                dir_x * SPIKE_SPEED_PER_S,
                dir_y * SPIKE_SPEED_PER_S,
                dir_z * SPIKE_SPEED_PER_S,
            ];
            // Lifetime = (215 - virtual_spawn_frame) frames —
            // computed from the virtual spawn time so catch-up spawns get
            // the same duration they would have at real-time.
            let virtual_spawn_frame = virtual_spawn_age * FRAMES_PER_SECOND;
            let duration_frames = (FINAL_FRAME - virtual_spawn_frame).max(SPIKE_FADE_FRAMES);
            let duration = duration_frames / FRAMES_PER_SECOND;
            // Advance the position by elapsed catch-up time so the spike
            // sits where it would have ended up under real-time updates.
            // Respect the speed limit: any catch-up beyond `SPEED_LIMIT_S`
            // doesn't move the spike (it would have already stopped).
            let move_age = initial_age.min(SPEED_LIMIT_S);
            let aged_pos = [
                spawn_pos[0] + velocity[0] * move_age,
                spawn_pos[1] + velocity[1] * move_age,
                spawn_pos[2] + velocity[2] * move_age,
            ];
            self.spikes.push(IceSpike {
                age: initial_age,
                duration,
                base_pos: aged_pos,
                velocity,
                tilt_x_deg: SPIKE_TILT_X_DEG,
                rotation_y_deg: heading_deg,
                size,
            });
        }
    }
}

impl Effect for StormgustEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta;
        self.age += dt;
        for spike in &mut self.spikes {
            spike.step(dt);
        }

        let spawn_end_s = SPAWN_END_FRAME / FRAMES_PER_SECOND;
        while self.next_spawn_at <= self.age && self.next_spawn_at <= spawn_end_s {
            let virtual_age = self.next_spawn_at;
            self.spawn_pair(virtual_age);
            self.next_spawn_at += SPAWN_INTERVAL_S;
        }

        self.spikes.retain(|s| s.alive());

        let end_s = FINAL_FRAME / FRAMES_PER_SECOND;
        if self.age >= end_s && self.spikes.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for spike in &self.spikes {
            let alpha = spike.alpha();
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: spike.base_pos,
                size: spike.size,
                height: SPIKE_HEIGHT,
                tilt_x_deg: spike.tilt_x_deg,
                rotation_y_deg: spike.rotation_y_deg,
                texture: ICE_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
            });
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
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

    fn draws(effect: &StormgustEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut StormgustEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None });
    }

    #[test]
    fn declares_stormgust_str_overlay() {
        // The Slice G hybrid-path validator: a factory Custom effect drives
        // the STR playback alongside its primitives.
        let s = StormgustEffect::new(Attach::WorldPos([0.0, 0.0, 0.0]));
        assert_eq!(s.str_overlay(), Some(STR_OVERLAY));
    }

    #[test]
    fn no_spikes_before_first_spawn() {
        let mut s = StormgustEffect::new(Attach::WorldPos([0.0, 0.0, 0.0]));
        // First spawn is at frame 58 ≈ 0.97 s. Tick to just before that.
        step(&mut s, 0.9);
        assert!(draws(&s).is_empty());
    }

    #[test]
    fn spawns_eight_spikes_over_lifetime() {
        // 4 spawn events between frames 58 and 145, 2 spikes each = 8 total.
        let mut s = StormgustEffect::new(Attach::WorldPos([0.0, 0.0, 0.0]));
        // Tick to just past the final spawn frame (~2.42 s) before any spike
        // has died — easiest to assert max spikes alive at once.
        step(&mut s, 2.45);
        let count = draws(&s).len();
        assert!(
            count >= 6 && count <= 8,
            "expected 6-8 spikes alive at peak, got {count}"
        );
    }

    #[test]
    fn spike_alpha_fades_near_end_of_life() {
        let mut s = StormgustEffect::new(Attach::WorldPos([0.0, 0.0, 0.0]));
        step(&mut s, 1.0); // Just past first spawn at frame 58.
        let early = match draws(&s).first().unwrap() {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => panic!(),
        };
        assert!(
            (early - PEAK_ALPHA).abs() < 1e-3,
            "alpha at peak right after spawn (got {early})"
        );

        // Run past the fade window of the first spike. First spike has
        // duration ≈ (215-58)/60 ≈ 2.62s, fade starts at 2.45s. We jump to
        // 2.55s into its life.
        let mut s2 = StormgustEffect::new(Attach::WorldPos([0.0, 0.0, 0.0]));
        step(&mut s2, 58.0 / FRAMES_PER_SECOND + 0.01); // spawn first pair
        step(&mut s2, 2.55); // age first spike well into its fade
        let late = draws(&s2)
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::QuadHorn { color, .. } => Some(color[3]),
                _ => None,
            })
            .fold(0.0f32, f32::max);
        assert!(
            late < PEAK_ALPHA,
            "some spike has faded below peak (late={late})"
        );
    }

    #[test]
    fn spike_moves_during_speed_limit_window_then_freezes() {
        // Speed limit: spike moves for the first SPEED_LIMIT_S
        // (~50 ms) along its apex direction, then freezes for the rest of
        // its lifetime. With apex pointing UP (tilt=10°), the spike rises
        // briefly — native RO -Y = up, so Y *decreases* during the window.
        let mut s = StormgustEffect::new(Attach::WorldPos([0.0, 0.0, 0.0]));
        step(&mut s, 0.97);
        let pos_during = match draws(&s)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base,
            _ => panic!(),
        };
        // Stay inside the speed-limit window (spike age ~3 ms + 30 ms).
        step(&mut s, 0.03);
        let pos_in_window = match draws(&s)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base,
            _ => panic!(),
        };
        assert!(
            pos_in_window[1] != pos_during[1],
            "spike base Y changed during motion window: before={} after={}",
            pos_during[1],
            pos_in_window[1],
        );
        // Step past the speed-limit window and confirm Y stops changing.
        step(&mut s, 0.5);
        let pos_after_freeze = match draws(&s)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base,
            _ => panic!(),
        };
        step(&mut s, 0.5);
        let pos_later = match draws(&s)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base,
            _ => panic!(),
        };
        assert!(
            (pos_later[1] - pos_after_freeze[1]).abs() < 1e-4,
            "spike is frozen after speed-limit window: y stayed at {} (was {})",
            pos_later[1],
            pos_after_freeze[1],
        );
    }

    #[test]
    fn total_duration_matches_final_frame() {
        // Sanity: TOTAL_DURATION_MS should equal FINAL_FRAME / 60 * 1000.
        let expected = ((FINAL_FRAME / FRAMES_PER_SECOND) * 1000.0) as u32;
        assert_eq!(TOTAL_DURATION_MS, expected);
        assert!(TOTAL_DURATION_MS >= 3500 && TOTAL_DURATION_MS <= 3700);
    }
}
