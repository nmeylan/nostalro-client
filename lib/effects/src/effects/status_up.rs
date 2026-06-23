//! `EF_INCAGILITY` / `EF_DECAGILITY` / `EF_INCAGIDEX` — agility/dex status-up
//! visual. The three effects share one cross-textured streak emitter plus a
//! center label texture; they differ in particle direction (Incagility /
//! Incagidex rise, Decagility falls), in the streak tint, and in which label
//! is drawn (`agi_up.bmp`, `slow.bmp`, `dex_agi_up.bmp`).
//!
//! Per-particle recipe (cross-textured streaks rising or falling around the
//! entity, fading in and out over their lifetime):
//!   * every 2 parent frames, spawn one cross-textured streak
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
//! Center label is a screen overlay. The
//! original draws it in 2D screen space; we approximate with a camera-facing
//! world billboard at the entity, which keeps it readable on top of the
//! streaks. It rises (or falls, for Decagility) and fades in/out across the
//! parent's lifetime.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const STREAK_TEXTURE: &str = "ac_center2.tga";
pub const AGI_UP_TEXTURE: &str = "agi_up.bmp";
pub const SLOW_TEXTURE: &str = "slow.bmp";
pub const DEX_AGI_UP_TEXTURE: &str = "dex_agi_up.bmp";
pub const TEXTURES: &[&str] = &[STREAK_TEXTURE, AGI_UP_TEXTURE, SLOW_TEXTURE, DEX_AGI_UP_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 60.0;
const SPAWN_PERIOD_FRAMES: u32 = 2;
const PARTICLE_DURATION_FRAMES: f32 = 50.0;
const PARTICLE_FADE_IN_FRAMES: f32 = 20.0;
const PARTICLE_FADEOUT_AT: f32 = PARTICLE_DURATION_FRAMES - 20.0;
const PARTICLE_MAX_ALPHA: f32 = 200.0 / 255.0;
// After the ±90° tilt the cross-texture's
// width becomes the streak's vertical extent and height
// becomes its perpendicular thickness. The literal range
// `(random(60)+30)/10 = 3..9` reads directly in our world units — the
// streaks are meant to span roughly one character height, not a
// fraction of it.
const PARTICLE_LENGTH_MIN: f32 = 3.0;
const PARTICLE_LENGTH_MAX: f32 = 7.0;
const PARTICLE_THICKNESS: f32 = 0.6;
// Radius `random(7) + 2` = 2..9 wu; gif shows the streaks
// clustered tight enough to read as one column above the entity, but
// still wide enough to show several streaks side-by-side around it.
const RADIUS_MIN: f32 = 2.0;
const RADIUS_MAX: f32 = 9.0;

// Center label sizing. The original uses half-extents
// width=40 / height=20 px in screen space; at our default camera
// distance ~5 px/wu, so 10 wu reads close to that screen
// footprint and stays legible without going over the character.
const LABEL_WIDTH: f32 = 10.0;
const LABEL_HEIGHT_INC: f32 = 5.0;
const LABEL_HEIGHT_DEC: f32 = 2.5;
const LABEL_MAX_ALPHA: f32 = 200.0 / 255.0;
const LABEL_FADE_FRAMES: f32 = 15.0;
const LABEL_FADEOUT_AT: f32 = PARENT_DURATION_FRAMES - LABEL_FADE_FRAMES;
// Vertical drift in world units / frame. The original's 1.5 / 1.0 px/frame
// values map to ~0.15 / 0.10 wu/frame at the same 10 px/wu scale.
const LABEL_RISE_SPEED: f32 = 0.15;
const LABEL_FALL_SPEED: f32 = 0.10;
// Decagility starts the label above the entity (the original lifts it 80
// px → ~8 wu in world space, native RO -Y up).
const LABEL_DEC_SPAWN_Y: f32 = -8.0;
// Center label sits roughly at chest height above the entity origin
// (origin is at feet) so it reads as a tag on the character, not
// floating in the dirt.
const LABEL_INC_SPAWN_Y: f32 = -4.0;

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
    /// Center text/icon texture (`agi_up.bmp`, `slow.bmp` or
    /// `dex_agi_up.bmp`) drawn as a billboard at the entity.
    pub label_texture: &'static str,
    /// Label half-height in world units (width is shared across variants).
    pub label_height: f32,
    /// Initial Y velocity for the label (native RO: negative = upward).
    pub label_speed_per_frame: f32,
    /// Y offset where the label spawns relative to the entity origin.
    pub label_spawn_y: f32,
}

pub const INCAGILITY: Params = Params {
    // Speed `(random(50)+20)/100` upward = -0.45 avg per frame.
    initial_speed_per_frame: -0.45,
    accel_per_frame: 0.0,
    spawn_y_offset: 0.0,
    tint: [1.0, 1.0, 1.0],
    label_texture: AGI_UP_TEXTURE,
    label_height: LABEL_HEIGHT_INC,
    label_speed_per_frame: -LABEL_RISE_SPEED,
    label_spawn_y: LABEL_INC_SPAWN_Y,
};

