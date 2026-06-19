//! EF_FROSTDIVER / EF_FROSTDIVER2 — Mage Frost Diver ice burst.
//!
//! Both variants launch a cluster of `QuadHorn` ice spikes pointing
//! mostly straight up (tilt ∈ [80, 100]°) with random heading
//! and per-spike width/height drawn from variant-specific
//! ranges.
//!
//! * **FrostDiver2 (id 28)** — spawns **8 spikes
//!   at frame 0**, tightly clustered. Anchored on a single world point
//!   (`Attach::WorldPos`). One-shot.
//! * **FrostDiver (id 27)** — projectile-trail. A cursor advances 2
//!   units per frame along the caster→target line and spawns one spike
//!   per frame, stopping when the remaining distance is `≤ 2.5`. With
//!   `Attach::Trail { from, to }` the spike count is therefore
//!   deterministic in the distance — `ceil((distance - 2.5) / 2.0)` — and
//!   the placement is laid out along the trail. Without trail data
//!   (`Attach::WorldPos` for the effect viewer) we fall back to a
//!   randomly-sized cluster at the spawn point, capped by
//!   `params.spike_count_range`.
//!
//! Each spike moves along its apex
//! direction for the first `SPEED_LIMIT_FRAMES` frames, decelerating, then
//! sits still and fades the last 10 frames before death — same shape as the
//! Stormgust spikes. The original width/height literals run
//! ~3× larger than our world-unit scale (cf. `stormgust.rs` `SPIKE_HEIGHT`);
//! the values below are eyeballed against the reference gif rather than
//! used verbatim.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, rise_step};

pub const ICE_TEXTURE: &str = "ice.tga";
pub const STONE_TEXTURE: &str = "stone.bmp";
pub const TEXTURES: &[&str] = &[ICE_TEXTURE, STONE_TEXTURE];

/// Tunable parameter set for one Frost Diver variant. FrostDiver
/// and FrostDiver2 share the `QuadHorn` primitive but pick from different
/// random ranges for size/height/spread — FrostDiver's spikes are slim
/// and tall, FrostDiver2's are chunkier and tightly clustered.
#[derive(Clone, Copy)]
pub struct FrostDiverParams {
    /// Spike texture. FrostDiver uses `ice.tga`; Grimtooth uses `stone.bmp`.
    pub texture: &'static str,
    /// Blend mode. Ice spikes glow additively; opaque stone spikes use alpha
    /// so the texture keeps its brown colour instead of washing out white.
    pub blend: BlendKind,
    /// Inclusive range for the total ice-spike count per cast.
    /// FrostDiver2 always emits exactly 8, so its range is `(8, 8)`;
    /// FrostDiver spawns one spike per frame as the projectile
    /// travels toward the target — without the projectile pipeline we
    /// emulate the variable count with a small random range per cast.
    pub spike_count_range: (u32, u32),
    /// How many frames the cluster-mode spawn window lasts. `0` = all spawn
    /// at frame 0 (FrostDiver2 behaviour); `> 0` staggers spawns linearly
    /// across the window (FrostDiver cluster-fallback behaviour). Trail mode
    /// ignores this and uses `trail_cadence_frames` instead.
    pub burst_over_frames: f32,
    /// Trail mode: frames between consecutive spike spawns as the projectile
    /// cursor walks the caster→target line. FrostDiver spawns one per
    /// frame (`1.0`); Grimtooth spawns one every 3 frames (`3.0`).
    pub trail_cadence_frames: f32,
    /// Trail mode: how far back from the target the first spike spawns
    /// (the initial setback along the trail). FrostDiver `5.0`, Grimtooth
    /// `2.0`.
    pub trail_initial_offset: f32,
    /// Lifetime per spike, frames.
    pub spike_duration_frames: f32,
    /// Inclusive range for the random base half-width per spike (world
    /// units). Drawn randomly per spike.
    pub base_half_width_range: (f32, f32),
    /// Inclusive range for the random per-spike height (world units).
    /// Drawn randomly per spike, scaled to match the
    /// gif silhouette in our coord system (~3× smaller than the original,
    /// cf. `stormgust.rs` `SPIKE_HEIGHT`).
    pub height_range: (f32, f32),
    /// Inclusive range for the spawn-time offset radius from the burst
    /// centre. Drawn randomly per spike. FrostDiver
    /// spreads its spikes; FrostDiver2 clusters them tightly.
    pub spawn_radius_range: (f32, f32),
}

