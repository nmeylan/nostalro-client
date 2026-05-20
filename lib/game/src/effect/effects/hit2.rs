//! `EF_HIT2` — bash-style impact: 8 lens-flare petals arranged in a flower
//! around the target.
//!
//! The original game's Hit2 effect spawns one
//! 2D-texture per 45° slice (8 petals total) at frame 0. Each petal is
//! a screen-space billboard textured with `lens1.tga` or `lens2.tga`
//! (alternating around the ring) that:
//!
//!   * Rolls in screen space to align its long axis with the radial
//!     direction (roll = slice angle ± 15°).
//!   * Shrinks in width and grows tall over its lifetime
//!     (width speed = -width/duration, height speed = 1.5,
//!     height accel = 0.25) — the petal goes from a stubby bright
//!     blob to a long radial streak.
//!   * Translates outward at `speed` along its radial direction with
//!     deceleration (accel = -(speed/duration)/2).
//!   * Fades in over 8 frames (alpha speed = max alpha / 8).
//!
//! The original game's pixel scale is ~5-6× larger than ours; we divide
//! width/height by ~5 so the petals occupy a similar fraction
//! of the viewport as in `imgs/0-50/1.gif`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const LENS1: &str = "lens1.tga";
pub const LENS2: &str = "lens2.tga";
pub const TEXTURES: &[&str] = &[LENS1, LENS2];

const FRAMES_PER_SECOND: f32 = 60.0;

/// Number of petals around the flower. The original game iterates
/// `i = 0..360 step 45`, producing 8.
const PETAL_COUNT: usize = 8;

/// Lift the flower off the ground to roughly chest level on a character
/// at `world_pos = ground`. The original game's literal is `-20`; our
/// viewer's `world_pos` is at the entity's ground anchor so the same
/// literal would put the flower a character-and-a-half off the floor,
/// which is too high. `-10` matches the Hit3/Hit4 family's chest-level
/// placement.
const Y_OFFSET_BASE: f32 = -10.0;

/// Linear scale factor applied to the original game's width/height
/// literals. The original source uses `width_size = 5..20` and `height_size =
/// 20..40` against a sprite-pixel-to-world scale that's ~3× ours;
/// dividing keeps the silhouette comparable to the reference gif
/// (`imgs/0-50/1.gif` shows petals occupying most of the viewport).
const SIZE_SCALE: f32 = 1.0 / 3.0;

/// Per-petal width range (in original game's 5..20 mapped through `SIZE_SCALE`).
const WIDTH_MIN: f32 = 5.0 * SIZE_SCALE;
const WIDTH_MAX: f32 = 20.0 * SIZE_SCALE;
const HEIGHT_MIN: f32 = 20.0 * SIZE_SCALE;
const HEIGHT_MAX: f32 = 40.0 * SIZE_SCALE;

/// Height growth: per-frame at 60 fps. `height_speed = 1.5`,
/// `height_accel = 0.25` from the original game's literal — scaled
/// alongside the size.
const HEIGHT_SPEED_INIT_BASE: f32 = 1.5;
const HEIGHT_ACCEL_BASE: f32 = 0.25;

/// Radial speed (outward translation) range from the original game:
/// speed = (random(45) + 5) / 10 → 0.5..5.0 per frame at 60 fps.
const SPEED_MIN_PER_FRAME: f32 = 0.5;
const SPEED_MAX_PER_FRAME: f32 = 5.0;

/// Per-petal duration: `random(20) + 10` → 10..30 frames.
const DURATION_MIN_FRAMES: f32 = 10.0;
const DURATION_MAX_FRAMES: f32 = 30.0;

/// Fade-in: alpha rises by max-alpha/8 per frame → reaches full alpha at
/// frame 8.
const FADE_IN_FRAMES: f32 = 8.0;

/// Random initial radial offset along each petal's direction (original game's
/// `length = random(5)`). Scaled to keep the flower compact.
const SPAWN_RADIUS_MAX: f32 = 5.0 * SIZE_SCALE;

/// Jitter applied to each petal's compass angle: `(slice - 15) + random(30)`
/// → ±15° around the slice's centre, in degrees.
const ANGLE_JITTER_DEG: f32 = 15.0;

