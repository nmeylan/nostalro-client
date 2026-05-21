//! `EF_INCAGILITY` / `EF_DECAGILITY` / `EF_INCAGIDEX` — agility/dex status-up
//! visual. The three effects share one cross-textured streak emitter; they
//! differ only in particle direction (Incagility/Incagidex rise, Decagility
//! falls) and the per-skill 2D-icon hue (icon is the original game's
//! 2D-texture screen overlay — we don't have a screen-anchored primitive
//! yet, so the icon is currently omitted and only the streak particles
//! render).
//!
//! Per-particle recipe from the original game (the increase-agility effect),
//! `DecAgility()` @ `:9975`, `IncAGIDEX()` @ `:8280`):
//!   * every 2 parent frames, spawn one 3D cross-texture prim
//!   * random Y-rotation longitude; offset = (radius·sin, 0, radius·cos)
//!     where `radius = random(7) + 2`  (= 2..9 wu around the entity)
//!   * the streak is two perpendicular textured quads; we render
//!     one camera-facing `Billboard` per particle, which matches the
//!     silhouette in the gif reference (`0-50/37.gif`).
//!   * Incagility/Incagidex: speed `(random(50)+20)/100` upward
//!     (0.2..0.7 wu / frame in native RO -Y), tilted to lie vertical
//!   * Decagility: accel 0.015 downward, particle starts 20 wu
//!     above the ground, tilted the other way
//!   * width `(random(60)+30)/10` → 3..9 wu; height 0.18
//!     (very thin streak)
//!   * alpha ramps in over 20 frames to max alpha 200/255, then fades out
//!     in the last 20 frames of a 50-frame lifetime
//!
//! Visual reference: short vertical white/violet streaks rising from a
//! disc around the entity.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const STREAK_TEXTURE: &str = "ac_center2.tga";
pub const TEXTURES: &[&str] = &[STREAK_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 60.0;
const SPAWN_PERIOD_FRAMES: u32 = 2;
const PARTICLE_DURATION_FRAMES: f32 = 50.0;
const PARTICLE_FADE_IN_FRAMES: f32 = 20.0;
const PARTICLE_FADEOUT_AT: f32 = PARTICLE_DURATION_FRAMES - 20.0;
const PARTICLE_MAX_ALPHA: f32 = 200.0 / 255.0;
// After the original game's X-rotation of ±90° the cross-texture's
// width size becomes the streak's vertical extent and height size
// becomes its perpendicular thickness. The original game's literal
// `(random(60)+30)/10 = 3..9` is in its own scale; the gif reference
// shows streaks shorter than a character (~5 wu in our coords), so we
// shrink to sub-character size to match the silhouette.
const PARTICLE_LENGTH_MIN: f32 = 1.5;
const PARTICLE_LENGTH_MAX: f32 = 3.0;
const PARTICLE_THICKNESS: f32 = 0.4;
// radius `random(7) + 2` = 2..9 wu; the gif shows streaks
// clustered tight around the entity (within roughly one character
// footprint, ~1.5 wu), so we shrink the disc to match.
const RADIUS_MIN: f32 = 0.6;
const RADIUS_MAX: f32 = 2.0;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

/// Per-variant motion + tint. `INCAGILITY` rises; `DECAGILITY` falls;
/// `INCAGIDEX` rises and is tinted violet/pink.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Initial Y velocity (native RO units / frame). Negative = upward.
    pub initial_speed_per_frame: f32,
    /// Y acceleration (native RO units / frame²). Positive = downward.
    pub accel_per_frame: f32,
    /// Per-particle Y spawn offset (negative = above ground).
    pub spawn_y_offset: f32,
    /// RGB tint multiplied onto the streak texture's alpha mask.
    pub tint: [f32; 3],
}

pub const INCAGILITY: Params = Params {
    // speed = (random(50)+20)/100 upward = -0.45 avg per frame.
    initial_speed_per_frame: -0.45,
    accel_per_frame: 0.0,
    spawn_y_offset: 0.0,
    tint: [1.0, 1.0, 1.0],
};

pub const DECAGILITY: Params = Params {
    // accel = 0.015 downward; no initial speed. Particle starts
    // 20 wu above ground (its vertical offset starts 20 above) and falls toward it.
    initial_speed_per_frame: 0.0,
    accel_per_frame: 0.015,
    spawn_y_offset: -20.0,
    tint: [1.0, 1.0, 1.0],
};

pub const INCAGIDEX: Params = Params {
    initial_speed_per_frame: -0.45,
    accel_per_frame: 0.0,
    spawn_y_offset: 0.0,
    // gif reference (`imgs/0-50/43.gif`) shows mauve/violet streaks
    // instead of pure white.
    tint: [0.85, 0.7, 1.0],
};