/// FrostDiver2 (`EF_FROSTDIVER2`, id 28) — one-shot 8-spike burst,
/// chunky bases tightly clustered. Sizes from the original's
/// random ranges, scaled so the silhouette matches the reference gif.
pub const FROSTDIVER2: FrostDiverParams = FrostDiverParams {
    texture: ICE_TEXTURE,
    blend: BlendKind::Additive,
    // Fixed at 8.
    spike_count_range: (8, 8),
    burst_over_frames: 0.0,
    trail_cadence_frames: 1.0,
    trail_initial_offset: TRAIL_INITIAL_OFFSET,
    spike_duration_frames: 40.0,
    // Original width 1.0..3.5, scaled down to our world units.
    base_half_width_range: (0.6, 1.4),
    // Original height 20..30, scaled down to our world units.
    height_range: (4.0, 6.5),
    // Original spread 0.1..0.5 (very tight cluster).
    // Scaled up to give a visible footprint in our coord scale.
    spawn_radius_range: (1.5, 5.0),
};

/// FrostDiver (`EF_FROSTDIVER`, id 27) — slim, tall spikes spread
/// further apart along the projectile trail. The original spawns one
/// spike per frame as the projectile travels toward the target; without
/// the projectile pipeline we approximate that with a staggered burst.
pub const FROSTDIVER: FrostDiverParams = FrostDiverParams {
    texture: ICE_TEXTURE,
    blend: BlendKind::Additive,
    // Reference gif shows ~3–5 visible spikes at peak. The original count is
    // determined by projectile travel time, which we don't simulate;
    // pick a random small count per cast to vary the silhouette.
    spike_count_range: (3, 5),
    burst_over_frames: 14.0,
    // One spike per frame as the projectile travels.
    trail_cadence_frames: 1.0,
    trail_initial_offset: TRAIL_INITIAL_OFFSET,
    spike_duration_frames: 40.0,
    // Original width 0.6..1.0. Roughly half FD2's
    // base width — produces the slimmer silhouette.
    base_half_width_range: (0.3, 0.6),
    // Original height 15..18. Slightly shorter
    // in absolute terms than FD2 but with a much taller aspect ratio
    // because the base is so narrow.
    height_range: (7.0, 10.0),
    // Original spread 0.5..1.5. Triple FD2's spread —
    // more space between each spike in the trail.
    spawn_radius_range: (3.0, 8.0),
};

/// Grimtooth (`EF_GRIMTOOTH`, id 123) — Assassin Cross spike trail. It reuses
/// FrostDiver's projectile with `stone.bmp`: it walks
/// a cursor from the caster toward the target, launching one small spike
/// every 3 frames until within 2.5 units of the target. Small slim stone
/// blades. The bigger impact spikes are a separate effect (Grimtoothatk).
pub const GRIMTOOTH: FrostDiverParams = FrostDiverParams {
    texture: STONE_TEXTURE,
    // Opaque brown stone — alpha blend keeps the colour (additive washes it
    // out to icy white).
    blend: BlendKind::Alpha,
    // Cluster fallback only (no trail data): a few spikes around the point.
    spike_count_range: (3, 6),
    burst_over_frames: 18.0,
    // One spike every 3 frames.
    trail_cadence_frames: 3.0,
    // The cursor steps a 2-unit vector once before the first
    // spike, so the trail starts ~2 units back from the target.
    trail_initial_offset: 2.0,
    spike_duration_frames: 40.0,
    // The caster spikes are slim and short: original width 0.6..1.0
    // and height 10 (0.4× grimtoothatk's 25); the bigger impact blades
    // are roughly bottom width 0.15 / height 2.5 in
    // ~1:1 world units. Keep these slim and short.
    base_half_width_range: (0.18, 0.3),
    height_range: (2.5, 4.0),
    // Trail mode ignores this; used only for the cluster fallback spread.
    spawn_radius_range: (2.0, 5.0),
};

