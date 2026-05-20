//! `EF_HIT1` / `EF_HIT3` / `EF_HIT4` — weapon-swing impact effects.
//!
//! Each Hit variant is a composite spawned at the target's position at
//! frame 0:
//!
//! - 1-2 expanding cylinder rings — a flared cone (inner ring at
//!   the impact plane, outer ring offset by `height_size` along the
//!   cone's axis) tilted onto its side and aimed
//!   along `180° - master heading`. We render this as the
//!   `Frustum` primitive with `tilt_x_rad = -π/2` and
//!   `rotation_y_rad = angle`. The flared cone's tip points along the
//!   impact direction; the inner ring stays at the target's feet.
//! - 4-8 debris bursts (forward and, for Hit1, also
//!   gravity-affected backward bursts) with random initial
//!   direction inside a 80°-wide cone around `angle` (forward) or
//!   `angle + 180°` (backward), per-frame velocity integration, optional
//!   gravity (the gravity variants), and a 3-segment trail
//!   emitted as three
//!   `SpriteParticle` draws per particle with decreasing alpha and size.
//!
//! Recipes verbatim:
//!
//! ```text
//! id | dur | cylinder(inner/outer/H/tex)        | fwd | back | sprite
//! ---|-----|------------------------------------|-----|------|---------
//! 0  | 10  | 5 / 10  / 3.5 / ring_blue.tga      |  2  |  2   | particle1
//! 2  | 15  | 1.5/1.5 / ~  / lens2.tga (×2)      |  8  |  0   | particle1
//! 3  | 15  | 0.5/4.0 / ~  / lens2.tga           |  5  |  0   | particle1
//! ```
//!
//! refs:
//!   * first hit variant
//!   * third hit variant
//!   * fourth hit variant
//!   * 3D cylinder primitive rendering
//!   * segment-shift step
//!
//! Native-direction handling: the `angle` parameter to `Hit::new_with_angle`
//! is the impact compass heading in radians (CCW from world +X around
//! world +Y). When the spawn pipeline doesn't carry a direction (current
//! state), `Hit::new` defaults `angle` to 0; the cone then points along
//! world +Z and the visual is still a directional ring shockwave + cone
//! of debris — just not aimed at the attacker. Threading a real impact
//! heading is a follow-up the entity-rotation pipeline will unlock.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

pub const RING_BLUE: &str = "ring_blue.tga";
pub const LENS2: &str = "lens2.tga";
pub const TEXTURES: &[&str] = &[RING_BLUE, LENS2];