pub const TOTAL_DURATION_MS: u32 =
    (DURATION_MAX_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Deterministic LCG — same convention as `stormgust.rs` and
/// `hit.rs`; keeps tests stable.
fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy)]
struct Petal {
    /// Slice angle in radians (0, π/4, π/2, ...). Used for the radial
    /// outward direction and the billboard's screen-space rotation
    /// (with a small jitter — `roll_rad`).
    slice_angle_rad: f32,
    /// Actual roll angle for the billboard, equals slice angle plus a
    /// jitter in ±15°.
    roll_rad: f32,
    /// Outward radial speed at frame 0 (world units / second).
    speed_world_per_s: f32,
    /// Per-second deceleration applied to `speed_world_per_s`.
    decel_world_per_s2: f32,
    /// Current outward radius (integrated from `speed_world_per_s` each
    /// frame); starts at `initial_radius`.
    radius: f32,
    /// Current width (world units); shrinks toward 0 over `lifetime`.
    width: f32,
    /// Current height (world units); grows over `lifetime`.
    height: f32,
    /// Width shrink rate (per second).
    width_speed_world_per_s: f32,
    /// Height growth state — integrated like the original game's
    /// `height_speed`/`height_accel` per frame.
    height_speed_per_frame: f32,
    /// Constant per-frame height accel (scaled by SIZE_SCALE).
    height_accel_per_frame: f32,
    age: f32,
    lifetime: f32,
    /// Texture for this petal — alternating `lens1.tga` / `lens2.tga`
    /// around the flower.
    texture: &'static str,
}

impl Petal {
    fn alive(&self) -> bool {
        self.age < self.lifetime
    }

    fn alpha(&self) -> f32 {
        let frame = self.age * FRAMES_PER_SECOND;
        let fade_in = (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
        let fade_out = (1.0 - self.age / self.lifetime).clamp(0.0, 1.0);
        fade_in * fade_out
    }
}

pub struct Hit2Effect {
    world_pos: [f32; 3],
    petals: Vec<Petal>,
    age: f32,
    total_duration_s: f32,
    rng_state: u32,
    has_spawned: bool,
}

impl Hit2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13);
        Self {
            world_pos,
            petals: Vec::with_capacity(PETAL_COUNT),
            age: 0.0,
            total_duration_s: TOTAL_DURATION_MS as f32 / 1000.0,
            rng_state,
            has_spawned: false,
        }
    }

    fn spawn_petals(&mut self) {
        let slice = std::f32::consts::TAU / PETAL_COUNT as f32;
        for k in 0..PETAL_COUNT {
            let slice_angle = k as f32 * slice;
            // Roll jitter: ±15° around the slice direction (matches
            // original game's `(i - 15) + random(30)`).
            let jitter = (lcg_float(&mut self.rng_state) * 2.0 - 1.0)
                * ANGLE_JITTER_DEG.to_radians();
            let roll = slice_angle + jitter;

            let initial_radius =
                lcg_float(&mut self.rng_state) * SPAWN_RADIUS_MAX;

            let speed_per_frame = SPEED_MIN_PER_FRAME
                + lcg_float(&mut self.rng_state)
                    * (SPEED_MAX_PER_FRAME - SPEED_MIN_PER_FRAME);
            let duration_frames = DURATION_MIN_FRAMES
                + lcg_float(&mut self.rng_state)
                    * (DURATION_MAX_FRAMES - DURATION_MIN_FRAMES);
            let lifetime = duration_frames / FRAMES_PER_SECOND;

            // original game accel = -(speed / duration) / 2,
            // per-frame. Convert to per-second^2.
            let decel_per_frame = -(speed_per_frame / duration_frames) / 2.0;
            let speed_world_per_s = speed_per_frame * FRAMES_PER_SECOND;
            let decel_world_per_s2 =
                decel_per_frame * FRAMES_PER_SECOND * FRAMES_PER_SECOND;

            let width = WIDTH_MIN
                + lcg_float(&mut self.rng_state) * (WIDTH_MAX - WIDTH_MIN);
            let height = HEIGHT_MIN
                + lcg_float(&mut self.rng_state) * (HEIGHT_MAX - HEIGHT_MIN);
            // Width shrinks linearly from `width` to 0 over `lifetime`.
            let width_speed_world_per_s = -width / lifetime;
            // Height initial speed + accel are scaled the same way as
            // initial size so the growth-vs-initial ratio matches the
            // original game.
            let height_speed_per_frame = HEIGHT_SPEED_INIT_BASE * SIZE_SCALE;
            let height_accel_per_frame = HEIGHT_ACCEL_BASE * SIZE_SCALE;

            let texture = if k % 2 == 0 { LENS1 } else { LENS2 };

            self.petals.push(Petal {
                slice_angle_rad: slice_angle,
                roll_rad: roll,
                speed_world_per_s,
                decel_world_per_s2,
                radius: initial_radius,
                width,
                height,
                width_speed_world_per_s,
                height_speed_per_frame,
                height_accel_per_frame,
                age: 0.0,
                lifetime,
                texture,
            });
        }
    }

    fn step_petals(&mut self, dt: f32) {
        let dt_frames = dt * FRAMES_PER_SECOND;
        for p in &mut self.petals {
            // Radial outward velocity + deceleration.
            p.speed_world_per_s =
                (p.speed_world_per_s + p.decel_world_per_s2 * dt).max(0.0);
            p.radius += p.speed_world_per_s * dt;
            // Width shrinks linearly until clamped at 0.
            p.width = (p.width + p.width_speed_world_per_s * dt).max(0.0);
            // Height grows: accel applied first then speed (the same
            // integration ordering used elsewhere in the hit
            // family).
            p.height_speed_per_frame += p.height_accel_per_frame * dt_frames;
            p.height += p.height_speed_per_frame * dt_frames;
            p.age += dt;
        }
    }
}