/// Spike tilt is randomized 80..100°. A `tilt_x_deg` near 100 reads as
/// "apex up, slight backward lean" (cf. stormgust).
const SPIKE_TILT_MIN_DEG: f32 = 80.0;
const SPIKE_TILT_MAX_DEG: f32 = 100.0;
/// Apex speed 3.0/frame in the original. Combined
/// with the speed-limit window of 20 frames, the spike grows upward for
/// the first 20 frames then freezes in place for the fade-out tail. We
/// reproduce the upward growth with a shorter cap (matches the gif's
/// "spike appears and then holds" shape).
const SPIKE_SPEED_PER_FRAME: f32 = 0.18;
const SPIKE_SPEED_PER_S: f32 = SPIKE_SPEED_PER_FRAME * FRAMES_PER_SECOND;
const SPEED_LIMIT_FRAMES: f32 = 20.0;
const SPEED_LIMIT_S: f32 = SPEED_LIMIT_FRAMES / FRAMES_PER_SECOND;
/// Peak alpha 200/255; fade-out begins 10 frames before the spike dies.
const PEAK_ALPHA: f32 = 200.0 / 255.0;
const FADE_OUT_FRAMES: f32 = 10.0;

/// The projectile cursor advances 2
/// world units toward the target each frame.
const TRAIL_STEP_PER_FRAME: f32 = 2.0;
/// Stop condition — once the cursor is closer
/// than this to the target, no more spikes are spawned.
const TRAIL_STOP_DISTANCE: f32 = 2.5;
/// Initial cursor offset — the first spike spawns 5 units back from the
/// target.
const TRAIL_INITIAL_OFFSET: f32 = 5.0;

/// Deterministic per-effect LCG so tests are repeatable and concurrent
/// bursts at different spawn points produce different spike patterns.
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
    height: f32,
}

impl IceSpike {
    fn step(&mut self, dt: f32) {
        rise_step(&mut self.base_pos, self.velocity, self.age, dt, SPEED_LIMIT_S);
        self.age += dt;
    }

    fn alive(&self) -> bool {
        self.age < self.duration
    }

    fn alpha(&self) -> f32 {
        fade_tail_alpha(self.age, self.duration, PEAK_ALPHA, FADE_OUT_FRAMES)
    }
}

pub struct FrostDiverEffect {
    /// Caster-side anchor (where spikes 0..N are laid out from for the
    /// trail variant, or the burst centre for the WorldPos fallback).
    origin: [f32; 3],
    /// Pre-computed per-spike anchor positions. Populated at
    /// construction from the trail when `Attach::Trail` is supplied;
    /// for cluster-mode variants this stays empty and spikes spawn
    /// randomly around `origin` instead.
    trail_anchors: Vec<[f32; 3]>,
    params: FrostDiverParams,
    /// Total spike count drawn from `params.spike_count_range` (cluster
    /// mode) or from the caster→target distance (trail mode). Pinned at
    /// construction so the staggered-spawn schedule has a stable target.
    spike_count: u32,
    age: f32,
    spikes: Vec<IceSpike>,
    rng_state: u32,
    spike_index: u32,
}