pub const DECAGILITY: Params = Params {
    // Accel 0.015 downward; no initial speed. Particle starts
    // 20 wu above ground and falls toward it.
    initial_speed_per_frame: 0.0,
    accel_per_frame: 0.015,
    spawn_y_offset: -20.0,
    tint: [1.0, 1.0, 1.0],
    label_texture: SLOW_TEXTURE,
    label_height: LABEL_HEIGHT_DEC,
    label_speed_per_frame: LABEL_FALL_SPEED,
    label_spawn_y: LABEL_DEC_SPAWN_Y,
};

pub const INCAGIDEX: Params = Params {
    initial_speed_per_frame: -0.45,
    accel_per_frame: 0.0,
    spawn_y_offset: 0.0,
    // gif reference (`imgs/0-50/43.gif`) shows mauve/violet streaks
    // instead of pure white.
    tint: [0.85, 0.7, 1.0],
    label_texture: DEX_AGI_UP_TEXTURE,
    label_height: LABEL_HEIGHT_INC,
    label_speed_per_frame: -LABEL_RISE_SPEED,
    label_spawn_y: LABEL_INC_SPAWN_Y,
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
        // A radius vector rotated about Y by `longitude`
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

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
        for p in &mut self.particles {
            p.anchor = pos;
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
                // Streaks use straight alpha blend
                // (source over destination), not additive — additive
                // vanishes against a bright lightmap.
                blend: BlendKind::Alpha,
            });
        }

        if let Some((label_pos, label_alpha)) = self.label_state() {
            out.push(EffectPrimitiveDraw::Billboard {
                pos: label_pos,
                size: [LABEL_WIDTH, self.params.label_height],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: self.params.label_texture,
                color: [1.0, 1.0, 1.0, label_alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

impl StatusUpEffect {
    /// `(world_position, alpha)` of the center label when it should be
    /// drawn, or `None` past the parent lifetime.
    fn label_state(&self) -> Option<([f32; 3], f32)> {
        if self.age_frames > PARENT_DURATION_FRAMES {
            return None;
        }
        let fade_in = (self.age_frames / LABEL_FADE_FRAMES).clamp(0.0, 1.0);
        let fade_out = if self.age_frames < LABEL_FADEOUT_AT {
            1.0
        } else {
            let span = (PARENT_DURATION_FRAMES - LABEL_FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - LABEL_FADEOUT_AT) / span).clamp(0.0, 1.0)
        };
        let alpha = LABEL_MAX_ALPHA * fade_in * fade_out;
        if alpha <= 0.0 {
            return None;
        }
        let y = self.world_pos[1]
            + self.params.label_spawn_y
            + self.params.label_speed_per_frame * self.age_frames;
        Some(([self.world_pos[0], y, self.world_pos[2]], alpha))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
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
    fn incagility_emits_streaks_on_ring_plus_center_label() {
        // Sociable test: 2-frame spawn cadence + radius-2..9 ring +
        // upward Y motion, and a single center label sitting at the
        // entity's XZ. The label is the `agi_up.bmp` overlay.
        let mut e = StatusUpEffect::new([10.0, 0.0, 20.0], INCAGILITY);
        for _ in 0..6 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let (labels, streaks): (Vec<_>, Vec<_>) =
            list.primitives.iter().partition(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, .. } => *texture == AGI_UP_TEXTURE,
                _ => false,
            });
        assert_eq!(labels.len(), 1, "exactly one center label per frame");
        let EffectPrimitiveDraw::Billboard { pos, color, .. } = labels[0] else {
            unreachable!();
        };
        assert!((pos[0] - 10.0).abs() < 1e-3 && (pos[2] - 20.0).abs() < 1e-3);
        assert!(color[3] > 0.0);

        // Frames 0,2,4 spawned → at least 3 streak particles.
        assert!(streaks.len() >= 3, "spawn cadence every 2 frames");
        assert!(
            streaks.iter().all(|p| matches!(p, EffectPrimitiveDraw::Billboard { blend: BlendKind::Alpha, .. })),
            "streaks are alpha-blended (original default RF_ALPHA)"
        );
        for prim in &streaks {
            let EffectPrimitiveDraw::Billboard { pos, color, .. } = prim else {
                panic!("expected Billboard, got {prim:?}");
            };
            let dx = pos[0] - 10.0;
            let dz = pos[2] - 20.0;
            let r = (dx * dx + dz * dz).sqrt();
            assert!(
                (RADIUS_MIN - 0.5..=RADIUS_MAX + 0.5).contains(&r),
                "particle on radius disc: r={r}",
            );
            assert!(color[3] > 0.0, "non-zero alpha during fade-in");
        }
    }

    #[test]
    fn each_variant_uses_its_own_label_texture() {
        for (params, expected) in [
            (INCAGILITY, AGI_UP_TEXTURE),
            (DECAGILITY, SLOW_TEXTURE),
            (INCAGIDEX, DEX_AGI_UP_TEXTURE),
        ] {
            let mut e = StatusUpEffect::new([0.0, 0.0, 0.0], params);
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            let mut list = EffectDrawList::new();
            e.collect_draws(&mut list, &render_ctx());
            let found = list.primitives.iter().any(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, .. } => *texture == expected,
                _ => false,
            });
            assert!(found, "label texture {expected} not emitted");
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
                EffectPrimitiveDraw::Billboard { pos, texture, .. }
                    if *texture == STREAK_TEXTURE =>
                {
                    Some(pos[1])
                }
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