impl Effect for Hit2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if !self.has_spawned {
            self.spawn_petals();
            self.has_spawned = true;
        }
        self.age += ctx.delta;
        self.step_petals(ctx.delta);
        self.petals.retain(|p| p.alive());
        if self.age >= self.total_duration_s && self.petals.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // Petals sit in the XY world plane (vertical, screen-aligned at
        // the default camera). The original game uses
        // delta-pos X `= length*sin(i)` /
        // delta-pos Y `= -length*cos(i) - 20`, i.e. a vertical circle
        // of small offsets centred Y_OFFSET_BASE above the master. We
        // reproduce that: petal at slice angle i is at
        //   x = radius * sin(i)
        //   y = -radius * cos(i) + Y_OFFSET_BASE      (native RO -Y up)
        //   z = 0
        for p in &self.petals {
            let alpha = p.alpha();
            if alpha <= 0.0 || p.width <= 0.0 || p.height <= 0.0 {
                continue;
            }
            let (sin_a, cos_a) = p.slice_angle_rad.sin_cos();
            let pos = [
                self.world_pos[0] + p.radius * sin_a,
                self.world_pos[1] + Y_OFFSET_BASE - p.radius * cos_a,
                self.world_pos[2],
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [p.width, p.height],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: p.roll_rad,
                texture: p.texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn spawns_eight_petals_alternating_textures() {
        let mut e = Hit2Effect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        // Frame 0 alpha is 0 (fade-in start) — step one frame so the
        // petals become visible before checking the draw count.
        e.update(&ctx(1.0 / 60.0));
        list.primitives.clear();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), PETAL_COUNT);
        // Alternating textures: even indices get LENS1, odd get LENS2.
        let textures: Vec<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        assert!(textures.iter().any(|t| *t == LENS1));
        assert!(textures.iter().any(|t| *t == LENS2));
        // Equal count of each.
        let n1 = textures.iter().filter(|t| **t == LENS1).count();
        let n2 = textures.iter().filter(|t| **t == LENS2).count();
        assert_eq!(n1, PETAL_COUNT / 2);
        assert_eq!(n2, PETAL_COUNT / 2);
    }

    #[test]
    fn petals_arranged_radially_around_centre() {
        // Spawn at a known origin and confirm the 8 petals form a
        // vertical-plane (XY) ring around it. Each petal's position
        // should be reachable from (origin_x, origin_y + Y_OFFSET_BASE)
        // via a small radial offset.
        let mut e = Hit2Effect::new([10.0, 20.0, 30.0]);
        e.update(&ctx(1.0 / 60.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let centre = [10.0_f32, 20.0 + Y_OFFSET_BASE, 30.0];
        for prim in &list.primitives {
            let EffectPrimitiveDraw::Billboard { pos, .. } = prim else {
                continue;
            };
            // XZ stays close to the centre (no Z offset, X spreads in
            // the screen-X direction). Allow a small radius from the
            // outward velocity that's already accumulated.
            assert!(
                (pos[2] - centre[2]).abs() < 1e-3,
                "Z stays at centre (vertical-plane layout): got {}",
                pos[2]
            );
            // Each petal sits within a few world units of the centre
            // in the XY plane (initial radius + 1 frame of outward
            // motion).
            let dx = pos[0] - centre[0];
            let dy = pos[1] - centre[1];
            let r = (dx * dx + dy * dy).sqrt();
            assert!(r < 10.0, "petal within flower radius: r={r}");
        }
    }

    #[test]
    fn petal_width_shrinks_height_grows() {
        let mut e = Hit2Effect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let initial: Vec<(f32, f32)> =
            e.petals.iter().map(|p| (p.width, p.height)).collect();
        // Advance ~half a typical petal's lifetime.
        for _ in 0..10 {
            e.update(&ctx(1.0 / 60.0));
        }
        // At least one petal should still be alive with a smaller
        // width and larger height than its initial values.
        let shrunk_and_grew = e.petals.iter().zip(&initial).any(|(p, (w0, h0))| {
            p.width < *w0 && p.height > *h0
        });
        assert!(
            shrunk_and_grew,
            "expected at least one petal with shrunk width + grown height"
        );
    }

    #[test]
    fn effect_dies_after_petals_finish() {
        let mut e = Hit2Effect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        // Run for 2× the longest petal lifetime (30 frames = 0.5 s).
        while t < 1.5 {
            status = e.update(&ctx(1.0 / 60.0));
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