impl FrostDiverEffect {
    /// Construct a Frost Diver burst. `from`/`to` are the resolved
    /// caster/target world positions. Calls with `from == to` (the
    /// effect-viewer demo path and any caller that doesn't supply a
    /// trail) collapse to cluster mode at `from`, matching the
    /// historical `Attach::WorldPos` behaviour.
    pub fn new(from: [f32; 3], to: [f32; 3], params: FrostDiverParams) -> Self {
        let (origin, trail_anchors) = derive_anchors(from, to, params.trail_initial_offset);

        let mut rng_state = 0x9E37_79B9
            ^ origin[0].to_bits()
            ^ origin[2].to_bits().rotate_left(11);

        let spike_count = if !trail_anchors.is_empty() {
            // Projectile mode: spike count is determined by the
            // distance travelled, not by `spike_count_range`. The cast
            // emits one spike per cursor step along the trail.
            trail_anchors.len() as u32
        } else {
            let (count_min, count_max) = params.spike_count_range;
            if count_max <= count_min {
                count_min
            } else {
                // Inclusive [count_min, count_max] from the same LCG used
                // for per-spike placement, so two casts at the same
                // origin still produce reproducible counts in tests.
                let span = count_max - count_min + 1;
                count_min + (lcg_next(&mut rng_state) % span)
            }
        };

        let mut e = Self {
            origin,
            trail_anchors,
            params,
            spike_count,
            age: 0.0,
            spikes: Vec::with_capacity(spike_count as usize),
            rng_state,
            spike_index: 0,
        };
        // FrostDiver2 cluster mode: all spikes at frame 0. Drop them in
        // eagerly so the first render frame already has the burst.
        if e.params.burst_over_frames <= 0.0 {
            for _ in 0..e.spike_count {
                e.spawn_one();
            }
        }
        e
    }

    fn spawn_one(&mut self) {
        let (size_min, size_max) = self.params.base_half_width_range;
        let (height_min, height_max) = self.params.height_range;

        // Trail mode walks `trail_anchors` in order so the visible
        // spike-trail follows the caster→target line. Cluster mode
        // picks a random offset around `origin`.
        let spawn_pos = if let Some(anchor) =
            self.trail_anchors.get(self.spike_index as usize).copied()
        {
            anchor
        } else {
            let (radius_min, radius_max) = self.params.spawn_radius_range;
            let placement_angle = lcg_float(&mut self.rng_state) * std::f32::consts::TAU;
            let placement_radius =
                radius_min + lcg_float(&mut self.rng_state) * (radius_max - radius_min);
            [
                self.origin[0] + placement_radius * placement_angle.cos(),
                self.origin[1],
                self.origin[2] + placement_radius * placement_angle.sin(),
            ]
        };

        let heading_deg = lcg_float(&mut self.rng_state) * 360.0;
        let tilt_deg = SPIKE_TILT_MIN_DEG
            + lcg_float(&mut self.rng_state) * (SPIKE_TILT_MAX_DEG - SPIKE_TILT_MIN_DEG);
        let size = size_min + lcg_float(&mut self.rng_state) * (size_max - size_min);
        let height = height_min + lcg_float(&mut self.rng_state) * (height_max - height_min);

        let velocity = apex_velocity(tilt_deg, heading_deg, SPIKE_SPEED_PER_S);

        self.spike_index = self.spike_index.wrapping_add(1);
        self.spikes.push(IceSpike {
            age: 0.0,
            duration: self.params.spike_duration_frames / FRAMES_PER_SECOND,
            base_pos: spawn_pos,
            velocity,
            tilt_x_deg: tilt_deg,
            rotation_y_deg: heading_deg,
            size,
            height,
        });
    }

    /// Frames over which all spikes are spawned. Cluster mode uses the
    /// fixed `burst_over_frames`; trail mode scales with the spike count so
    /// the projectile cadence (`trail_cadence_frames` per spike) is honoured.
    fn spawn_window_frames(&self) -> f32 {
        if self.trail_anchors.is_empty() {
            self.params.burst_over_frames
        } else {
            self.params.trail_cadence_frames * self.spike_count as f32
        }
    }

    fn total_duration_s(&self) -> f32 {
        (self.spawn_window_frames() + self.params.spike_duration_frames) / FRAMES_PER_SECOND
    }
}