pub const PARTICLE1_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[PARTICLE1_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// The Hit family uses a 3-segment trail per debris particle.
/// Each particle renders this many sprite billboards per frame: index 0
/// at the current position with full alpha/size, indices 1..N at
/// historical positions with proportionally reduced alpha and size.
const NUM_SEGMENTS: usize = 3;

/// Linear fade-in over the first 3 frames of the cylinder, hold, then
/// linear fade-out over the back half (the Hit family fades out
/// across the second half of its life).
const FADE_IN_FRAMES: f32 = 3.0;

/// Particle1 sprite has multiple motions, advanced every
/// 4 ticks at 60 fps. The default animation cadence for
/// effect debris is 4 ticks per motion frame, giving a
/// 60 fps / 4 = 15 fps motion cadence.
/// We honour that cadence.
const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

/// One cylinder ring per the original game's literal fields. The cylinder is
/// a flared cone (inner ring at the local origin, outer ring offset by
/// `height_size` along the cone's axis). Each frame:
///
///
///   * `height_speed += height_accel`
///   * `height_size  += height_speed` (capped at 100)
///   * `speed        += speed_accel`
///   * `pos          += speed × axis_unit_vector` (translation along
///                                                 the post-tilt -Y
///                                                 direction = heading)
///
/// So Hit1 sees a static-shape ring (height_speed=0) that translates
/// outward at `speed`, decelerating; Hit3/Hit4 see a ring whose
/// `height_size` grows from 0 — the cylinder ELONGATES into a long
/// bright shaft of light. With `lens2.tga`'s star/lens-flare pattern
/// wrapped around that shaft, the visible reference shows radial
/// rays bursting outward (see `imgs/0-50/2.gif` frame 2).
#[derive(Clone, Copy, Debug)]
pub struct RingParams {
    pub duration_frames: f32,
    /// Bottom ring radius (constant per-frame; the Hit
    /// family doesn't grow the outer ring).
    pub outer_size: f32,
    /// Top ring radius (constant per-frame).
    pub inner_size: f32,
    /// Height-size value at frame 0. Defaults to 0 unless explicitly set;
    /// Hit1 sets it to 3.5; Hit3 ring 1 / Hit3 ring 2 / Hit4 leave it
    /// at 0 and grow it via `initial_height_speed`, so their cones
    /// elongate from nothing over the first few frames.
    pub initial_height_size: f32,
    /// Height growth speed at frame 0 (per-frame at 60 fps).
    pub initial_height_speed: f32,
    /// Height growth accel (per-frame at 60 fps). Added to `height_speed`
    /// each frame BEFORE applying `height_speed` to `height_size`, so
    /// frame 1 already feels the accel.
    pub height_accel: f32,
    /// Translation speed at frame 0 (per-frame at 60 fps). Drives translation
    /// of the cylinder's centre along the heading direction.
    pub initial_speed: f32,
    /// Translation accel (per-frame at 60 fps). Decelerates `speed` each frame.
    /// Hit family pattern: `-(speed / duration) / 2`.
    pub speed_accel: f32,
    /// World-Y offset of the cylinder's anchor relative to the effect's
    /// `world_pos`. Native RO has -Y = up, so a negative value lifts
    /// the cylinder off the ground; without it the lower half of the
    /// ring sits below the ground plane.
    ///
    /// The original game lifts the ring a few units relative to the
    /// character's body center (≈ one character height above ground),
    /// but ours is anchored at the entity's ground point. So that
    /// small lift puts our cylinder a character height too low —
    /// Hit1 sits flat on the floor instead of at torso level. We
    /// compensate by using -10 for the whole family so it reads at
    /// torso level.
    pub y_offset: f32,
    pub texture: &'static str,
    pub color: [f32; 4],
}

impl RingParams {
    fn alpha_at(&self, frame: f32) -> f32 {
        let fade_out_at = self.duration_frames - self.duration_frames / 2.0;
        if frame <= FADE_IN_FRAMES {
            (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if frame >= fade_out_at {
            let span = (self.duration_frames - fade_out_at).max(1e-3);
            (1.0 - (frame - fade_out_at) / span).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

/// Per-ring per-frame state — accumulates the integrated values that
/// the 3D cylinder prim mutates each frame. One instance per
/// `RingParams` in the recipe.
#[derive(Clone, Copy, Debug)]
struct RingState {
    /// Integrated height size.
    height_size: f32,
    /// Integrated height speed.
    height_speed: f32,
    /// Integrated translation speed.
    speed: f32,
    /// Accumulated translation offset (delta from the anchor in world
    /// coords). Native RO frame.
    position_offset: [f32; 3],
}

/// One debris burst spec (replicated `count` times per spawn). Each
/// individual particle randomises its direction within `cone_half_width_deg`
/// of `base_yaw_deg`, picks a random speed / size / duration from the
/// supplied ranges, and integrates per-frame with optional gravity.
#[derive(Clone, Copy, Debug)]
pub struct DebrisBurst {
    pub count: usize,
    /// Compass heading the cone is centred on, *relative to* the
    /// effect's overall `angle` parameter (degrees). 0° = forward
    /// (same as the cylinder axis), 180° = backward.
    pub base_yaw_deg: f32,
    /// Cone half-width around the base heading (degrees). The Hit
    /// family uses 40°.
    pub cone_half_width_deg: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub size_min: f32,
    pub size_max: f32,
    pub duration_min_frames: f32,
    pub duration_max_frames: f32,
    /// Initial Y offset above the spawn point (delta-pos Y in the original
    /// — negative because native RO is -Y up).
    pub spawn_y_offset: f32,
    /// Random spawn-distance range along the direction:
    /// `random(4) + 4` → 4..8 world units.
    pub spawn_distance_min: f32,
    pub spawn_distance_max: f32,
    /// Initial gravitational velocity (world units / sec along world
    /// -Y for upward, +Y for downward). When `gravity_accel != 0` the
    /// arc curves over the particle's lifetime; otherwise the particle
    /// flies in a straight line under speed-decel alone.
    pub gravity_initial_world_y: f32,
    /// Per-second acceleration applied to gravity_initial (so the
    /// "upward" speed weakens and the particle falls). The original derives
    /// this from `-(grav_speed / duration) * 2`.
    pub gravity_accel_world_y: f32,
}

/// One Hit-family recipe: cylinder rings + debris bursts. Constructed
/// statically per variant (HIT1 / HIT3 / HIT4) and passed into
/// `HitEffect::new`.
#[derive(Clone, Copy, Debug)]
pub struct HitParams {
    pub rings: &'static [RingParams],
    pub bursts: &'static [DebrisBurst],
}

pub const HIT1: HitParams = HitParams {
    rings: &[RingParams {
        duration_frames: 10.0,
        // Hit1: outer=10, inner=5, height size=3.5 (static shape,
        // no height speed/accel set), speed=0.7 decelerating
        // → ring TRANSLATES forward 5..7 wu over its 10-frame life
        // and stays the same flared-cone shape.
        outer_size: 10.0,
        inner_size: 5.0,
        initial_height_size: 3.5,
        initial_height_speed: 0.0,
        height_accel: 0.0,
        initial_speed: 0.7,
        speed_accel: -(0.7 / 10.0) / 2.0,
        y_offset: -10.0,
        texture: RING_BLUE,
        color: [1.0, 1.0, 1.0, 1.0],
    }],
    // Hit1 debris: when minimal-effects mode is on
    // the entire debris block is skipped — no sparkles at all. In
    // normal mode the `particle1.spr` sprite renders at
    // a much smaller pixel footprint than ours does for the same
    // size value (our `(ppu / 7.5) * size_scale` is calibrated
    // higher). Cap the size range well below the original
    // 0.6..1.6 so the visible sparkle silhouette matches the reference.
    bursts: &[
        // 2× forward debris.
        DebrisBurst {
            count: 2,
            base_yaw_deg: 0.0,
            cone_half_width_deg: 40.0,
            speed_min: 0.6,
            speed_max: 1.5,
            size_min: 0.2,
            size_max: 0.5,
            duration_min_frames: 6.0,
            duration_max_frames: 30.0,
            spawn_y_offset: -10.0,
            spawn_distance_min: 4.0,
            spawn_distance_max: 8.0,
            gravity_initial_world_y: 0.0,
            gravity_accel_world_y: 0.0,
        },
        // 2× backward gravity debris: initial gravity = -(rand+30)/100
        // → -0.3..-1.2 (upward initial in native RO). Accel
        // = -(grav/dur)*2 (decelerates upward, then particles fall).
        // Convert to world-units/s: per-frame at 60 fps × 60.
        DebrisBurst {
            count: 2,
            base_yaw_deg: 180.0,
            cone_half_width_deg: 40.0,
            speed_min: 0.6,
            speed_max: 1.5,
            size_min: 0.2,
            size_max: 0.5,
            duration_min_frames: 6.0,
            duration_max_frames: 30.0,
            spawn_y_offset: -10.0,
            spawn_distance_min: 4.0,
            spawn_distance_max: 8.0,
            // Upward in native RO is -Y. The initial gravity is
            // negative → particle moves up. Per-frame value × 60 fps =
            // world units per second.
            gravity_initial_world_y: -0.75 * 60.0,
            // grav accel = -initial/dur*2 with dur ≈ 18 frames =
            // 0.75/18*2 ≈ 0.0833/frame = 5 wu/s². Positive (downward).
            gravity_accel_world_y: 5.0,
        },
    ],
};

pub const HIT3: HitParams = HitParams {
    // Hit3 launches two cylinders at the same point with different
    // sizes plus 8 forward particles. The first cylinder has
    // outer=inner=1.5 → zero geometric thickness; we render it with a
    // small visible height (`height_size=1.0`) so the spike still reads
    // before the second wider ring grows past it.
    rings: &[
        // Hit3 ring 1: outer=inner=1.5 (thin tube), height size
        // starts at 0 and grows fast (height speed 0.5, height accel 0.2)
        // → over 15 frames the cone elongates to ~35 wu, producing the
        // long bright shaft of light visible in the reference at frame 2.
        RingParams {
            duration_frames: 15.0,
            outer_size: 1.5,
            inner_size: 1.5,
            initial_height_size: 0.0,
            initial_height_speed: 0.5,
            height_accel: 0.2,
            initial_speed: 0.7,
            speed_accel: -(0.7 / 15.0) / 2.0,
            y_offset: -10.0,
            texture: LENS2,
            color: [1.0, 1.0, 1.0, 1.0],
        },
        // Hit3 ring 2: outer=4.0, inner=1.5 (wider flare),
        // slower height growth (height speed 0.25, height accel 0.2)
        // → a fatter conical shaft that wraps the thin spike in the
        // ring 1 above.
        RingParams {
            duration_frames: 15.0,
            outer_size: 4.0,
            inner_size: 1.5,
            initial_height_size: 0.0,
            initial_height_speed: 0.25,
            height_accel: 0.2,
            initial_speed: 0.7,
            speed_accel: -(0.7 / 15.0) / 2.0,
            y_offset: -10.0,
            texture: LENS2,
            color: [1.0, 1.0, 1.0, 1.0],
        },
    ],
    // Hit3 spawns 8 forward particles with
    // speed `(rand(120)+80)/100` → 0.8..2.0 (faster than Hit1) and
    // size shrink (`-size/duration`).
    bursts: &[DebrisBurst {
        count: 8,
        base_yaw_deg: 0.0,
        cone_half_width_deg: 40.0,
        speed_min: 0.8,
        speed_max: 2.0,
        size_min: 0.6,
        size_max: 1.6,
        duration_min_frames: 6.0,
        duration_max_frames: 30.0,
        spawn_y_offset: -10.0,
        spawn_distance_min: 4.0,
        spawn_distance_max: 8.0,
        gravity_initial_world_y: 0.0,
        gravity_accel_world_y: 0.0,
    }],
};

pub const HIT4: HitParams = HitParams {
    // Hit4: outer=4.0, inner=0.5 (narrow base flaring out),
    // height size starts at 0 and grows slower (height_speed=0.25,
    // heightAccel=0.15 — half Hit3 ring 2's accel) → over 15 frames
    // the cone elongates to ~28 wu, slightly shorter and slower than
    // Hit3's outer ring; combined with the wider flare (0.5..4.0)
    // this is a distinctly different visual silhouette from Hit3.
    rings: &[RingParams {
        duration_frames: 15.0,
        outer_size: 4.0,
        inner_size: 0.5,
        initial_height_size: 0.0,
        initial_height_speed: 0.25,
        height_accel: 0.15,
        initial_speed: 0.7,
        speed_accel: -(0.7 / 15.0) / 2.0,
        y_offset: -10.0,
        texture: LENS2,
        color: [1.0, 1.0, 1.0, 1.0],
    }],
    // Hit4: 5 forward particles.
    bursts: &[DebrisBurst {
        count: 5,
        base_yaw_deg: 0.0,
        cone_half_width_deg: 40.0,
        speed_min: 0.6,
        speed_max: 1.5,
        size_min: 0.6,
        size_max: 1.6,
        duration_min_frames: 6.0,
        duration_max_frames: 30.0,
        spawn_y_offset: -10.0,
        spawn_distance_min: 4.0,
        spawn_distance_max: 8.0,
        gravity_initial_world_y: 0.0,
        gravity_accel_world_y: 0.0,
    }],
};

pub const HIT1_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT1);
pub const HIT3_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT3);
pub const HIT4_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT4);

/// Total visible duration: max(ring durations, max debris duration).
/// Both are in frames at 60 fps. Used as the spec's
/// `Custom { duration_ms }` so the holder despawns at the right time.
const fn total_duration_ms(params: HitParams) -> u32 {
    let mut max_frames = 0.0_f32;
    let mut i = 0;
    while i < params.rings.len() {
        if params.rings[i].duration_frames > max_frames {
            max_frames = params.rings[i].duration_frames;
        }
        i += 1;
    }
    let mut j = 0;
    while j < params.bursts.len() {
        if params.bursts[j].duration_max_frames > max_frames {
            max_frames = params.bursts[j].duration_max_frames;
        }
        j += 1;
    }
    (max_frames / FRAMES_PER_SECOND * 1000.0) as u32
}

/// Deterministic LCG so the same spawn produces the same particle
/// pattern (matches `stormgust.rs` convention; keeps tests stable).
fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

/// One live debris particle. Carries enough history to render
/// `NUM_SEGMENTS` trail billboards per frame.
#[derive(Clone, Copy)]
struct Particle {
    /// Most recent positions, index 0 = current, 1..N = previous frames.
    history: [[f32; 3]; NUM_SEGMENTS],
    /// World-units / second velocity. Decelerates each frame by
    /// `decel_world_y_per_s2` along the heading; gravity is applied
    /// separately on the Y axis.
    velocity: [f32; 3],
    /// Speed magnitude along the heading direction. Speed plus
    /// accel decelerate the projection onto the
    /// initial direction; the velocity vector is rebuilt each frame
    /// from `direction * speed`.
    speed_world_per_s: f32,
    decel_world_per_s2: f32,
    /// Stored heading unit vector (initial direction).
    direction: [f32; 3],
    /// Current Y-axis gravity component (world units / second). For
    /// GRAVITY particles this starts negative (upward) and accelerates
    /// toward positive (downward).
    gravity_velocity_y: f32,
    gravity_accel_y: f32,
    age: f32,
    lifetime: f32,
    size: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age < self.lifetime
    }

    /// Step the particle by `dt` and shift the trail history forward.
    fn step(&mut self, dt: f32) {
        // Shift history: segment[N-1] = segment[N-2], ..., segment[1] = segment[0]
        for i in (1..NUM_SEGMENTS).rev() {
            self.history[i] = self.history[i - 1];
        }
        // Original game: speed += accel once per frame, with accel
        // negative (decelerating). We apply at world-time so the curve is
        // frame-rate independent. The original game also clamps to zero implicitly via
        // its fadeOutCnt=0 logic (no fade), but the visual stops once
        // speed hits 0.
        self.speed_world_per_s = (self.speed_world_per_s + self.decel_world_per_s2 * dt).max(0.0);
        // Rebuild velocity along the heading.
        self.velocity = [
            self.direction[0] * self.speed_world_per_s,
            self.direction[1] * self.speed_world_per_s,
            self.direction[2] * self.speed_world_per_s,
        ];
        // Apply gravity on Y separately.
        self.gravity_velocity_y += self.gravity_accel_y * dt;
        // Integrate.
        let mut new_pos = self.history[0];
        new_pos[0] += self.velocity[0] * dt;
        new_pos[1] += (self.velocity[1] + self.gravity_velocity_y) * dt;
        new_pos[2] += self.velocity[2] * dt;
        self.history[0] = new_pos;
        self.age += dt;
    }

    /// Linear alpha fade-out matching the original game's `fade_out_cnt = 0` semantics:
    /// the particle's alpha drops linearly from its peak across its
    /// whole lifetime instead of holding then fading.
    fn alpha(&self) -> f32 {
        (1.0 - self.age / self.lifetime).clamp(0.0, 1.0)
    }
}

pub struct HitEffect {
    world_pos: [f32; 3],
    params: HitParams,
    /// Impact heading: Y-axis rotation in radians. 0 = cone points along
    /// world +Z. The cylinder rotation_y_rad uses this directly.
    angle_rad: f32,
    /// Per-ring integrated state — the cylinder integration mutates these
    /// each frame. Indexed parallel to `params.rings`.
    ring_state: Vec<RingState>,
    particles: Vec<Particle>,
    age: f32,
    total_duration_s: f32,
    /// LCG seed; mixed with the spawn position so repeated spawns at
    /// different locations get different particle patterns. Stable per
    /// (params, world_pos) so tests are reproducible.
    rng_state: u32,
    /// Whether `spawn_particles` has run yet — debris is one-shot at
    /// frame 0.
    has_spawned: bool,
}

impl HitEffect {
    /// Spawn with a default heading. Until the spawn pipeline carries
    /// a real impact direction (the master entity's facing), the viewer
    /// needs a heading that doesn't point straight away from the
    /// camera (which would foreshorten the entire cone into a tiny
    /// dot). The effect viewer's default camera looks at the origin
    /// from yaw=0 with the world's +Z direction toward the viewer —
    /// the worst case for an axis-aligned cone is heading=0 (cone
    /// extends along -Z, away from the camera). We default to a 45°
    /// heading so the cone projects diagonally across screen, which
    /// also matches the diagonal silhouette in the original game's
    /// reference gifs (`imgs/0-50/2.gif` / `3.gif`).
    pub fn new(attach: Attach, params: HitParams) -> Self {
        Self::new_with_angle(attach, params, std::f32::consts::FRAC_PI_4)
    }

    /// Spawn with an explicit impact heading. `angle_rad` is rotation
    /// around world +Y (CCW from world +X). The cylinder's flared tip
    /// and the forward-debris cone both point along this heading; the
    /// backward gravity cone (Hit1) points the opposite way.
    pub fn new_with_angle(attach: Attach, params: HitParams, angle_rad: f32) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
        let total_duration_s = total_duration_ms(params) as f32 / 1000.0;
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13)
            ^ angle_rad.to_bits().rotate_left(7);
        // One state slot per ring; primed from the recipe's initial
        // values so frame 0 already reflects the starting height_size /
        // height_speed / speed (the ring will integrate from there).
        let ring_state: Vec<RingState> = params
            .rings
            .iter()
            .map(|r| RingState {
                height_size: r.initial_height_size,
                height_speed: r.initial_height_speed,
                speed: r.initial_speed,
                position_offset: [0.0; 3],
            })
            .collect();
        Self {
            world_pos,
            params,
            angle_rad,
            ring_state,
            particles: Vec::new(),
            age: 0.0,
            total_duration_s,
            rng_state,
            has_spawned: false,
        }
    }

    /// World-direction unit vector for the cylinder's per-frame
    /// translation. The original computes `speed3d = (0, -speed, 0) ×
    /// rotation matrix and adds it to the per-frame position offset, so in the original game's
    /// Y-up coordinate system the cylinder drifts downward over its
    /// lifetime. Mapped to this codebase's native RO -Y-up coordinates,
    /// that's the same direction the user described as "appears higher
    /// [via the y_offset lift] and moves a bit lower" — `+Y` in our
    /// frame is down. The horizontal heading (`angle_rad`) doesn't
    /// participate: the original game's matrix with latitude `-90` cancels out the
    /// Y-rotation for a `(0, -speed, 0)` input vector.
    fn heading_unit(&self) -> [f32; 3] {
        [0.0, 1.0, 0.0]
    }

    /// Integrate one frame's worth (dt_frames frames) of
    /// 3D-cylinder state for every ring. The original game does this once per
    /// game tick (60 fps); we scale to whatever `dt` the holder gave
    /// us so the simulation is frame-rate independent.
    fn step_rings(&mut self, dt_frames: f32) {
        let heading = self.heading_unit();
        for (params, state) in self.params.rings.iter().zip(self.ring_state.iter_mut()) {
            // speed += accel * dt; height_speed += height_accel * dt
            state.speed += params.speed_accel * dt_frames;
            state.height_speed += params.height_accel * dt_frames;
            // height_size += height_speed * dt
            state.height_size += state.height_speed * dt_frames;
            // Native RO height is capped at a max height of 100 in
            // the 3D cylinder prim.
            if state.height_size > 100.0 {
                state.height_size = 100.0;
            }
            if state.height_size < 0.0 {
                state.height_size = 0.0;
            }
            // pos += speed × heading * dt
            let step = state.speed * dt_frames;
            state.position_offset[0] += heading[0] * step;
            state.position_offset[1] += heading[1] * step;
            state.position_offset[2] += heading[2] * step;
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    /// Spawn all debris particles at frame 0 per the recipe.
    fn spawn_particles(&mut self) {
        for burst in self.params.bursts {
            let base_yaw_rad = self.angle_rad + burst.base_yaw_deg.to_radians();
            let cone_half_rad = burst.cone_half_width_deg.to_radians();
            for _ in 0..burst.count {
                // Random direction within the cone:
                // longitude in (base ± cone) and latitude in
                // (-90 + 40 ± random(100)) — an upper-hemisphere bias.
                let yaw_jitter =
                    (lcg_float(&mut self.rng_state) * 2.0 - 1.0) * cone_half_rad;
                let yaw = base_yaw_rad + yaw_jitter;
                // latitude `−90 + 40 + random(100)` ∈ −50..50,
                // measured from "facing down the local Z axis". Convert
                // to elevation from horizontal: roughly 0..40° upward.
                // Random in [-50, 50]° latitude = 40..140° elevation.
                let elev_deg = 40.0 + lcg_float(&mut self.rng_state) * 100.0 - 90.0;
                let elev_rad = elev_deg.to_radians();
                let (sin_e, cos_e) = elev_rad.sin_cos();
                let (sin_y, cos_y) = yaw.sin_cos();
                // Spherical to Cartesian: horizontal cos_e in (X,Z),
                // vertical -sin_e on Y (native RO -Y = up).
                let dir = [cos_e * sin_y, -sin_e, cos_e * cos_y];

                let speed_per_frame = burst.speed_min
                    + lcg_float(&mut self.rng_state) * (burst.speed_max - burst.speed_min);
                let speed_world_per_s = speed_per_frame * FRAMES_PER_SECOND;
                let duration_frames = burst.duration_min_frames
                    + lcg_float(&mut self.rng_state)
                        * (burst.duration_max_frames - burst.duration_min_frames);
                let lifetime = duration_frames / FRAMES_PER_SECOND;
                // Per-frame decel = `-(speed / duration) / 2`.
                // Converting to per-second: divide by frame time (1/60s)
                // to get per-second decel.
                let decel_per_frame =
                    -(speed_per_frame / duration_frames) / 2.0;
                let decel_world_per_s2 = decel_per_frame * FRAMES_PER_SECOND * FRAMES_PER_SECOND;

                let size = burst.size_min
                    + lcg_float(&mut self.rng_state) * (burst.size_max - burst.size_min);

                let spawn_distance = burst.spawn_distance_min
                    + lcg_float(&mut self.rng_state)
                        * (burst.spawn_distance_max - burst.spawn_distance_min);
                let spawn_pos = [
                    self.world_pos[0] + dir[0] * spawn_distance,
                    self.world_pos[1] + dir[1] * spawn_distance + burst.spawn_y_offset,
                    self.world_pos[2] + dir[2] * spawn_distance,
                ];

                self.particles.push(Particle {
                    history: [spawn_pos; NUM_SEGMENTS],
                    velocity: [
                        dir[0] * speed_world_per_s,
                        dir[1] * speed_world_per_s,
                        dir[2] * speed_world_per_s,
                    ],
                    speed_world_per_s,
                    decel_world_per_s2,
                    direction: dir,
                    gravity_velocity_y: burst.gravity_initial_world_y,
                    gravity_accel_y: burst.gravity_accel_world_y,
                    age: 0.0,
                    lifetime,
                    size,
                });
            }
        }
    }
}

impl Effect for HitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if !self.has_spawned {
            self.spawn_particles();
            self.has_spawned = true;
        }
        self.age += ctx.delta;
        // Step the cylinder rings (height_size growth + translation)
        // and the debris particles by the same dt. Convert delta to
        // per-frame integration unit (60 fps).
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.step_rings(dt_frames);
        for p in &mut self.particles {
            p.step(ctx.delta);
        }
        self.particles.retain(|p| p.alive());

        if self.age >= self.total_duration_s && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // -- Cylinder rings --------------------------------------------
        let frame = self.frame();
        for (ring, state) in self.params.rings.iter().zip(self.ring_state.iter()) {
            if frame >= ring.duration_frames {
                continue;
            }
            // Hit3/Hit4 start with height_size=0; before any ticks
            // accumulate, the cone has zero extent. Skip rendering an
            // invisible zero-length cone so we don't push a degenerate
            // primitive. The cone becomes visible once height_size grows
            // past zero on the next frame.
            if state.height_size <= 0.001 {
                continue;
            }
            let alpha = ring.alpha_at(frame);
            let color = [
                ring.color[0],
                ring.color[1],
                ring.color[2],
                ring.color[3] * alpha,
            ];
            // Map the flared-cone geometry to Frustum:
            //   bottom ring  → `bottom_size = outer_size` at base[1]
            //   top ring     → `top_size    = inner_size` at base[1]-h
            //   tilt_x_rad = -π/2  (lay the cone on its side so its
            //                       local -Y axis points along world)
            //   rotation_y_rad = self.angle_rad
            //                    (aim the cone along the heading)
            // Cylinder anchor is world_pos + y_offset + accumulated
            // translation along the heading (state.position_offset
            // integrates speed × heading_unit each tick).
            let cylinder_base = [
                self.world_pos[0] + state.position_offset[0],
                self.world_pos[1] + ring.y_offset + state.position_offset[1],
                self.world_pos[2] + state.position_offset[2],
            ];
            out.push(EffectPrimitiveDraw::Frustum {
                base: cylinder_base,
                // local frame for the 3D cylinder:
                //   inner ring at y=0          → at master level
                //   outer ring at y=-height_size → above master (native
                //                                  RO -Y = up)
                // Frustum's local frame:
                //   bottom ring at base[1]            → ground level
                //   top    ring at base[1] - height   → above ground
                // So map `bottom_size = inner_size` (narrow ring at
                // master) and `top_size = outer_size` (wide flare at
                // height): Hit1 has the wide 10-radius ring above
                // master with a narrow 5-radius base — like an
                // inverted-bell shockwave.
                //
                // No `tilt_x_rad` / `rotation_y_rad` — empirically the
                // original game's Hit1 renders the ring HORIZONTAL on
                // the ground (visible as a flat disc), not VERTICAL
                // (standing up like a wheel). The original uses latitude -90
                // does not translate to a -π/2 X-rotation in this
                // codebase's row-vector / -Y-up coordinate convention;
                // omitting the tilt gives the correct horizontal
                // shape. The user confirmed this visually against the
                // original-game Hit1 reference.
                bottom_size: ring.inner_size,
                top_size: ring.outer_size,
                height: state.height_size,
                sides: 16,
                rotation: 0.0,
                // Lens flare textures like lens2.tga benefit from tiling
                // around the cone (the star pattern in the texture
                // becomes a ring of repeated lens-flare rays). The
                // UV advances 0.25 per segment across 4 segments —
                // one full texture wrap per quadrant — so the texture
                // wraps 4 times around the cone circumference.
                uv_repeat: 4.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: ring.texture,
                color,
                blend: BlendKind::Additive,
            });
        }

        // -- Debris particles ------------------------------------------
        // Each particle renders NUM_SEGMENTS sprite billboards: index 0
        // at the current position with full alpha/size, indices 1..N at
        // historical positions with proportionally reduced alpha and
        // size (matching the original game's segment shift: alpha[i] = alpha*(N-i)/N,
        // size[i] = size*(2N-i)/(2N)).
        for p in &self.particles {
            let base_alpha = p.alpha();
            if base_alpha <= 0.0 {
                continue;
            }
            for i in 0..NUM_SEGMENTS {
                let seg_alpha = base_alpha
                    * (NUM_SEGMENTS - i) as f32
                    / NUM_SEGMENTS as f32;
                let seg_size = p.size
                    * (2 * NUM_SEGMENTS - i) as f32
                    / (2 * NUM_SEGMENTS) as f32;
                let frame_index = (p.age * 1000.0 / PARTICLE_FRAME_MS) as usize;
                let motion = frame_index.saturating_sub(i);
                out.push(EffectPrimitiveDraw::SpriteParticle {
                    sprite_path: PARTICLE1_SPRITE,
                    position: p.history[i],
                    motion_index: motion,
                    size_scale: seg_size,
                    color: [1.0, 1.0, 1.0, seg_alpha],
                    blend: BlendKind::Additive,
                });
            }
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
    fn hit1_emits_horizontal_ring_plus_three_segments_per_particle() {
        // Sociable test: drive Hit1 one tick. The first draw should be
        // the cylinder Frustum: tilt_x_rad=0 + rotation_y_rad=0 so the
        // axis stays vertical and the ring renders flat on the ground.
        // The inner/outer mapping puts inner_size at Frustum's bottom
        // (= master level) and outer_size at the top (above master).
        // The cylinder also translates downward over time (the
        // equivalent 3D-speed direction in this codebase's
        // native RO frame is +Y = downward).
        let mut e = HitEffect::new_with_angle(
            Attach::WorldPos([1.0, 2.0, 3.0]),
            HIT1,
            0.5, // angle_rad still recorded but unused for vertical cylinders
        );
        e.update(&ctx(1.0 / 60.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let EffectPrimitiveDraw::Frustum {
            base,
            tilt_x_rad,
            rotation_y_rad,
            bottom_size,
            top_size,
            height,
            ..
        } = list.primitives[0]
        else {
            panic!("first draw must be the cylinder Frustum, got {:?}", list.primitives[0]);
        };
        // XZ stays put (no horizontal translation).
        assert!((base[0] - 1.0).abs() < 1e-4, "X stays at spawn: {}", base[0]);
        assert!((base[2] - 3.0).abs() < 1e-4, "Z stays at spawn: {}", base[2]);
        // Y starts at master_y + y_offset and moves DOWNWARD (+Y
        // in native RO) by speed × dt_frames. After 1 frame at
        // speed=0.7, base[1] should be > spawn-offset Y.
        let spawn_y = 2.0 + HIT1.rings[0].y_offset;
        assert!(
            base[1] > spawn_y,
            "cylinder Y moved downward (toward master): got {} starting from {}",
            base[1],
            spawn_y
        );
        assert!((tilt_x_rad).abs() < 1e-5, "cylinder is vertical (tilt=0): got {tilt_x_rad}");
        assert!((rotation_y_rad).abs() < 1e-5, "no Y-rotation: got {rotation_y_rad}");
        // inner=5 at y=0 → Frustum bottom_size=inner=5.
        // outer=10 at y=-height size → Frustum top_size=outer=10.
        assert!((bottom_size - 5.0).abs() < 1e-4, "bottom_size=inner_size=5: {bottom_size}");
        assert!((top_size - 10.0).abs() < 1e-4, "top_size=outer_size=10: {top_size}");
        assert!((height - 3.5).abs() < 1e-4, "Hit1 heightSize is static at 3.5");

        // The remaining draws are SpriteParticles.
        let particles: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle {
                    sprite_path,
                    color,
                    size_scale,
                    ..
                } => Some((*sprite_path, color[3], *size_scale)),
                _ => None,
            })
            .collect();
        // HIT1 has 2 forward + 2 backward = 4 particles × 3 segments = 12.
        assert_eq!(
            particles.len(),
            HIT1.bursts.iter().map(|b| b.count).sum::<usize>() * NUM_SEGMENTS,
            "expected NUM_SEGMENTS=3 sprite draws per particle"
        );
        assert!(particles.iter().all(|(s, _, _)| *s == PARTICLE1_SPRITE));

        // Per-particle trail check: groups of NUM_SEGMENTS should have
        // strictly decreasing alpha and size from segment 0 to 2.
        for chunk in particles.chunks(NUM_SEGMENTS) {
            assert!(chunk[0].1 >= chunk[1].1, "segment 1 alpha ≤ segment 0");
            assert!(chunk[1].1 >= chunk[2].1, "segment 2 alpha ≤ segment 1");
            assert!(chunk[0].2 >= chunk[1].2, "segment 1 size ≤ segment 0");
            assert!(chunk[1].2 >= chunk[2].2, "segment 2 size ≤ segment 1");
        }
    }

    #[test]
    fn hit3_emits_two_rings_after_height_grows_and_eight_particles() {
        // Hit3's two cylinders start with height_size=0 and grow via
        // height_speed/height_accel. After at least one tick both rings
        // should have non-zero height and render. Particles spawn
        // immediately so they always show.
        let mut e = HitEffect::new(Attach::WorldPos([0.0; 3]), HIT3);
        // First tick — height integration applies height_accel then
        // height_speed. With height_speed_init=0.5, accel=0.2 for ring 1,
        // after one frame height_size = 0.5 + 0.2 = 0.7 > 0.
        e.update(&ctx(1.0 / 60.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let ring_count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        let particle_count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(ring_count, 2, "HIT3 launches 2 concentric rings after height>0");
        // inner=outer=1.5 for HIT3 ring 1 (so both ends of the
        // Frustum are 1.5), and inner=1.5/outer=4.0 for ring 2 (so
        // bottom=1.5, top=4.0). At least one Frustum should carry
        // top_size=4.0 (the wide flare on ring 2) while the other
        // stays at top_size=1.5.
        let tops: Vec<f32> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { top_size, .. } => Some(*top_size),
                _ => None,
            })
            .collect();
        assert!(tops.contains(&1.5));
        assert!(tops.contains(&4.0));
        assert_eq!(
            particle_count,
            8 * NUM_SEGMENTS,
            "HIT3 launches 8 forward particles × 3 trail segments"
        );
    }

    #[test]
    fn hit3_height_grows_over_time_while_hit4_grows_slower() {
        // The original game differentiates Hit3 from Hit4 mostly through height speed/
        // heightAccel. After several ticks, Hit3's outer ring has
        // grown taller than Hit4's ring even though they share the
        // same outer_size=4.0.
        let mut h3 = HitEffect::new(Attach::WorldPos([0.0; 3]), HIT3);
        let mut h4 = HitEffect::new(Attach::WorldPos([0.0; 3]), HIT4);
        for _ in 0..5 {
            h3.update(&ctx(1.0 / 60.0));
            h4.update(&ctx(1.0 / 60.0));
        }
        // Look at the outer Hit3 ring (the second one — outer_size=4.0
        // is the wider flare matching Hit4's outer_size).
        let h3_outer_height = h3
            .ring_state
            .iter()
            .zip(h3.params.rings.iter())
            .find(|(_, r)| (r.outer_size - 4.0).abs() < 1e-3)
            .map(|(s, _)| s.height_size)
            .unwrap();
        let h4_height = h4.ring_state[0].height_size;
        // Hit3 ring 2: height_speed=0.25, height_accel=0.2 → after 5 ticks
        //   accumulated height_speed = 0.25 + 5*0.2 = 1.25, height ≈ 5*avg_speed
        // Hit4 ring : height_speed=0.25, height_accel=0.15 → slower growth
        // So Hit3 outer must be taller than Hit4 outer.
        assert!(
            h3_outer_height > h4_height,
            "Hit3 outer height ({h3_outer_height}) must exceed Hit4 height ({h4_height}) after 5 ticks"
        );
    }

    #[test]
    fn debris_particles_have_3d_velocity_and_decay() {
        // After a tick, particle history[0] should differ from its
        // spawn position (proof of 3D velocity integration), and after
        // several ticks the speed-decel should slow the motion.
        let mut e = HitEffect::new_with_angle(
            Attach::WorldPos([0.0; 3]),
            HIT1,
            0.0,
        );
        e.update(&ctx(0.0)); // triggers spawn
        let spawn_positions: Vec<[f32; 3]> =
            e.particles.iter().map(|p| p.history[0]).collect();
        e.update(&ctx(1.0 / 60.0));
        // At least one particle has moved.
        let moved = e
            .particles
            .iter()
            .zip(&spawn_positions)
            .any(|(p, s)| (p.history[0][0] - s[0]).abs() > 0.01
                || (p.history[0][1] - s[1]).abs() > 0.01
                || (p.history[0][2] - s[2]).abs() > 0.01);
        assert!(moved, "particles must move on integration step");
    }

    #[test]
    fn hit1_gravity_particles_arc_upward_then_fall() {
        // The backward gravity burst (the second DebrisBurst in HIT1)
        // starts with negative Y-velocity (upward in native RO) and a
        // positive Y-acceleration. After enough time the gravity_velocity_y
        // should have grown positive, indicating the particle has
        // transitioned from rising to falling.
        let mut e = HitEffect::new(Attach::WorldPos([0.0; 3]), HIT1);
        e.update(&ctx(0.0));
        // Backward gravity particles are the last `count` particles in
        // the spawn order (forward burst first, backward second).
        let backward = HIT1.bursts[1].count;
        let total = e.particles.len();
        let initial_gravity: Vec<f32> = e
            .particles
            .iter()
            .skip(total - backward)
            .map(|p| p.gravity_velocity_y)
            .collect();
        assert!(
            initial_gravity.iter().all(|&g| g < 0.0),
            "gravity particles start with upward (negative) velocity: {initial_gravity:?}"
        );
        // Step ~half a second; gravity accel should now have flipped
        // the sign on most particles.
        for _ in 0..30 {
            e.update(&ctx(1.0 / 60.0));
        }
        let now_falling = e
            .particles
            .iter()
            .filter(|p| p.gravity_velocity_y > 0.0)
            .count();
        assert!(
            now_falling > 0 || e.particles.is_empty(),
            "after 0.5s some backward gravity particles should be falling"
        );
    }

    #[test]
    fn effect_dies_after_total_duration() {
        let mut e = HitEffect::new(Attach::WorldPos([0.0; 3]), HIT1);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        // Total duration covers the longest particle's lifetime
        // (30 frames = 0.5s). Run for 2× that to be safe.
        while t < 2.0 {
            status = e.update(&ctx(1.0 / 60.0));
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
