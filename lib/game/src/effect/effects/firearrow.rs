//! `EF_FIREARROW` — Archer Fire Arrow (id 31).
//!
//! Original game's FireArrow emits a particle trail
//! particles (`particle4.spr`) every 4 frames — small shrinking fire
//! sprites scattering outward from the impact point. At frame 12 a
//! 3D-cross-texture cross (two perpendicular billboards cycling
//! `불화살1-6.tga` flame frames) flies outward along the scatter
//! direction.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::seed_from_world;

const PARTICLE4_SPRITE: &str = "data/sprite/이팩트/particle4";
pub const SPRITES: &[&str] = &[PARTICLE4_SPRITE];

const FLAME_TEXTURES: &[&str] = &[
    "불화살1.tga",
    "불화살2.tga",
    "불화살3.tga",
    "불화살4.tga",
    "불화살5.tga",
    "불화살6.tga",
    "불화살7.tga",
    "불화살8.tga",
];
pub const TEXTURES: &[&str] = FLAME_TEXTURES;

const FPS: f32 = 60.0;

// Cross (3D cross texture)
const CROSS_SPAWN_FRAME: f32 = 12.0;
const CROSS_DURATION: f32 = 70.0;
const CROSS_FADE_OUT_START: f32 = 50.0;
const CROSS_TEX_COUNT: usize = 6;
const CROSS_SPEED: f32 = 0.3;
const CROSS_WIDTH: f32 = 4.0;
const CROSS_HEIGHT: f32 = 1.0;

// Trail (3D particle)
const TRAIL_FIRST_FRAME: f32 = 4.0;
const TRAIL_INTERVAL: f32 = 4.0;
const TRAIL_SPAWN_CUTOFF: f32 = 80.0;
const TRAIL_MIN_DURATION: f32 = 6.0;
const TRAIL_MAX_DURATION: f32 = 30.0;
const TRAIL_MIN_SPEED: f32 = 0.6;
const TRAIL_MAX_SPEED: f32 = 1.5;
const TRAIL_MIN_SIZE: f32 = 0.2;
const TRAIL_MAX_SIZE: f32 = 0.5;

const TOTAL_FRAMES: f32 = TRAIL_SPAWN_CUTOFF + TRAIL_MAX_DURATION;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

const Y_OFFSET: f32 = -1.5;

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

struct TrailParticle {
    spawn_frame: f32,
    duration: f32,
    dir: [f32; 3],
    speed: f32,
    accel: f32,
    init_size: f32,
    size_speed: f32,
}

pub struct FireArrowEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    base_dir: [f32; 3],
    trail: Vec<TrailParticle>,
}

impl FireArrowEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut rng = seed_from_world(world_pos);

        let base_lon = lcg_float(&mut rng) * std::f32::consts::TAU;
        let base_lat = 50.0_f32.to_radians();
        let cl = base_lat.cos();
        let base_dir = [cl * base_lon.sin(), -base_lat.sin(), cl * base_lon.cos()];

        let mut trail = Vec::new();
        let mut frame = TRAIL_FIRST_FRAME;
        while frame <= TRAIL_SPAWN_CUTOFF {
            let lon = base_lon + (-20.0 + lcg_float(&mut rng) * 40.0).to_radians();
            let lat = base_lat + (-10.0 + lcg_float(&mut rng) * 20.0).to_radians();
            let c = lat.cos();
            let dir = [c * lon.sin(), -lat.sin(), c * lon.cos()];

            let duration =
                TRAIL_MIN_DURATION + lcg_float(&mut rng) * (TRAIL_MAX_DURATION - TRAIL_MIN_DURATION);
            let speed =
                TRAIL_MIN_SPEED + lcg_float(&mut rng) * (TRAIL_MAX_SPEED - TRAIL_MIN_SPEED);
            let accel = -(speed / duration) / 1.5;
            let init_size =
                TRAIL_MIN_SIZE + lcg_float(&mut rng) * (TRAIL_MAX_SIZE - TRAIL_MIN_SIZE);
            let size_speed = -(init_size / duration);

            trail.push(TrailParticle {
                spawn_frame: frame,
                duration,
                dir,
                speed,
                accel,
                init_size,
                size_speed,
            });

            frame += TRAIL_INTERVAL;
        }

        Self {
            world_pos,
            age_frames: 0.0,
            base_dir,
            trail,
        }
    }
}