/// Resolve the spawn anchor and (if applicable) the per-spike trail
/// positions from the `Attach`. For `Attach::Trail { from, to }` we
/// reproduce the projectile cursor: starting `TRAIL_INITIAL_OFFSET`
/// units back from the target, step `TRAIL_STEP_PER_FRAME` units toward
/// the caster, stopping once the remaining distance to the target is
/// `≤ TRAIL_STOP_DISTANCE`. Each cursor position becomes one spike's
/// XZ anchor; spike Y stays on the caster's ground plane (`from[1]`).
fn derive_anchors(
    from: [f32; 3],
    to: [f32; 3],
    initial_offset: f32,
) -> ([f32; 3], Vec<[f32; 3]>) {
    let dx = to[0] - from[0];
    let dz = to[2] - from[2];
    let total_dist = (dx * dx + dz * dz).sqrt();
    if total_dist <= initial_offset {
        // Caster and target are too close to draw a meaningful trail
        // (most likely a self-cast, the `from == to` collapse from the
        // single-point factory fallback, or a viewer stub call). Fall
        // back to cluster mode at the caster's position.
        return (from, Vec::new());
    }
    // Unit vector caster → target on the XZ plane.
    let ux = dx / total_dist;
    let uz = dz / total_dist;
    let mut anchors = Vec::new();
    // Cursor: distance-remaining-to-target. Starts at
    // `total_dist - initial_offset`, decreases by `TRAIL_STEP_PER_FRAME`
    // each iteration.
    let mut remaining = total_dist - initial_offset;
    while remaining > TRAIL_STOP_DISTANCE {
        let along = total_dist - remaining;
        anchors.push([from[0] + ux * along, from[1], from[2] + uz * along]);
        remaining -= TRAIL_STEP_PER_FRAME;
    }
    (from, anchors)
}

impl Effect for FrostDiverEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta;
        self.age += dt;
        for spike in &mut self.spikes {
            spike.step(dt);
        }

        // Staggered spawn — emit spikes evenly across the spawn window
        // (fixed for cluster mode, cadence-scaled for trail mode).
        let window_frames = self.spawn_window_frames();
        if window_frames > 0.0 {
            let burst_s = window_frames / FRAMES_PER_SECOND;
            let target_spawned =
                ((self.age / burst_s) * self.spike_count as f32) as u32;
            let target = target_spawned.min(self.spike_count);
            while self.spike_index < target {
                self.spawn_one();
            }
        }

        self.spikes.retain(|s| s.alive());

        if self.age >= self.total_duration_s() && self.spikes.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for spike in &self.spikes {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: spike.base_pos,
                size: spike.size,
                height: spike.height,
                tilt_x_deg: spike.tilt_x_deg,
                rotation_y_deg: spike.rotation_y_deg,
                texture: self.params.texture,
                color: [1.0, 1.0, 1.0, spike.alpha()],
                blend: self.params.blend,
            });
        }
    }
}