#[derive(Clone, Copy, Debug)]
struct Particle {
    anchor: [f32; 3],
    offset: [f32; 3],
    y_velocity_per_frame: f32,
    length: f32,
    age_frames: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age_frames < PARTICLE_DURATION_FRAMES
    }

    fn step(&mut self, dt_frames: f32, accel_per_frame: f32) {
        self.y_velocity_per_frame += accel_per_frame * dt_frames;
        self.offset[1] += self.y_velocity_per_frame * dt_frames;
        self.age_frames += dt_frames;
    }

    fn alpha(&self) -> f32 {
        let fade_in = (self.age_frames / PARTICLE_FADE_IN_FRAMES).clamp(0.0, 1.0);
        let fade_out = if self.age_frames < PARTICLE_FADEOUT_AT {
            1.0
        } else {
            let span = (PARTICLE_DURATION_FRAMES - PARTICLE_FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - PARTICLE_FADEOUT_AT) / span).clamp(0.0, 1.0)
        };
        PARTICLE_MAX_ALPHA * fade_in * fade_out
    }

    fn position(&self) -> [f32; 3] {
        [
            self.anchor[0] + self.offset[0],
            self.anchor[1] + self.offset[1],
            self.anchor[2] + self.offset[2],
        ]
    }
}

pub struct StatusUpEffect {
    world_pos: [f32; 3],
    params: Params,
    particles: Vec<Particle>,
    age_frames: f32,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl StatusUpEffect {
    pub fn new(world_pos: [f32; 3], params: Params) -> Self {
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13)
            ^ (params.initial_speed_per_frame.to_bits()).rotate_left(7);
        Self {
            world_pos,
            params,
            particles: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
            rng_state,
        }
    }

    fn lcg(&mut self) -> u32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.rng_state
    }

    fn lcg_float(&mut self) -> f32 {
        (self.lcg() >> 8) as f32 / ((1u32 << 24) as f32)
    }

    fn spawn_particle(&mut self) {
        let longitude_deg = self.lcg_float() * 360.0;
        let radius = RADIUS_MIN + self.lcg_float() * (RADIUS_MAX - RADIUS_MIN);
        let length = PARTICLE_LENGTH_MIN
            + self.lcg_float() * (PARTICLE_LENGTH_MAX - PARTICLE_LENGTH_MIN);
        let (sn, cs) = longitude_deg.to_radians().sin_cos();
        // delta position = `(0, 0, radius)` rotated by longitude about Y
        // expands to (radius·sin, 0, radius·cos).
        self.particles.push(Particle {
            anchor: self.world_pos,
            offset: [radius * sn, self.params.spawn_y_offset, radius * cs],
            y_velocity_per_frame: self.params.initial_speed_per_frame,
            length,
            age_frames: 0.0,
        });
    }
}

impl Effect for StatusUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        if (self.age_frames as f32) <= PARENT_DURATION_FRAMES {
            let next_frame = self.last_spawn_frame + 1;
            for f in next_frame..=current_frame {
                if f >= 0
                    && (f as f32) <= PARENT_DURATION_FRAMES
                    && (f as u32) % SPAWN_PERIOD_FRAMES == 0
                {
                    self.spawn_particle();
                }
            }
            self.last_spawn_frame = current_frame;
        }

        for p in &mut self.particles {
            p.step(dt_frames, self.params.accel_per_frame);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES
            && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            let alpha = p.alpha();
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: p.position(),
                size: [PARTICLE_THICKNESS, p.length],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: 0.0,
                texture: STREAK_TEXTURE,
                color: [
                    self.params.tint[0],
                    self.params.tint[1],
                    self.params.tint[2],
                    alpha,
                ],
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
    fn incagility_emits_billboards_rising_around_entity() {
        // Sociable test: 2-frame spawn cadence + radius-2..9 ring +
        // upward Y motion. After a handful of frames there's more than
        // one particle, all on the disc around the entity, and the Y
        // velocity is negative (upward in native RO).
        let mut e = StatusUpEffect::new([10.0, 0.0, 20.0], INCAGILITY);
        for _ in 0..6 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        // Frames 0,2,4 spawned → at least 3 particles.
        assert!(list.primitives.len() >= 3, "spawn cadence every 2 frames");
        for prim in &list.primitives {
            let EffectPrimitiveDraw::Billboard { pos, color, .. } = prim else {
                panic!("expected Billboard, got {prim:?}");
            };
            let dx = pos[0] - 10.0;
            let dz = pos[2] - 20.0;
            let r = (dx * dx + dz * dz).sqrt();
            assert!(
                (RADIUS_MIN - 0.5..=RADIUS_MAX + 0.5).contains(&r),
                "particle on radius 2..9 disc: r={r}",
            );
            assert!(color[3] > 0.0, "non-zero alpha during fade-in");
        }
    }

    #[test]
    fn decagility_particles_fall_and_incagidex_is_tinted() {
        // Sociable: after a few frames, Decagility's particles are
        // moving DOWN (+Y) — the accel pulls them past their spawn
        // offset. Incagidex's color tint is non-white.
        let mut dec = StatusUpEffect::new([0.0, 0.0, 0.0], DECAGILITY);
        for _ in 0..30 {
            dec.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let mut list = EffectDrawList::new();
        dec.collect_draws(&mut list, &render_ctx());
        // First-spawned particle (age ~30 frames) has fallen below
        // spawn_y_offset = -20.
        let lowest_y = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, .. } => Some(pos[1]),
                _ => None,
            })
            .fold(f32::MIN, f32::max);
        assert!(lowest_y > DECAGILITY.spawn_y_offset, "particles fall over time");

        // Incagidex is mauve/violet.
        assert_ne!(INCAGIDEX.tint, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn effect_dies_after_parent_plus_particle_lifetime() {
        let mut e = StatusUpEffect::new([0.0; 3], INCAGILITY);
        let total = PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