impl Effect for FireArrowEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let pos_base = [
            self.world_pos[0],
            self.world_pos[1] + Y_OFFSET,
            self.world_pos[2],
        ];

        for p in &self.trail {
            let local_age = self.age_frames - p.spawn_frame;
            if local_age < 0.0 || local_age >= p.duration {
                continue;
            }

            let dist = p.speed * local_age + 0.5 * p.accel * local_age * local_age;
            let pos = [
                pos_base[0] + p.dir[0] * dist,
                pos_base[1] + p.dir[1] * dist,
                pos_base[2] + p.dir[2] * dist,
            ];

            let size = (p.init_size + p.size_speed * local_age).max(0.0);
            let alpha = (1.0 - local_age / p.duration).clamp(0.0, 1.0);
            let motion = (local_age * 2.0) as usize;

            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE4_SPRITE,
                position: pos,
                action_index: 0,
                motion_index: motion,
                size_scale: size,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                aim_target: None,
                no_depth: false,
            });
        }

        let cross_age = self.age_frames - CROSS_SPAWN_FRAME;
        if cross_age >= 0.0 && cross_age < CROSS_DURATION {
            let dist = CROSS_SPEED * cross_age;
            let cross_pos = [
                pos_base[0] + self.base_dir[0] * dist,
                pos_base[1] + self.base_dir[1] * dist,
                pos_base[2] + self.base_dir[2] * dist,
            ];

            let tex_idx = (cross_age as usize) % CROSS_TEX_COUNT;

            let alpha = if cross_age < CROSS_FADE_OUT_START {
                1.0
            } else {
                1.0 - (cross_age - CROSS_FADE_OUT_START) / (CROSS_DURATION - CROSS_FADE_OUT_START)
            };

            for rotation in [0.0_f32, std::f32::consts::FRAC_PI_2] {
                out.push(EffectPrimitiveDraw::Billboard {
                    pos: cross_pos,
                    size: [CROSS_WIDTH, CROSS_HEIGHT],
                    uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                    rotation,
                    texture: FLAME_TEXTURES[tex_idx],
                    color: [1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)],
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

    fn advance(e: &mut FireArrowEffect, frames: f32) {
        e.update(&ctx(frames / FPS));
    }

    fn draws(e: &FireArrowEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_trail_sprite_particles_and_cross_billboards() {
        let mut e = FireArrowEffect::new([0.0; 3]);
        advance(&mut e, 15.0);

        let d = draws(&e);
        let sprites = d
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == PARTICLE4_SPRITE))
            .count();
        let billboards = d
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. }))
            .count();

        assert!(sprites > 0, "trail particles present after frame 4");
        assert_eq!(billboards, 2, "cross = two perpendicular billboards after frame 12");
    }

    #[test]
    fn trail_particles_scatter_outward_from_impact() {
        let pos = [10.0, 0.0, 20.0];
        let mut e = FireArrowEffect::new(pos);
        advance(&mut e, 20.0);

        let positions: Vec<[f32; 3]> = draws(&e)
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => Some(*position),
                _ => None,
            })
            .collect();

        assert!(positions.len() >= 2, "multiple trail particles alive at frame 20");
        let any_moved = positions.iter().any(|p| {
            let dx = p[0] - pos[0];
            let dz = p[2] - pos[2];
            (dx * dx + dz * dz).sqrt() > 0.5
        });
        assert!(any_moved, "particles scatter outward from impact point");
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = FireArrowEffect::new([0.0; 3]);
        let status = e.update(&ctx(TOTAL_FRAMES / FPS + 0.01));
        assert_eq!(status, EffectStatus::Dead);
    }
}