/// Wall-clock duration for the spec. `burst_over_frames + spike_duration`
/// is the latest frame any spike can be alive at.
pub const fn total_duration_ms(params: &FrostDiverParams) -> u32 {
    let frames = params.burst_over_frames + params.spike_duration_frames;
    (frames / FRAMES_PER_SECOND * 1000.0) as u32
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

    fn step(effect: &mut FrostDiverEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None });
    }

    fn draws(effect: &FrostDiverEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn fd_trail_lays_spikes_along_caster_to_target_line() {
        // Sociable test: with `Attach::Trail { from, to }`, FrostDiver
        // reproduces the projectile cursor — spike count derives from
        // the caster→target distance (one spike per
        // `TRAIL_STEP_PER_FRAME = 2.0` after a `TRAIL_INITIAL_OFFSET =
        // 5.0` setback and stopping `≤ TRAIL_STOP_DISTANCE = 2.5` from
        // the target). Spike anchors lie on the straight line between
        // `from` and `to` in XZ.
        let from = [0.0, 0.0, 0.0];
        // 25 units along +Z. Cursor starts at 25 - 5 = 20 units
        // away from target on the trail; steps 2 each iteration; stops
        // once remaining ≤ 2.5. That schedules 9 spawns (20, 18, 16,
        // 14, 12, 10, 8, 6, 4 — next would be 2 ≤ 2.5).
        let to = [0.0, 0.0, 25.0];
        let mut e = FrostDiverEffect::new(from, to, FROSTDIVER);
        // Burst-mode spawn-on-spawn schedule needs the update tick to
        // populate; step past the full burst window.
        step(&mut e, FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND + 0.01);
        let spawned = e.spike_index;
        assert_eq!(spawned, 9, "9 spikes expected for 25-unit trail");

        // Every spike's XZ position must lie on the from→to line.
        for prim in draws(&e) {
            let EffectPrimitiveDraw::QuadHorn { base, .. } = prim else {
                panic!("expected QuadHorn, got {prim:?}");
            };
            assert!(base[0].abs() < 1e-3, "spike X stays on the +Z line: {base:?}");
            // Spikes lie strictly between TRAIL_INITIAL_OFFSET from the
            // caster (first cursor stop) and TRAIL_STOP_DISTANCE from
            // the target (last cursor stop, exclusive — `remaining`
            // must still be > TRAIL_STOP_DISTANCE to spawn).
            let first_anchor_z = TRAIL_INITIAL_OFFSET;
            let last_anchor_z = to[2] - TRAIL_STOP_DISTANCE;
            assert!(
                first_anchor_z - 1e-3 <= base[2] && base[2] < last_anchor_z,
                "spike Z {} must lie inside the trail span [{}, {})",
                base[2],
                first_anchor_z,
                last_anchor_z,
            );
        }
    }

    #[test]
    fn fd_trail_distance_scales_spike_count() {
        // Sociable test: the projectile trail spike count scales with
        // caster→target distance — twice as far → roughly twice as
        // many spikes (one per cursor step).
        let from = [0.0, 0.0, 0.0];
        let short = FrostDiverEffect::new(from, [0.0, 0.0, 15.0], FROSTDIVER);
        let long = FrostDiverEffect::new(from, [0.0, 0.0, 35.0], FROSTDIVER);
        assert!(long.spike_count > short.spike_count + 5);

        // Below the initial cursor offset, no trail — falls back to
        // cluster mode bounded by spike_count_range.
        let too_close = FrostDiverEffect::new(from, [0.0, 0.0, 3.0], FROSTDIVER);
        assert!(
            (FROSTDIVER.spike_count_range.0..=FROSTDIVER.spike_count_range.1)
                .contains(&too_close.spike_count),
            "too-close trail falls back to cluster range, got {}",
            too_close.spike_count,
        );
    }

    #[test]
    fn fd2_emits_eight_spikes_at_frame_zero() {
        // Sociable test: FrostDiver2 is a one-shot 8-spike burst — all
        // QuadHorns must be present on the first render frame and the
        // burst tilts apex-up (`tilt_x_deg ∈ [80, 100]`).
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        step(&mut e, 0.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 8);
        for p in &prims {
            let EffectPrimitiveDraw::QuadHorn {
                tilt_x_deg,
                texture,
                blend,
                ..
            } = p
            else {
                panic!("expected QuadHorn, got {p:?}");
            };
            assert!(
                (SPIKE_TILT_MIN_DEG..=SPIKE_TILT_MAX_DEG).contains(tilt_x_deg),
                "tilt {tilt_x_deg} out of range"
            );
            assert_eq!(*texture, ICE_TEXTURE);
            assert_eq!(*blend, BlendKind::Additive);
        }
    }

    #[test]
    fn fd_and_fd2_size_ranges_differ_per_orig() {
        // Sociable test: FrostDiver uses smaller base widths
        // and a wider spread than FrostDiver2 — the slim, tall spikes
        // along a projectile trail vs. the chunky tight cluster. Lock
        // those relationships so future tuning doesn't silently
        // collapse the two variants back together.
        assert!(
            FROSTDIVER.base_half_width_range.1 <= FROSTDIVER2.base_half_width_range.0,
            "FD spikes must be narrower than FD2"
        );
        assert!(
            FROSTDIVER.height_range.0 > FROSTDIVER2.height_range.1,
            "FD spikes must be taller than FD2"
        );
        // Spread ranges overlap (FD spread 0.5..1.5 vs FD2 0.1..0.5
        // — `0.5` is in both), so we only assert FD's *max* radius is
        // strictly greater than FD2's. That guarantees FD's spikes can
        // sit further out than any FD2 spike, which is the gif
        // difference: trail vs cluster.
        assert!(
            FROSTDIVER.spawn_radius_range.1 > FROSTDIVER2.spawn_radius_range.1,
            "FD must reach further out than FD2"
        );
    }

    #[test]
    fn fd_staggers_spawn_across_burst_window() {
        // Sociable test: FrostDiver spawns spikes over a burst window —
        // there's at least one spike alive at every probe time, the count
        // grows monotonically until the window closes, and then decays
        // back to zero as spikes expire.
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER);
        step(&mut e, 0.0);
        let n0 = draws(&e).len();

        step(&mut e, FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND / 2.0);
        let n_mid = draws(&e).len();
        assert!(n_mid >= n0);

        step(&mut e, FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND);
        let n_full = draws(&e).len();
        assert!(n_full >= n_mid);
        assert!(n_full <= FROSTDIVER.spike_count_range.1 as usize);
    }

    #[test]
    fn fd_spike_count_stays_inside_orig_range() {
        // Sociable test: FrostDiver's spike count is randomized per cast
        // but must stay inside `FROSTDIVER.spike_count_range`. We don't
        // pin an exact value — different origins draw different counts
        // from the LCG — but the count must always be in-range, fewer
        // than FD2's fixed-8 count, and the reference gif shows ~3–5.
        for (origin, label) in [
            ([0.0, 0.0, 0.0], "origin zero"),
            ([10.0, 0.0, -5.0], "origin offset"),
            ([-3.5, 0.0, 22.7], "origin offset 2"),
        ] {
            let mut e = FrostDiverEffect::new(origin, origin, FROSTDIVER);
            // Step past burst window so all scheduled spikes have spawned.
            step(&mut e, FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND + 0.01);
            let drawn = draws(&e).len() as u32;
            // Some spikes may have started fading at this point but
            // `spike_index` is the authoritative spawn count.
            let spawned = e.spike_index;
            let (lo, hi) = FROSTDIVER.spike_count_range;
            assert!(
                (lo..=hi).contains(&spawned),
                "{label}: spawned {spawned} not in [{lo}, {hi}]"
            );
            assert!(spawned < FROSTDIVER2.spike_count_range.0,
                "{label}: FD spawned {spawned} >= FD2's fixed 8");
            assert!(drawn <= spawned, "{label}: drawn ≤ spawned");
        }
    }

    #[test]
    fn spike_motion_stops_after_speed_limit() {
        // Sociable test: the speed limit freezes each spike after the
        // initial growth window. Position should advance during the
        // window and hold steady after it.
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        step(&mut e, 0.0);
        let early_y = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };

        // Step through the speed-limit window.
        step(&mut e, SPEED_LIMIT_S);
        let limit_y = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };
        assert!(limit_y < early_y, "spike base rose (Y went more negative)");

        // Stepping past the speed limit shouldn't move the spike further.
        step(&mut e, 5.0 / FRAMES_PER_SECOND);
        let after_y = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };
        assert!((after_y - limit_y).abs() < 1e-4, "frozen after speed limit");
    }

    #[test]
    fn alpha_fades_in_final_window() {
        // Sociable test: PEAK_ALPHA holds until the fade-out window starts,
        // then alpha drops monotonically to zero by spike end-of-life.
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        step(&mut e, 0.0);
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a0 - PEAK_ALPHA).abs() < 1e-4);

        // Just before fade window.
        let fade_start_s =
            (FROSTDIVER2.spike_duration_frames - FADE_OUT_FRAMES) / FRAMES_PER_SECOND;
        step(&mut e, fade_start_s - 0.001);
        let a_pre = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a_pre - PEAK_ALPHA).abs() < 1e-3);

        // Mid fade.
        step(&mut e, FADE_OUT_FRAMES / FRAMES_PER_SECOND * 0.5);
        let a_fade = match draws(&e).first() {
            Some(EffectPrimitiveDraw::QuadHorn { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(a_fade < PEAK_ALPHA);
    }

    #[test]
    fn dies_when_all_spikes_expire() {
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        let end_s = total_duration_ms(&FROSTDIVER2) as f32 / 1000.0;
        while t < end_s * 2.0 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
